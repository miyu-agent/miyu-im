//! 后台页调的方法（设计 03 第一节第 6 条、第二节第 5 条，施工 S-1 下）：核心发 `method.call {method, params, from?}`，网页的
//! 「软件后台」和 im 插件卡片的「打开面板」打开的是同一份页面（`sponsor/page/`），都经它调。改、删只在这里（不给 AI）。
//!
//! - `totals` → `{base, month, year, all, pending}`：本月、今年、全部换成默认币种的合计（照本机时区），待补汇率的笔数。
//! - `list {query?, platform?, from?, to?}` → `{base, entries}`：明细，新的在前；`query` 搜号或名字（不分大小写）。
//! - `ranking {platform?, from?, to?}` → `{base, rows: [{user, name, platform, sum, count}]}`：全部（施工 I-60）。
//! - `add {platform, user?, name?, amount, currency?, note?}` → `{entry}`：记一笔（照汇率换算、冻结），号、名字至少一样。
//! - `update {id, user?, name?, amount?, currency?, note?, rate?}` → `{entry}`：改一笔；写了 `rate` 的照它重算（补汇率），改了
//!   金额、币种的照原来的汇率重算（币种变了的汇率作废、照现在的再查一次）。
//! - `delete {id}` → `{deleted}`。
//! - `rate {currency}` → `{base, rate}`：照现在的默认币种查一次（补汇率时页面先问），查不到的 `rate` 是 `null`。
//!
//! 参数写错的回 JSON-RPC 的 -32602，`message` 说哪个；没有这一笔的回 `not_found`。

use std::sync::{MutexGuard, PoisonError};

use jiff::tz::TimeZone;
use serde_json::{Value, json};

use crate::State;
use crate::money::{self, Period, Sponsor};
use crate::rates::frozen;
use crate::settings::currency;
use crate::store::{Book, Entry, New};
use crate::wire::Answer;

/// 登记的方法。
pub const NAMES: [&str; 7] = [
    "totals", "list", "ranking", "add", "update", "delete", "rate",
];

/// 名字、号最多几个字。
const NAME: usize = 64;

/// 备注最多几个字。
const NOTE: usize = 200;

/// 金额最多多大。
const LARGEST: f64 = 1e9;

/// `package.methods` 的参数。
pub fn register() -> Value {
    let methods: Vec<Value> = NAMES.iter().map(|name| json!({"name": name})).collect();
    json!({"methods": methods})
}

/// 参数写错了。
fn bad(why: &str) -> Value {
    json!({"code": -32602, "message": why})
}

/// 账本的锁。
fn locked(state: &State) -> MutexGuard<'_, Book> {
    state.book.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 办一次 `method.call`：`now` 是此刻（毫秒），`zone` 是本机的时区。
pub fn call(state: &State, request: &Value, (now, zone): (i64, &TimeZone)) -> Answer {
    let params = &request["params"];
    let base = state
        .settings
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .currency
        .clone();
    match request["method"].as_str().unwrap_or_default() {
        "totals" => Ok(totals(locked(state).entries(), &base, (now, zone))),
        "list" => {
            let wanted = Filter::read(params)?;
            let book = locked(state);
            let entries: Vec<&Entry> = book
                .entries()
                .iter()
                .rev()
                .filter(|entry| wanted.keeps(entry))
                .collect();
            Ok(json!({"base": base, "entries": entries}))
        }
        "ranking" => {
            let wanted = Filter::read(params)?;
            let within: Vec<Entry> = locked(state)
                .entries()
                .iter()
                .filter(|entry| wanted.keeps(entry))
                .cloned()
                .collect();
            let rows: Vec<Value> = money::ranking(&within, &base)
                .into_iter()
                .map(|row| {
                    let (user, name) = match row.sponsor {
                        Sponsor::Id(user) => (Some(user), row.name),
                        Sponsor::Name(name) => (None, Some(name)),
                    };
                    json!({"user": user, "name": name, "platform": row.platform, "sum": row.sum, "count": row.count})
                })
                .collect();
            Ok(json!({"base": base, "rows": rows}))
        }
        "add" => add(state, params, (&base, now)),
        "update" => update(state, params, &base),
        "delete" => {
            let id = id_of(params)?;
            let deleted = locked(state)
                .delete(id)
                .map_err(|why| json!({"code": -32000, "message": why}))?;
            Ok(json!({"deleted": deleted}))
        }
        "rate" => {
            let currency = params["currency"]
                .as_str()
                .and_then(currency)
                .ok_or_else(|| bad("currency must be a three-letter code"))?;
            let (rate, _) = frozen(state.rates.as_ref(), 1.0, &currency, &base);
            Ok(json!({"base": base, "rate": rate}))
        }
        _ => Err(crate::wire::unknown_method()),
    }
}

