//! 工具 `sponsor`（设计 03 第一节第 2、3 条，2026-10-24 补定）：`{action, user?, name?, amount?, currency?, note?, period?}`。
//!
//! - `add` 记一笔：只有管理员叫的那一轮能记（`tool.call` 的 `owner` 是真的，或者 `by.role` 是 `manager`），别的答
//!   `admin-only`（程序判断，不靠工具说明：这是钱的账）。谁：写了 `user`（号）的照号；没写的照叫她的那一条引用的人、唯一 @
//!   的那一个（im 的 `chat.venue` 交的 `origin`）；都没有、写了 `name` 的记成只有昵称的赞助人。币种不写的照默认；别的币种照记账那
//!   一刻的汇率换成默认币种、冻结进这一笔，查不到的记原币种、标未换算。
//! - `person` 查一个人：谁的认法同上；累计（换成默认币种的）和最近 [`SHOWN`] 笔，新的在前。谁都能查。
//! - `ranking` 排行榜：`period` 是 `month`、`year`、`all`（出厂 `all`），最多 [`SHOWN`] 个。谁都能查。
//!
//! 改、删只在后台页（S-1 下），不给 AI。参数写错的说哪个（`not-done`）。

use std::sync::{Mutex, PoisonError};

use jiff::tz::TimeZone;
use serde_json::{Value, json};

use crate::money::{self, Period, SHOWN, Sponsor};
use crate::rates::Rates;
use crate::settings::{Settings, currency};
use crate::store::{Book, Entry, New};
use crate::texts::say;

/// 名字最多几个字。
const NAME: usize = 64;

/// 备注最多几个字。
const NOTE: usize = 200;

/// 金额最多多大。
const MOST: f64 = 1e9;

/// 写错了的那一句。
type Bad = &'static str;

/// 做什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Add,
    Person,
    Ranking,
}

/// 读好的参数。
#[derive(Debug, Clone, PartialEq)]
struct Asked {
    action: Action,
    user: Option<i64>,
    name: Option<String>,
    amount: Option<f64>,
    currency: Option<String>,
    note: Option<String>,
    period: Period,
}

/// 读参数 `args`（纯逻辑）：写错的交回哪个、期望什么。
fn read(args: &Value) -> Result<Asked, Bad> {
    let action = match args["action"].as_str() {
        Some("add") => Action::Add,
        Some("person") => Action::Person,
        Some("ranking") => Action::Ranking,
        _ => return Err("action must be add, person or ranking"),
    };
    let user = match &args["user"] {
        Value::Null => None,
        Value::Number(number) => Some(
            number
                .as_i64()
                .filter(|n| *n > 0)
                .ok_or("user must be a number")?,
        ),
        Value::String(text) => Some(
            text.trim()
                .parse::<i64>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or("user must be a number")?,
        ),
        _ => return Err("user must be a number"),
    };
    let text = |key: &str, most: usize, bad: Bad| -> Result<Option<String>, Bad> {
        match &args[key] {
            Value::Null => Ok(None),
            Value::String(text) => Ok(clean(text, most)),
            _ => Err(bad),
        }
    };
    let name = text("name", NAME, "name must be text")?;
    let note = text("note", NOTE, "note must be text")?;
    let amount = match &args["amount"] {
        Value::Null => None,
        value => Some(
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
                .filter(|amount: &f64| amount.is_finite() && *amount > 0.0 && *amount <= MOST)
                .ok_or("amount must be a number above 0")?,
        ),
    };
    let currency = match &args["currency"] {
        Value::Null => None,
        value => Some(
            value
                .as_str()
                .and_then(currency)
                .ok_or("currency must be a three-letter code such as CNY or USD")?,
        ),
    };
    let period = match &args["period"] {
        Value::Null => Period::All,
        value => value
            .as_str()
            .and_then(Period::read)
            .ok_or("period must be month, year or all")?,
    };
    Ok(Asked {
        action,
        user,
        name,
        amount,
        currency,
        note,
        period,
    })
}

/// 洗一段字：去掉控制字符，截到 `most` 个字；去掉首尾空白是空的交回空的。
fn clean(text: &str, most: usize) -> Option<String> {
    let cleaned: String = text
        .chars()
        .filter(|char| !char.is_control())
        .take(most)
        .collect();
    let cleaned = cleaned.trim().to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// 认出的赞助人：号（平台身份）、名字。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Who {
    user: Option<String>,
    name: Option<String>,
}

