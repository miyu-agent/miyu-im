//! 开一轮时告诉她为什么叫她（施工 I-16，照旧版 `real_context/inject.rs`、`targeting.rs`；`chat.md` 第三条第 7 条「回合开头那句」、
//! 第五条第 6 条）：判下来要回的一轮，`session.respond` 的事实里多带一两块，模板是出厂数据（`rules::facts`），给模型的字登记在
//! `docs/ledger.md`。
//!
//! - 主触发是抽中的（插话）：带 `join-in`，旧版实测不带她会说「没 @ 我不接」。
//! - 因为违规才回的（只查违规那一次判了违规；或者打分没过门槛、严重程度过了）：带 `moderation-precheck`，写判官说的严重程度和理由。
//!
//! 纯逻辑，模板由外面交进来；渲染不出的（照说不会：起来时查过字段）记一行，不带。

use miyu_chat::{Kind, Route as Path};
use miyu_client::protocol::Template;
use serde_json::{Value, json};

use super::body::Judged;
use super::decide::Decision;
use crate::TARGET;

/// 抽中插话那一块的 `kind`。
const JOIN_IN: &str = "join-in";

/// 违规预检那一块的 `kind`。
const PRECHECK: &str = "moderation-precheck";

/// 这一轮要带的几块事实：`join_in`、`precheck` 是两份模板。
pub(super) fn primer(
    decision: &Decision,
    judged: Option<&Judged<'_>>,
    (join_in, precheck): (&Template, &Template),
) -> Vec<Value> {
    let mut facts = Vec::new();
    let Some(passed) = &decision.passed else {
        return facts;
    };
    if passed.conditions.primary() == Some(Kind::Probability) {
        facts.extend(render(join_in, &[], JOIN_IN));
    }
    let judgement = judged.and_then(|judged| judged.answer.result.as_ref().ok());
    let score = judged.and_then(|judged| judged.score);
    let violated = match passed.route {
        Path::ModerationOnly => score.is_some_and(|score| score.reply),
        _ => score.is_some_and(|score| score.reply && score.total < score.threshold),
    };
    if let (true, Some(judgement)) = (violated, judgement) {
        let severity = judgement.severity.unwrap_or_default().to_string();
        let fields = [
            ("severity", severity.as_str()),
            ("reason", judgement.reason.as_str()),
        ];
        facts.extend(render(precheck, &fields, PRECHECK));
    }
    facts
}

/// 照模板 `template` 填 `fields`，套成种类是 `kind` 的一块事实。
fn render(template: &Template, fields: &[(&str, &str)], kind: &str) -> Option<Value> {
    match template.render(&fields.iter().copied().collect()) {
        Ok(text) => Some(json!({"kind": kind, "text": text})),
        Err(error) => {
            tracing::warn!(target: TARGET, kind, error = %error, "turn fact not written");
            None
        }
    }
}