/// 头上的合计（纯逻辑）：本月、今年、全部，待补汇率的笔数。
fn totals(entries: &[Entry], base: &str, (now, zone): (i64, &TimeZone)) -> Value {
    let since = |period: Period| period.since(now, zone);
    let sum = |from: Option<i64>| {
        money::total(
            entries
                .iter()
                .filter(|entry| from.is_none_or(|from| entry.at >= from)),
            base,
        )
        .sum
    };
    let all = money::total(entries, base);
    json!({
        "base": base,
        "month": money::cents(sum(since(Period::Month))),
        "year": money::cents(sum(since(Period::Year))),
        "all": money::cents(all.sum),
        "pending": all.pending,
    })
}

/// 明细、排行榜的筛法。
#[derive(Debug, Clone, Default, PartialEq)]
struct Filter {
    query: Option<String>,
    platform: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
}

impl Filter {
    /// 读参数（纯逻辑）：写错的交回 -32602。
    fn read(params: &Value) -> Result<Filter, Value> {
        let text = |key: &str| -> Result<Option<String>, Value> {
            match &params[key] {
                Value::Null => Ok(None),
                Value::String(text) => {
                    Ok(Some(text.trim().to_lowercase()).filter(|text| !text.is_empty()))
                }
                _ => Err(bad(&format!("{key} must be text"))),
            }
        };
        let millis = |key: &str| -> Result<Option<i64>, Value> {
            match &params[key] {
                Value::Null => Ok(None),
                value => value
                    .as_i64()
                    .map(Some)
                    .ok_or_else(|| bad(&format!("{key} must be milliseconds"))),
            }
        };
        Ok(Filter {
            query: text("query")?,
            platform: text("platform")?,
            from: millis("from")?,
            to: millis("to")?,
        })
    }

    /// 这一笔留不留：号、名字里有 `query`，平台对得上，时刻在 `from`（含）、`to`（不含）中间。
    fn keeps(&self, entry: &Entry) -> bool {
        let matches = |query: &str| {
            entry
                .user
                .as_deref()
                .is_some_and(|user| user.to_lowercase().contains(query))
                || entry
                    .name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(query))
        };
        self.query.as_deref().is_none_or(matches)
            && self
                .platform
                .as_deref()
                .is_none_or(|platform| entry.platform == platform)
            && self.from.is_none_or(|from| entry.at >= from)
            && self.to.is_none_or(|to| entry.at < to)
    }
}

/// 一段字：去掉控制字符，截到 `most` 个字；空的是空的。不是字的交回 -32602。
fn text(params: &Value, key: &str, most: usize) -> Result<Option<String>, Value> {
    match &params[key] {
        Value::Null => Ok(None),
        Value::String(text) => {
            let cleaned: String = text
                .chars()
                .filter(|char| !char.is_control())
                .take(most)
                .collect();
            Ok(Some(cleaned.trim().to_string()).filter(|text| !text.is_empty()))
        }
        _ => Err(bad(&format!("{key} must be text"))),
    }
}

