//! 读群公告（施工 I-37，2026-10-23 项目主人定：她能读群公告，群规、活动通知这些她就答得出）：`qq_notices`，没有参数，只给群。
//! 照 NapCat 的 `_get_group_notice {group_id}` 取，新的在前，最多 [`MOST`] 条，每条的正文最多 [`TEXT`] 个字；只读。
//!
//! NapCat 回的一条：`sender_id`、`publish_time`（整数秒）、`message.text`（有的实现直接给 `message` 是字、或 `content`）。读不出
//! 正文的那条不列。名字照这个群缓存着的，没有的写号；正文过 `clean` 洗掉控制字符。

use serde_json::{Value, json};

use miyu_im::chat::To;
use miyu_im::toolset::{Call, Planned, clean};

use super::args::group_only;
use super::lookup::{said, undone};

/// 最多列几条。
const MOST: usize = 10;

/// 一条的正文最多几个字。
const TEXT: usize = 500;

/// 一条公告：谁发的、什么时候（整数秒）、正文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Notice {
    pub(crate) sender: Option<i64>,
    pub(crate) time: i64,
    pub(crate) text: String,
}

/// NapCat 回的 `data` 读成几条公告（纯逻辑）：新的在前，最多 [`MOST`] 条；读不出正文的不要。
pub(crate) fn notices(data: &Value) -> Vec<Notice> {
    let mut read: Vec<Notice> = data
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|one| {
            let raw = one["message"]["text"]
                .as_str()
                .or_else(|| one["message"].as_str())
                .or_else(|| one["content"].as_str())?;
            // 换行写成空格（`clean` 去掉控制字符，不换就粘在一起）。
            let text = clean(raw.replace('\n', " ").trim(), TEXT)?;
            Some(Notice {
                sender: one["sender_id"].as_i64(),
                time: one["publish_time"].as_i64().unwrap_or_default(),
                text,
            })
        })
        .collect();
    read.sort_by_key(|notice| std::cmp::Reverse(notice.time));
    read.truncate(MOST);
    read
}

/// 整数秒写成 UTC 的日期 `2026-10-23`（纯逻辑；0 和负的写 `?`）。
pub(crate) fn day(seconds: i64) -> String {
    if seconds <= 0 {
        return "?".to_string();
    }
    // Howard Hinnant 的 civil_from_days。
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `qq_notices` 的一次调用（模块的说明）。
pub(super) fn notices_of(call: Call) -> Planned {
    let To::Group(group) = call.to else {
        return group_only(&call);
    };
    Planned::later(async move { read(&call, group).await })
}

/// 取、答那一句。
async fn read(call: &Call, group: i64) -> Value {
    let Some(link) = &call.link else {
        return call.answers.say("unreachable", &[], true);
    };
    let reply = match link
        .raw("_get_group_notice", json!({"group_id": group}))
        .await
    {
        Ok(reply) if reply["status"] == "ok" => reply,
        Ok(reply) => return said(call, undone(crate::onebot::failed(&reply))),
        Err(error) => return said(call, undone(error)),
    };
    let read = notices(&reply["data"]);
    if read.is_empty() {
        return call.answers.say("no-notices", &[], false);
    }
    let list: Vec<String> = read
        .iter()
        .map(|notice| {
            let who = notice.sender.map_or_else(
                || "?".to_string(),
                |sender| {
                    call.names
                        .get(&sender)
                        .cloned()
                        .unwrap_or_else(|| sender.to_string())
                },
            );
            format!("[{} {who}] {}", day(notice.time), notice.text)
        })
        .collect();
    let count = list.len().to_string();
    call.answers.say(
        "notices",
        &[("count", &count), ("list", &list.join("; "))],
        false,
    )
}

#[cfg(test)]
mod tests;
