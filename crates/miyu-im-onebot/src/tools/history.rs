//! 查聊天记录（施工 C-1 下，设计 04 第一节第 4 条）：`qq_chat_history {query?, user?, from?, to?, all_chats?}`，私聊、群都给，
//! 只读。照 im 的聊天记录库搜（`Call::history`）：关键词、人（号或名字）、时间段（本机时区的 `2026-10-20` 或 `2026-10-20 14:03`，只写
//! 日子的「到」算到那天结束）；没写 `all_chats` 的只查这个场所，写了的要这一轮是终端管理员叫的（项目主人 10-23 定），别的答
//! `history-admin-only`。回复判断不在库的这一头，她查不到。
//!
//! 搜到的取最新的 [`SHOWN`] 条（工具面预算紧，不给 `limit`；要更早的照时间段再查），照先后从旧到新一行一条：时刻、（跨场所的）哪个场所、谁（她自己写
//! `you`）、`msg=` 编号（拿去引用、取图）、正文（洗掉控制字符、换行写成空格、最多 [`TEXT`] 个字）、带的东西的占位、引用了哪条、撤回了的
//! 标上。头一行照模板，一行一条接在后面（同查管理记录）。

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use serde_json::{Value, json};

use miyu_im::toolset::{Call, HistoryQuery, NAME, Planned, clean, clock};

use super::args::{Bad, bad_args, optional_text};

/// 一次列几条。
const SHOWN: i64 = 30;

/// 一条的正文最多几个字。
const TEXT: usize = 300;

/// 查什么（读好的参数）。
#[derive(Debug, Clone, PartialEq, Eq)]
struct Asked {
    text: String,
    user: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
    everywhere: bool,
}

/// 读参数 `args`，时刻照时区 `zone`（纯逻辑）：写错的交回哪个参数、期望什么。
fn read(args: &Value, zone: &TimeZone) -> Result<Asked, Bad> {
    let text = optional_text(args, "query", "query must be text")?
        .unwrap_or_default()
        .trim()
        .to_string();
    let user = match &args["user"] {
        Value::Null => None,
        Value::Number(number) => Some(number.to_string()),
        Value::String(user) => Some(user.trim().to_string()).filter(|user| !user.is_empty()),
        _ => return Err("user must be a QQ number or a name"),
    };
    let bad_time = "from and to must be YYYY-MM-DD or YYYY-MM-DD HH:MM";
    let from = match optional_text(args, "from", bad_time)? {
        None => None,
        Some(from) => Some(moment(from, zone, false).ok_or(bad_time)?),
    };
    let to = match optional_text(args, "to", bad_time)? {
        None => None,
        Some(to) => Some(moment(to, zone, true).ok_or(bad_time)?),
    };
    if let (Some(from), Some(to)) = (from, to)
        && from >= to
    {
        return Err("from must be before to");
    }
    let everywhere = match &args["all_chats"] {
        Value::Null => false,
        Value::Bool(everywhere) => *everywhere,
        _ => return Err("all_chats must be true or false"),
    };
    Ok(Asked {
        text,
        user,
        from,
        to,
        everywhere,
    })
}

/// 本机时区 `zone` 的 `2026-10-20` 或 `2026-10-20 14:03` 读成毫秒（纯逻辑）；只写日子的，`end` 是真的算到那天结束（第二天零点）。
fn moment(text: &str, zone: &TimeZone, end: bool) -> Option<i64> {
    let text = text.trim();
    let at = match DateTime::strptime("%Y-%m-%d %H:%M", text) {
        Ok(at) => at,
        Err(_) => {
            let day: Date = text.parse().ok()?;
            let day = if end { day.tomorrow().ok()? } else { day };
            day.to_datetime(jiff::civil::Time::midnight())
        }
    };
    let zoned = at.to_zoned(zone.clone()).ok()?;
    Some(zoned.timestamp().as_millisecond())
}