/// 金额：正数，最大 [`LARGEST`]。
fn amount(value: &Value) -> Result<f64, Value> {
    value
        .as_f64()
        .filter(|amount| amount.is_finite() && *amount > 0.0 && *amount <= LARGEST)
        .ok_or_else(|| bad("amount must be a number above 0"))
}

/// 编号。
fn id_of(params: &Value) -> Result<u64, Value> {
    params["id"]
        .as_u64()
        .ok_or_else(|| bad("id must be a number"))
}

/// 号写成平台身份：`40001` 写成 `qq:40001`，已经带平台的照原样。
fn identity(platform: &str, user: &str) -> String {
    if user.contains(':') {
        user.to_string()
    } else {
        format!("{platform}:{user}")
    }
}

/// 记一笔（模块的说明）。
fn add(state: &State, params: &Value, (base, now): (&str, i64)) -> Answer {
    let platform = text(params, "platform", NAME)?.ok_or_else(|| bad("platform is needed"))?;
    let user = text(params, "user", NAME)?.map(|user| identity(&platform, &user));
    let name = text(params, "name", NAME)?;
    if user.is_none() && name.is_none() {
        return Err(bad("user or name is needed"));
    }
    let amount = amount(&params["amount"])?;
    let currency = match &params["currency"] {
        Value::Null => base.to_string(),
        value => value
            .as_str()
            .and_then(currency)
            .ok_or_else(|| bad("currency must be a three-letter code"))?,
    };
    let (rate, converted) = frozen(state.rates.as_ref(), amount, &currency, base);
    let new = New {
        platform,
        user,
        name,
        amount,
        currency,
        base: base.to_string(),
        converted,
        rate,
        at: now,
        note: text(params, "note", NOTE)?,
        by: None,
        venue: None,
    };
    let entry = locked(state)
        .add(new)
        .map_err(|why| json!({"code": -32000, "message": why}))?;
    Ok(json!({"entry": entry}))
}

/// 改一笔（模块的说明）。
fn update(state: &State, params: &Value, base: &str) -> Answer {
    let id = id_of(params)?;
    let user = text(params, "user", NAME)?;
    let name = text(params, "name", NAME)?;
    let note = text(params, "note", NOTE)?;
    let amount = match &params["amount"] {
        Value::Null => None,
        value => Some(amount(value)?),
    };
    let currency = match &params["currency"] {
        Value::Null => None,
        value => Some(
            value
                .as_str()
                .and_then(currency)
                .ok_or_else(|| bad("currency must be a three-letter code"))?,
        ),
    };
    let rate = match &params["rate"] {
        Value::Null => None,
        value => Some(
            value
                .as_f64()
                .filter(|rate| rate.is_finite() && *rate > 0.0)
                .ok_or_else(|| bad("rate must be a number above 0"))?,
        ),
    };
    // 币种变了、没给汇率的：照现在的默认币种再查一次（不拿着账本的锁查）。
    let fresh = match (&currency, rate) {
        (Some(currency), None) => Some(frozen(state.rates.as_ref(), 1.0, currency, base).0),
        _ => None,
    };
    let changed = locked(state).update(id, |entry| {
        if let Some(user) = user {
            entry.user = Some(identity(&entry.platform, &user));
        }
        if name.is_some() {
            entry.name = name;
        }
        if note.is_some() {
            entry.note = note;
        }
        if let Some(amount) = amount {
            entry.amount = amount;
        }
        if let Some(currency) = currency {
            entry.currency = currency;
            entry.rate = fresh.flatten();
            entry.base = base.to_string();
        }
        if let Some(rate) = rate {
            entry.rate = Some(rate);
            entry.base = base.to_string();
        }
        entry.converted = entry.rate.map(|rate| money::cents(entry.amount * rate));
    });
    match changed.map_err(|why| json!({"code": -32000, "message": why}))? {
        Some(entry) => Ok(json!({"entry": entry})),
        None => Err(json!({"code": -32000, "message": "not_found"})),
    }
}

#[cfg(test)]
mod tests;