impl Who {
    /// 给模型看的：`小明 (qq:1)`、`qq:1`、`小明`。
    fn spoken(&self) -> String {
        match (&self.name, &self.user) {
            (Some(name), Some(user)) => format!("{name} ({user})"),
            (None, Some(user)) => user.clone(),
            (Some(name), None) => name.clone(),
            (None, None) => String::new(),
        }
    }
}

/// 谁（纯逻辑）：`asked` 写的号、引用的人、唯一 @ 的人、写的昵称，照这个先后；`venue` 是 im 的 `chat.venue` 的回答。都没有的
/// 是空的。
fn who(asked: &Asked, venue: &Value) -> Option<Who> {
    let platform = venue["platform"].as_str()?;
    let origin = &venue["origin"];
    let bot = venue["bot"].as_i64().map(|bot| format!("{platform}:{bot}"));
    let person = |one: &Value| -> Option<Who> {
        let id = one["id"].as_str()?.to_string();
        let name = one["name"].as_str().and_then(|name| clean(name, NAME));
        Some(Who {
            user: Some(id),
            name,
        })
    };
    // 照号找 origin 里认得的名字。
    let named = |id: &str| -> Option<String> {
        let quoted = std::iter::once(&origin["quote"]["sender"]);
        let mentioned = origin["mentions"].as_array().into_iter().flatten();
        quoted
            .chain(mentioned)
            .chain(std::iter::once(&origin["sender"]))
            .find(|one| one["id"] == id)
            .and_then(|one| one["name"].as_str())
            .and_then(|name| clean(name, NAME))
    };
    if let Some(user) = asked.user {
        let id = format!("{platform}:{user}");
        let name = asked.name.clone().or_else(|| named(&id));
        return Some(Who {
            user: Some(id),
            name,
        });
    }
    let not_her = |one: &Who| one.user.is_some() && one.user != bot;
    if let Some(quoted) = person(&origin["quote"]["sender"]).filter(not_her) {
        return Some(quoted);
    }
    let mentions: Vec<Who> = origin["mentions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(person)
        .filter(not_her)
        .collect();
    if let [only] = mentions.as_slice() {
        return Some(only.clone());
    }
    asked.name.clone().map(|name| Who {
        user: None,
        name: Some(name),
    })
}

/// 办一次调用要的：账本、设置、查汇率的一头、此刻（毫秒）、时区。
pub struct Context<'a> {
    /// 账本。
    pub book: &'a Mutex<Book>,
    /// 现在的设置。
    pub settings: Settings,
    /// 查汇率的一头。
    pub rates: &'a dyn Rates,
    /// 此刻（毫秒的 Unix 时间）。
    pub now: i64,
    /// 本机的时区（明细的日子、排行榜的这个月今年）。
    pub zone: TimeZone,
}

/// 一次 `tool.call`（`params` 是核心交的，`venue` 是 im 的 `chat.venue` 的回答，问不到的是 `null`）：交回工具结果。
pub fn call(params: &Value, venue: &Value, context: &Context<'_>) -> Value {
    let asked = match read(&params["args"]) {
        Ok(asked) => asked,
        Err(bad) => return answer(say("not-done", &[("detail", bad)]), true),
    };
    match asked.action {
        Action::Add => add(params, &asked, venue, context),
        Action::Person => person(&asked, venue, context),
        Action::Ranking => ranking(&asked, context),
    }
}

/// 一段字的结果。
fn answer(text: String, error: bool) -> Value {
    json!({"blocks": [{"type": "text", "text": text}], "error": error})
}

/// 这一轮是不是管理员叫的：核心认出的主人，或者场所的管理的人。
fn admin(params: &Value) -> bool {
    params["owner"] == true || params["by"]["role"] == "manager"
}

/// 叫她的人的平台身份：平台上的人照 `by.id`，私聊里认出的本人照 `by.via`。
fn caller(params: &Value) -> Option<String> {
    let by = &params["by"];
    by["id"]
        .as_str()
        .or_else(|| by["via"].as_str())
        .map(str::to_string)
}