/// `qq_chat_history` 的一次调用（模块的说明）。
pub(super) fn history(call: Call) -> Planned {
    let asked = match read(&call.args, &TimeZone::system()) {
        Ok(asked) => asked,
        Err(bad) => return bad_args(&call, bad),
    };
    if asked.everywhere && !call.admin {
        return Planned::Now(call.answers.say("history-admin-only", &[], true));
    }
    Planned::later(async move {
        let query = query_of(&asked, &call.venue);
        match call.history.search(query).await {
            Ok(found) => answered(&call, &found, asked.everywhere),
            Err(detail) => call.answers.say("failed", &[("detail", &detail)], true),
        }
    })
}

/// 照读好的参数 `asked` 拼搜的条件（纯逻辑）：没写 `all_chats` 的只搜场所 `venue`。
fn query_of(asked: &Asked, venue: &str) -> HistoryQuery {
    HistoryQuery {
        text: asked.text.clone(),
        venue: (!asked.everywhere).then(|| venue.to_string()),
        sender: asked.user.clone(),
        from: asked.from,
        to: asked.to,
        limit: SHOWN,
    }
}

/// 答的那一句：搜到的 `found`（新的在前），`everywhere` 的每一行写哪个场所。
fn answered(call: &Call, found: &[Value], everywhere: bool) -> Value {
    if found.is_empty() {
        return call.answers.say("no-history", &[], false);
    }
    let me = call.me.as_ref().map(|me| me.as_str().to_string());
    let lines: Vec<String> = found
        .iter()
        .rev()
        .map(|one| line(one, me.as_deref(), everywhere))
        .collect();
    let shown = lines.len().to_string();
    let head = call.answers.text("history", &[("count", shown.as_str())]);
    // 模板换字会把字段里的换行转义掉：一行一条接在头一行后面，行里外面来的字先洗掉控制字符。
    let text = format!("{}\n{}\n", head.trim_end(), lines.join("\n"));
    json!({"blocks": [{"type": "text", "text": text}], "error": false})
}

/// 一条一行（纯逻辑）：`2026-10-20 14:03 小明 (40001) msg=123: 今晚打团本吗 [image] (reply to msg=99) (recalled)`；`me` 是她自己的
/// 平台身份，`everywhere` 的时刻后面写 `group 31,`。
fn line(one: &Value, me: Option<&str>, everywhere: bool) -> String {
    let at = one["at"].as_i64().unwrap_or_default();
    let mut line = clock(u64::try_from(at / 1000).unwrap_or_default());
    if everywhere {
        // 从右边拆：最后一段是号、倒数第二段是种类（施工 I-81 起中间多一段机器人号，号里没有 `:`）。
        let venue = one["venue"].as_str().unwrap_or_default();
        let mut parts = venue.rsplit(':');
        if let (Some(id), Some(kind)) = (parts.next(), parts.next()) {
            line.push_str(&format!(" {kind} {id},"));
        }
    }
    let sender = one["sender"].as_str().unwrap_or_default();
    let number = sender.rsplit(':').next().unwrap_or_default();
    let who = if one["mine"] == true || me == Some(sender) {
        "you".to_string()
    } else {
        match one["name"].as_str().and_then(|name| clean(name, NAME)) {
            Some(name) => format!("{name} ({number})"),
            None => number.to_string(),
        }
    };
    let msg = one["msg"].as_str().unwrap_or_default();
    line.push_str(&format!(" {who} msg={msg}:"));
    let text = one["text"].as_str().unwrap_or_default().replace('\n', " ");
    if let Some(text) = clean(text.trim(), TEXT) {
        line.push(' ');
        line.push_str(&text);
    }
    for media in one["media"].as_array().into_iter().flatten() {
        line.push_str(&placeholder(media));
    }
    if let Some(quoted) = one["reply_to"].as_str() {
        line.push_str(&format!(" (reply to msg={quoted})"));
    }
    if one["recalled_at"].is_i64() {
        line.push_str(" (recalled)");
    }
    line
}

/// 带的一样东西的占位：` [image]`、` [file 排班.pdf]`。
fn placeholder(media: &Value) -> String {
    let kind = media["kind"].as_str().unwrap_or("file");
    match media["name"].as_str().and_then(|name| clean(name, NAME)) {
        Some(name) => format!(" [{kind} {name}]"),
        None => format!(" [{kind}]"),
    }
}

#[cfg(test)]
mod tests;
