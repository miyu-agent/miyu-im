//! 查用量（施工 I-206，项目主人 2026-11-10：群聊里 AI 没法查 token 用量；`qq_token_usage`、`tg_token_usage` 同用）：
//! `<平台>_token_usage {hours?, user?}`，只给群，谁问都能查（主人定）。`hours` 是往回看几小时（1 到 720，没写的 24）。
//!
//! - 没写 `user`：这个群这段时间一共用了多少。合计照核心的用量账本（`usage.query` 照场所分，这个群的那一行：判断、看一眼、压缩这些
//!   没算到谁头上的请求也在里面），金额是核心换好的；再照 im 自己照人记的账本（`Call::usage`）列用得最多的五个人（`TOP`）。
//! - 写了 `user`（号，或者群成员的名字）：这个人这段时间在这个群叫她用掉的（只有算到人头上的主请求）。认不出是谁的答 `bad-args`。
//! - 金额：照人记的是原币种的数，交核心的 `usage.convert` 换成显示的币种；没换成的照币种接在后面。没记录金额的请求写有几次
//!   （项目主人定：算不准的钱不当成准的）。一点用量都没有的答 `no-usage`。

use serde_json::{Value, json};

use crate::chat::To;
use crate::toolset::{Call, Costs, NAME, Planned, Spender, clean};

use super::args::{Bad, bad_args, group_only, number};

/// 没写 `hours` 的看几小时。
const HOURS: i64 = 24;

/// 最多往回看几小时：30 天（im 的账本留 31 天）。
const MOST_HOURS: i64 = 720;

/// 列用得最多的几个人。
const TOP: usize = 5;

/// 一小时，毫秒。
const HOUR_MS: i64 = 60 * 60 * 1000;

/// 读参数（纯逻辑）：交回（几小时、`user` 写的字）。写错的交回哪个不对；`user` 写错的照 `bad_user`。
///
/// # Errors
///
/// 写错的交回哪一个不对。
pub fn wanted(args: &Value, bad_user: Bad) -> Result<(i64, Option<String>), Bad> {
    let hours = match &args["hours"] {
        Value::Null => HOURS,
        value => match number(value) {
            Some(hours) if (1..=MOST_HOURS).contains(&hours) => hours,
            _ => return Err("hours must be 1 to 720"),
        },
    };
    let user = match &args["user"] {
        Value::Null => None,
        Value::Number(number) => Some(number.to_string()),
        Value::String(user) if !user.trim().is_empty() => Some(user.trim().to_string()),
        _ => return Err(bad_user),
    };
    Ok((hours, user))
}

/// `user` 写的字是这个群里谁（纯逻辑）：全是数字的照号；别的照群成员的名字，整个一样的（不分大小写），只有一个对得上才算。
fn who(call: &Call, user: &str) -> Option<String> {
    if user.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("{}:{user}", call.platform.name()));
    }
    let wanted = user.to_lowercase();
    let mut named = call
        .names
        .iter()
        .filter(|(_, name)| name.to_lowercase() == wanted);
    match (named.next(), named.next()) {
        (Some((id, _)), None) => Some(format!("{}:{id}", call.platform.name())),
        _ => None,
    }
}

/// 平台上的人 `person`（`qq:1`）在这个群里叫什么：群成员的名字，没有的写号。
fn named(call: &Call, person: &str) -> String {
    let number = person.split_once(':').map_or(person, |(_, number)| number);
    call.names
        .iter()
        .find(|(id, _)| id.as_str() == number)
        .and_then(|(_, name)| clean(name, NAME))
        .unwrap_or_else(|| number.to_string())
}

/// 一个数照人的写法（纯逻辑）：`1.2M`、`34.5k`、`987`。
pub fn short(tokens: u64) -> String {
    // 换成小数只为了写给人看，差一点不要紧。
    let value = tokens as f64;
    match tokens {
        1_000_000.. => format!("{:.1}M", value / 1e6),
        1_000.. => format!("{:.1}k", value / 1e3),
        _ => tokens.to_string(),
    }
}

/// 金额的数（纯逻辑，施工 I-209）：照网页用量页的写法，不到 1 的多留两位（`0.0832`，后面多出来的 0 不写），免得不到一分钱的
/// 写成 `0.00` 让人读成没花钱；小到四位都是 0 的写 `<0.0001`。
fn digits(amount: f64) -> String {
    if amount.abs() >= 1.0 {
        return format!("{amount:.2}");
    }
    let written = format!("{amount:.4}");
    if amount != 0.0 && written.trim_start_matches('-') == "0.0000" {
        return "<0.0001".to_string();
    }
    let kept = written.trim_end_matches('0').len().max(written.len() - 2);
    written[..kept].to_string()
}

/// 核心换好的一项 `converted`（`{currency, amount, rest?}`，没换成的是 `null`）写成一句（纯逻辑）：`1.23 CNY + 0.10 USD`；
/// 什么都没有的是空的。
pub fn money(converted: &Value) -> Option<String> {
    let one = |amount: &Value, currency: &Value| {
        let (amount, currency) = (amount.as_f64()?, currency.as_str()?);
        Some(format!("{} {currency}", digits(amount)))
    };
    let mut parts: Vec<String> = one(&converted["amount"], &converted["currency"])
        .into_iter()
        .collect();
    for rest in converted["rest"].as_array().into_iter().flatten() {
        parts.extend(one(&rest["amount"], &rest["currency"]));
    }
    (!parts.is_empty()).then(|| parts.join(" + "))
}

