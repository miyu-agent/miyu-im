//! 导入、导出（施工 I-131，设计 03 第四节；项目主人 11-04 定：迁移程序把旧版的赞助记录导进来，导出给备份和搬家用）。
//!
//! - `export` → `{version, entries}`：账本里的每一笔原样交出去（带编号），先记的在前。
//! - `import {entries}` → `{added, skipped}`：一笔一个对象，格照账本（`platform`、`user?`、`name?`、`amount`、`currency`、`base`、
//!   `converted?`、`rate?`、`at`、`note?`、`by?`、`venue?`），`at` 是原来的时刻（毫秒），照写、不换成此刻；汇率照给的，不再查；
//!   `id` 不认，编号由账本给（导出的东西原样能导回来）。
//! - 整份先查，有一笔不对的整份不导，回 -32602，`message` 说第几笔哪里不对。
//! - 去重：同平台、同一个人（有号认号，没号认名字）、同一刻、同金额、同币种的算同一笔，账本里有的、这一份里前面有的都跳过；所以
//!   重导是空操作。删过的那一笔账本里不留痕迹，重导会回来（迁移只跑一次，设计 03 第四节写着）。
//! - 一次写回：加的几笔一起写，写不进去的一笔都不算加。

use std::collections::HashSet;

use serde_json::{Value, json};

use super::{LARGEST, NAME, NOTE, State, amount, bad, identity, locked, text};
use crate::settings::currency;
use crate::store::{Entry, New};
use crate::wire::Answer;

/// 导出的格式版本：导入不看它，以后格变了再认。
const VERSION: u32 = 1;

/// 导出（模块的说明）。
pub(super) fn export(state: &State) -> Answer {
    Ok(json!({"version": VERSION, "entries": locked(state).entries()}))
}

/// 导入（模块的说明）。
pub(super) fn import(state: &State, params: &Value) -> Answer {
    let given = params["entries"]
        .as_array()
        .ok_or_else(|| bad("entries must be a list"))?;
    let mut news = Vec::with_capacity(given.len());
    for (index, one) in given.iter().enumerate() {
        news.push(read(one).map_err(|why| bad(&format!("entries[{index}]: {why}")))?);
    }
    let mut book = locked(state);
    let mut seen: HashSet<Key> = book.entries().iter().map(Key::of_entry).collect();
    let mut fresh = Vec::new();
    for new in news {
        if seen.insert(Key::of_new(&new)) {
            fresh.push(new);
        }
    }
    let skipped = given.len() - fresh.len();
    let added = book
        .add_all(fresh)
        .map_err(|why| json!({"code": -32000, "message": why}))?
        .len();
    Ok(json!({"added": added, "skipped": skipped}))
}

/// 读一笔：不对的交回哪里不对（英文短句，同别的方法的 -32602）。
fn read(one: &Value) -> Result<New, String> {
    if !one.is_object() {
        return Err("must be an object".to_string());
    }
    let text = |key: &str, most: usize| text(one, key, most).map_err(message);
    let platform = text("platform", NAME)?.ok_or("platform is needed")?;
    let user = text("user", NAME)?.map(|user| identity(&platform, &user));
    let name = text("name", NAME)?;
    if user.is_none() && name.is_none() {
        return Err("user or name is needed".to_string());
    }
    let code = |key: &str| {
        one[key]
            .as_str()
            .and_then(currency)
            .ok_or(format!("{key} must be a three-letter code"))
    };
    let optional = |key: &str, ok: fn(f64) -> bool| match &one[key] {
        Value::Null => Ok(None),
        value => value
            .as_f64()
            .filter(|number| ok(*number))
            .map(Some)
            .ok_or(format!("{key} must be a number")),
    };
    let at = one["at"]
        .as_i64()
        .filter(|at| *at > 0)
        .ok_or("at must be milliseconds since 1970")?;
    Ok(New {
        user,
        name,
        amount: amount(&one["amount"]).map_err(message)?,
        currency: code("currency")?,
        base: code("base")?,
        converted: optional("converted", |number| {
            number.is_finite() && (0.0..=LARGEST).contains(&number)
        })?,
        rate: optional("rate", |number| number.is_finite() && number > 0.0)?,
        at,
        note: text("note", NOTE)?,
        by: text("by", NAME)?.map(|by| identity(&platform, &by)),
        venue: text("venue", NAME * 2)?,
        platform,
    })
}

/// -32602 的那一句。
fn message(error: Value) -> String {
    error["message"].as_str().unwrap_or_default().to_string()
}

/// 去重认的那几样（模块的说明）：金额照位比（导出的再导回来一模一样）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Key {
    platform: String,
    who: String,
    at: i64,
    amount: u64,
    currency: String,
}

impl Key {
    fn new(
        platform: &str,
        (user, name): (Option<&String>, Option<&String>),
        (at, amount, currency): (i64, f64, &str),
    ) -> Key {
        Key {
            platform: platform.to_string(),
            who: user.or(name).cloned().unwrap_or_default(),
            at,
            amount: amount.to_bits(),
            currency: currency.to_string(),
        }
    }

    fn of_entry(entry: &Entry) -> Key {
        Key::new(
            &entry.platform,
            (entry.user.as_ref(), entry.name.as_ref()),
            (entry.at, entry.amount, &entry.currency),
        )
    }

    fn of_new(new: &New) -> Key {
        Key::new(
            &new.platform,
            (new.user.as_ref(), new.name.as_ref()),
            (new.at, new.amount, &new.currency),
        )
    }
}