/// 记一笔（模块的说明）。
fn add(params: &Value, asked: &Asked, venue: &Value, context: &Context<'_>) -> Value {
    if !admin(params) {
        return answer(say("admin-only", &[]), true);
    }
    let Some(platform) = venue["platform"].as_str() else {
        return answer(say("unreachable", &[]), true);
    };
    let Some(amount) = asked.amount else {
        return answer(say("not-done", &[("detail", "amount is needed")]), true);
    };
    let Some(who) = who(asked, venue) else {
        return answer(
            say(
                "not-done",
                &[("detail", "say who: user, name, or quote or @ the sponsor")],
            ),
            true,
        );
    };
    let base = context.settings.currency.clone();
    let currency = asked.currency.clone().unwrap_or_else(|| base.clone());
    let rate = if currency == base {
        Some(1.0)
    } else {
        context.rates.rate(&currency, &base)
    };
    let new = New {
        platform: platform.to_string(),
        user: who.user.clone(),
        name: who.name.clone(),
        amount,
        currency: currency.clone(),
        base: base.clone(),
        converted: rate.map(|rate| money::cents(amount * rate)),
        rate,
        at: context.now,
        note: asked.note.clone(),
        by: caller(params),
        venue: venue["venue"].as_str().map(str::to_string),
    };
    let written = context
        .book
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .add(new);
    let entry = match written {
        Ok(entry) => entry,
        Err(detail) => return answer(say("not-done", &[("detail", &detail)]), true),
    };
    let converted = match (entry.converted, entry.rate) {
        _ if currency == base => String::new(),
        (Some(converted), Some(rate)) => say(
            "converted",
            &[
                ("value", &format!("{} {base}", money::amount(converted))),
                ("rate", &rate.to_string()),
            ],
        ),
        _ => say("unconverted", &[("base", &base)]),
    };
    let fields = [
        ("id", entry.id.to_string()),
        ("who", who.spoken()),
        ("amount", format!("{} {currency}", money::amount(amount))),
        ("converted", converted),
    ];
    let fields: Vec<(&str, &str)> = fields
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    answer(say("recorded", &fields), false)
}

/// 查一个人（模块的说明）。
fn person(asked: &Asked, venue: &Value, context: &Context<'_>) -> Value {
    let Some(who) = who(asked, venue) else {
        return answer(
            say(
                "not-done",
                &[("detail", "say who: user, name, or quote or @ the sponsor")],
            ),
            true,
        );
    };
    let wanted = match (&who.user, &who.name) {
        (Some(user), _) => Sponsor::Id(user.clone()),
        (None, Some(name)) => Sponsor::Name(name.clone()),
        (None, None) => return answer(say("none", &[]), false),
    };
    let book = context.book.lock().unwrap_or_else(PoisonError::into_inner);
    let mine: Vec<&Entry> = book
        .entries()
        .iter()
        .filter(|entry| Sponsor::of(entry).as_ref() == Some(&wanted))
        .collect();
    if mine.is_empty() {
        return answer(say("none", &[]), false);
    }
    let base = &context.settings.currency;
    let total = money::total(mine.iter().copied(), base);
    let mut sum = format!("total {} {base}", money::amount(total.sum));
    if total.pending > 0 {
        sum.push_str(&format!(", {} not converted", total.pending));
    }
    let list: Vec<String> = mine
        .iter()
        .rev()
        .take(SHOWN)
        .map(|entry| {
            let mut line = format!(
                "{} {} {}",
                money::day(entry.at, &context.zone),
                money::amount(entry.amount),
                entry.currency
            );
            if let Some(note) = &entry.note {
                line.push_str(&format!(" ({note})"));
            }
            line
        })
        .collect();
    let named = Who {
        user: who.user.clone(),
        name: who
            .name
            .clone()
            .or_else(|| mine.iter().rev().find_map(|entry| entry.name.clone())),
    };
    let count = mine.len().to_string();
    let fields = [
        ("who", named.spoken()),
        ("count", count),
        ("total", sum),
        ("list", list.join("; ")),
    ];
    let fields: Vec<(&str, &str)> = fields
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    answer(say("person", &fields), false)
}

/// 排行榜（模块的说明）。
fn ranking(asked: &Asked, context: &Context<'_>) -> Value {
    let base = &context.settings.currency;
    let since = asked.period.since(context.now, &context.zone);
    let book = context.book.lock().unwrap_or_else(PoisonError::into_inner);
    let rows = money::ranking(book.entries(), base, since);
    if rows.is_empty() {
        return answer(say("none", &[]), false);
    }
    let list: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(index, (sponsor, name, sum))| {
            let who = match sponsor {
                Sponsor::Id(user) => Who {
                    user: Some(user.clone()),
                    name: name.clone(),
                },
                Sponsor::Name(name) => Who {
                    user: None,
                    name: Some(name.clone()),
                },
            };
            format!(
                "{}. {} {} {base}",
                index + 1,
                who.spoken(),
                money::amount(*sum)
            )
        })
        .collect();
    answer(
        say(
            "ranking",
            &[
                ("period", asked.period.spoken()),
                ("list", &list.join("; ")),
            ],
        ),
        false,
    )
}

#[cfg(test)]
mod tests;