/// 几个人的金额交核心换（一个人一组），照先后交回每个人的那一句；换不成的照原币种加起来写。
async fn costs_of(call: &Call, people: &[&Costs]) -> Vec<Option<String>> {
    let groups: Vec<Vec<Value>> = people
        .iter()
        .map(|costs| {
            let part = |(day, currency, amount): &(String, String, f64)| {
                json!({"day": day, "currency": currency, "amount": amount})
            };
            costs.parts.iter().map(part).collect()
        })
        .collect();
    let answered = match groups.iter().any(|group| !group.is_empty()) {
        true => {
            call.usage
                .core("usage.convert", json!({ "groups": groups }))
                .await
        }
        false => None,
    };
    people
        .iter()
        .enumerate()
        .map(|(at, costs)| {
            let converted = answered.as_ref().map(|got| &got["converted"][at]);
            match converted.filter(|one| one.is_object()) {
                Some(converted) => money(converted),
                None => money(&json!({ "rest": original(costs) })),
            }
        })
        .collect()
}

/// 没换成的：原币种的数照币种加起来（纯逻辑）。
fn original(costs: &Costs) -> Vec<Value> {
    let mut sums: std::collections::BTreeMap<&str, f64> = Default::default();
    for (_, currency, amount) in &costs.parts {
        *sums.entry(currency).or_default() += amount;
    }
    sums.into_iter()
        .map(|(currency, amount)| json!({"currency": currency, "amount": amount}))
        .collect()
}

/// 一次调用（模块的说明）；`user` 写错的答 `bad_user`。
pub fn usage(call: Call, bad_user: Bad) -> Planned {
    if !matches!(call.to, To::Group(_)) {
        return group_only(&call);
    }
    let (hours, user) = match wanted(&call.args, bad_user) {
        Ok(wanted) => wanted,
        Err(bad) => return bad_args(&call, bad),
    };
    let person = match &user {
        Some(user) => match who(&call, user) {
            Some(person) => Some(person),
            None => return bad_args(&call, bad_user),
        },
        None => None,
    };
    Planned::later(async move {
        let until = crate::records::now();
        let from = until - hours * HOUR_MS;
        let spenders = match call.usage.spenders(&call.venue, from, person.clone()).await {
            Ok(spenders) => spenders,
            Err(detail) => return call.answers.say("failed", &[("detail", &detail)], true),
        };
        let hours = hours.to_string();
        match person {
            Some(person) => one(&call, &hours, &person, spenders.first()).await,
            None => whole(&call, &hours, (from, until), &spenders).await,
        }
    })
}

/// 一个人的。
async fn one(call: &Call, hours: &str, person: &str, spent: Option<&Spender>) -> Value {
    let who = named(call, person);
    let Some(spent) = spent.filter(|spent| spent.tokens > 0) else {
        return call
            .answers
            .say("no-usage-user", &[("hours", hours), ("who", &who)], false);
    };
    let cost = costs_of(call, &[&spent.costs]).await.remove(0);
    let fields = [
        ("hours", hours),
        ("who", who.as_str()),
        ("tokens", &short(spent.tokens)),
        ("cost", &priced(cost, spent.costs.unpriced)),
    ];
    call.answers.say("usage-user", &fields, false)
}

/// 整个群的：合计照核心的账本，最多的几个人照 im 的。
async fn whole(call: &Call, hours: &str, (from, until): (i64, i64), spenders: &[Spender]) -> Value {
    let stamp = |at: i64| {
        miyu_client::time::Timestamp::from_unix_millis(at)
            .map(|at| at.to_string())
            .unwrap_or_default()
    };
    let asked = json!({"from": stamp(from), "until": stamp(until), "group": ["venue"]});
    let rows = call.usage.core("usage.query", asked).await;
    let row = rows.as_ref().and_then(|rows| {
        let all = rows["rows"].as_array()?;
        all.iter().find(|row| row["venue"] == call.venue.as_str())
    });
    let count = |key: &str| row.map_or(0, |row| row["usage"][key].as_u64().unwrap_or_default());
    let tokens = ["uncached", "cache_read", "cache_write", "output"]
        .into_iter()
        .map(count)
        .sum::<u64>();
    let requests = row.map_or(0, |row| row["requests"].as_u64().unwrap_or_default());
    if tokens == 0 && spenders.is_empty() {
        return call.answers.say("no-usage", &[("hours", hours)], false);
    }
    let cost = row.and_then(|row| money(&row["converted"]));
    let unpriced = row.map_or(0, |row| row["unpriced"].as_u64().unwrap_or_default());
    let top: Vec<&Spender> = spenders.iter().take(TOP).collect();
    let costs: Vec<&Costs> = top.iter().map(|one| &one.costs).collect();
    let each = costs_of(call, &costs).await;
    let list: Vec<String> = top
        .iter()
        .zip(each)
        .map(|(one, cost)| {
            let cost = cost.map(|cost| format!(", {cost}")).unwrap_or_default();
            format!("{} {}{cost}", named(call, &one.person), short(one.tokens))
        })
        .collect();
    let list = match list.is_empty() {
        true => "-".to_string(),
        false => list.join("; "),
    };
    let fields = [
        ("hours", hours),
        ("tokens", &short(tokens)),
        ("requests", &requests.to_string()),
        ("cost", &priced(cost, unpriced)),
        ("list", &list),
    ];
    call.answers.say("usage", &fields, false)
}

/// 金额那一格（纯逻辑）：有的照写；没记录金额的请求有几次接在后面；都没有的写 `no cost recorded`。
pub fn priced(cost: Option<String>, unpriced: u64) -> String {
    match (cost, unpriced) {
        (Some(cost), 0) => cost,
        (Some(cost), count) => format!("{cost} ({count} more request(s) have no recorded cost)"),
        (None, 0) => "no cost recorded".to_string(),
        (None, count) => format!("no cost recorded for {count} request(s)"),
    }
}

#[cfg(test)]
mod tests;
