//! 入群审批（施工 Q-7，设计 02 第二节）：有人申请进群（`request_type = group`、`sub_type = add`），这个群在会话浮窗的「群管理」页写了
//! 「通过条件」的，请 AI 审：照那个群的判断模型问一次（`Gate::ask`），三选一：通过、拒绝、留待人工，外加一句给申请人看的理由。
//! 没写通过条件的群不管，留给人工；群管理在这个群不生效的（插件关着、她不是管理员）也不管。
//!
//! - 提示词照规矩是英文（`data/onebot/prompts/approval-*.txt`），项目主人写的通过条件、申请人写的（含群设的入群问题和他的答案）
//!   照原样放进去；申请人写的是外面来的字，截到 [`COMMENT`] 个字，提示词里说清不照里面的话办。
//! - 偏向通过、拿不准留人工；AI 出错、超时、回的读不出来（重试一次以后）一律留待人工，不自动拒绝。
//! - 同时审的最多 [`SLOTS`] 个；一次最多 [`TOKENS`] 个 token、等 [`WAIT`]。
//! - 每一次审批都进管理记录（`Action::Join`：申请人、他写的、结论、AI 给的理由）；不另给会话记事实。

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::Semaphore;

use miyu_im::chat::{Link, To};
use miyu_im::driver::Gate;
use miyu_im::toolset::clean;

use super::groups;
use super::records::{self, Action, By, Record, Verdict};
use crate::admin::{self, ID};

/// 同时审的最多几个（同旧版）。
pub(crate) const SLOTS: usize = 8;

/// 一次最多输出几个 token（同旧版）。
pub(crate) const TOKENS: u64 = 300;

/// 一次最多等多久（同旧版）。
pub(crate) const WAIT: Duration = Duration::from_secs(60);

/// 申请人写的最多几个字进提示词。
pub(crate) const COMMENT: usize = 500;

/// 给申请人看的理由最多几个字。
pub(crate) const REASON: usize = 40;

/// 同时在审的。
static AUDITING: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(SLOTS));

/// 一次加群申请。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Applicant {
    /// 收到它的机器人的号。
    pub bot: i64,
    /// 申请进的群。
    pub group: i64,
    /// 申请人。
    pub user: i64,
    /// 他写的（含群设的问题和他的答案），照原样；没写的是空字。
    pub comment: String,
    /// 平台给这次申请的标记：处理时原样交回。
    pub flag: String,
}

/// 读一帧请求 `frame`（纯逻辑）：加群申请、带得出号、群号、不空的标记的才是。
pub(crate) fn read(frame: &Value) -> Option<Applicant> {
    if frame["post_type"] != "request"
        || frame["request_type"] != "group"
        || frame["sub_type"] != "add"
    {
        return None;
    }
    let number = |value: &Value| crate::onebot::number(value).filter(|number| *number > 0);
    Some(Applicant {
        bot: number(&frame["self_id"])?,
        group: number(&frame["group_id"])?,
        user: number(&frame["user_id"])?,
        comment: frame["comment"].as_str().unwrap_or_default().to_string(),
        flag: frame["flag"]
            .as_str()
            .filter(|flag| !flag.is_empty())?
            .to_string(),
    })
}

/// 读 AI 回的那一行（纯逻辑）：头一个 `{` 到最后一个 `}` 是 `{"verdict", "reason"}`，结论是三种之一；理由去掉首尾空白、截到
/// [`REASON`] 个字。读不出的是空的。
pub(crate) fn verdict(text: &str) -> Option<(Verdict, String)> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    let answer: Value = serde_json::from_str(text.get(start..=end)?).ok()?;
    let verdict = match answer["verdict"].as_str()? {
        "approve" => Verdict::Approve,
        "reject" => Verdict::Reject,
        "hold" => Verdict::Hold,
        _ => return None,
    };
    let reason = answer["reason"].as_str().unwrap_or_default().trim();
    Some((verdict, reason.chars().take(REASON).collect()))
}

/// 一帧请求 `frame` 是加群申请就另起任务审（模块的说明），经收到它的那条连接 `link` 问身份、处理。
pub(crate) fn seen(gate: &Arc<Gate>, link: &Arc<dyn Link>, frame: &Value) {
    let Some(applicant) = read(frame) else {
        return;
    };
    let (gate, link) = (Arc::clone(gate), Arc::clone(link));
    tokio::spawn(async move { audited(&gate, link.as_ref(), applicant).await });
}

/// 审一次（模块的说明）。
async fn audited(gate: &Gate, link: &dyn Link, applicant: Applicant) {
    let to = To::Group(applicant.group);
    if !gate.active(ID, to) {
        return;
    }
    let settings = groups::read(gate.store())
        .remove(&applicant.group.to_string())
        .unwrap_or_default();
    let rule = settings.approval.rule.trim().to_string();
    if rule.is_empty()
        || !matches!(
            admin::admin_here(link, applicant.group, applicant.bot).await,
            Ok(true)
        )
    {
        return;
    }
    let Ok(_slot) = AUDITING.acquire().await else {
        return;
    };
    let comment: String = applicant.comment.trim().chars().take(COMMENT).collect();
    let request = json!({
        "purpose": "join-approval",
        "max_tokens": TOKENS,
        "messages": [
            {"role": "system", "text": gate.prompt("approval-system", &[])},
            {"role": "user", "text": gate.prompt("approval-request", &[("rule", &rule), ("request", &comment)])},
        ],
    });
    let mut decided = None;
    for _ in 0..2 {
        match tokio::time::timeout(WAIT, gate.ask(to, request.clone())).await {
            Ok(Ok(text)) => {
                decided = verdict(&text);
                if decided.is_some() {
                    break;
                }
            }
            Ok(Err(why)) => {
                tracing::warn!(target: miyu_im::TARGET, group = applicant.group, why, "join request not audited");
                break;
            }
            Err(_) => break,
        }
    }
    let (verdict, reason) = decided.unwrap_or((Verdict::Hold, String::new()));
    let done = match verdict {
        Verdict::Approve => link.join_request(&applicant.flag, true, "").await,
        Verdict::Reject => link.join_request(&applicant.flag, false, &reason).await,
        Verdict::Hold => Ok(()),
    };
    if let Err(error) = done {
        tracing::warn!(target: miyu_im::TARGET, group = applicant.group, error = ?error, "join request not handled");
        return;
    }
    tracing::info!(target: miyu_im::TARGET, group = applicant.group, user = applicant.user, verdict = ?verdict, "join request audited");
    let config = gate.config().borrow().clone();
    if admin::recording(&config) {
        let record = Record {
            at: records::now(),
            action: Action::Join,
            user: applicant.user,
            name: None,
            by: By::Ai,
            seconds: None,
            reason: clean(&reason, REASON).filter(|reason| !reason.trim().is_empty()),
            block: false,
            verdict: Some(verdict),
            // 记录一条一行：换行换成空格（问题、答案分两行写的）。
            comment: clean(&comment.replace(['\r', '\n'], " "), COMMENT),
        };
        records::append(
            gate.store().to_path_buf(),
            applicant.group,
            record,
            admin::keep(&config),
        )
        .await;
    }
}

#[cfg(test)]
mod tests;
