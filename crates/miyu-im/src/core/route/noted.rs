//! 一笔判断说成会话浮窗「最近判断」的一条原因（施工 I-7 上补，`crate::recent`）：照记进会话的那一份判断（`body`，`body.rs`）说，
//! 纯逻辑。页面照原因的编号写字。
//!
//! - 进站链就只记下的：`off`（这个会话关着回复）、`asleep`（睡眠时间）、`muted`（她被禁言着）、`rate`（限流满了）、`repeated`（几个人
//!   在复读，施工 I-28；I-34 下以前说成 `off`）。
//! - 放行了的：没有成立的条件（没叫她、没抽中）的 `quiet`；有条件却只记下的 `skip_user`（在跳过判断的人里，施工 I-7 中三；I-16 起跳过判断的人只有插话的才判、不再只记下，这一格只剩以前记的）；直接回的 `called`（叫了她）、`continued`（续聊）、`after_speaking`
//!   （她刚说过话）；问了判官、算了分的 `judged`，带 `total`、`threshold`；判不了的 `unjudged`；只做管理检查的 `moderation`。
//! - 问判官、直接回的带 `trigger`：主触发是哪一种条件（`direct`、`continuation`、`followup`（施工 I-21）、`after_speaking`、`probability`、
//!   `moderation`）。
//! - 进站挂接点改了结论的带 `hook`：哪个包。
//! - 判官给了理由的带 `reason_text`（施工 I-16：浮窗照它写一行小字），照原样。

use serde_json::{Value, json};

/// 主触发照这个先后挑（同群聊内核的 `Conditions::primary`）。
const PRIMARY: [&str; 6] = [
    "direct",
    "continuation",
    "followup",
    "after_speaking",
    "probability",
    "moderation",
];

/// 判断 `body` 的原因：`{"reason", "trigger"?, "total"?, "threshold"?, "hook"?}`。
pub(super) fn why(body: &Value) -> Value {
    let mut why = reason(body);
    if let Some(package) = body["hook"]["package"].as_str() {
        why["hook"] = json!(package);
    }
    if let Some(text) = body["judge"]["answer"]["reason"]
        .as_str()
        .filter(|text| !text.trim().is_empty())
    {
        why["reason_text"] = json!(text);
    }
    why
}

/// 不带挂接点的那一段。
fn reason(body: &Value) -> Value {
    if body["inbound"] == "record_only" {
        let reason = match body["why"].as_str() {
            Some("asleep") => "asleep",
            Some("muted") => "muted",
            Some("rate_limited") => "rate",
            Some("repeated") => "repeated",
            _ => "off",
        };
        return json!({"reason": reason});
    }
    let trigger = trigger(body);
    match body["route"].as_str() {
        Some("commit") => {
            let reason = match trigger {
                Some("continuation") => "continued",
                Some("after_speaking") => "after_speaking",
                _ => "called",
            };
            json!({"reason": reason})
        }
        Some("judge") => match body.get("score") {
            Some(score) => json!({
                "reason": "judged",
                "trigger": trigger,
                "total": score["total"],
                "threshold": score["threshold"],
            }),
            None => json!({"reason": "unjudged", "trigger": trigger}),
        },
        Some("moderation_only") => json!({"reason": "moderation"}),
        // 有条件却只记下：发的人在「跳过主动回复判断的用户」里、没冲她来（施工 I-7 中三，群聊内核的 `route`）。
        Some("record") if trigger.is_some() => json!({"reason": "skip_user"}),
        _ => json!({"reason": "quiet"}),
    }
}

/// 成立的条件里的主触发。
fn trigger(body: &Value) -> Option<&'static str> {
    let hits: Vec<&str> = body["conditions"]
        .as_array()
        .map(|hits| hits.iter().filter_map(|hit| hit["kind"].as_str()).collect())
        .unwrap_or_default();
    PRIMARY.into_iter().find(|kind| hits.contains(kind))
}

#[cfg(test)]
mod tests;
