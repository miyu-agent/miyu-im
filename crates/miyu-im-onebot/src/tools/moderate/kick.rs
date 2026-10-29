//! 踢一个人（施工 I-17，照旧版 `group_management` 的 `kick_one`、`validate_target`）：
//!
//! - 先现问他（`no_cache`）：不在群里的不踢（`not-members`），群主、管理员不踢（`protected`）。
//! - 踢。NapCat 踢成了也可能回 `status=failed, retcode=100`（旧版 08-30 实测；等不到的也一样不确定）：这时照事实核实人走没走——
//!   她刚踢走的通知（`admin::sync::removed_lately`，踢成了一两秒内必到）、现问成员；最多 [`TRIES`] 次、隔 [`APART`]（成员表最终
//!   一致，SnowLuma 的缓存实测能滞后五秒以上）。走了的算成，记一行运行日志。

use std::time::Duration;

use miyu_im::chat::{CallError, Id, Link};

use super::super::acting::{Undone, undone};
use crate::admin::{ranked, sync};
use crate::onebot::fresh_member;

/// 核实最多几次。
const TRIES: u32 = 6;

/// 两次核实隔多久。
const APART: Duration = Duration::from_millis(900);

/// 把群 `group` 里的 `user` 踢出去，`block` 的以后不收他的加群申请（模块的说明）。
pub(super) async fn kick(link: &dyn Link, group: Id, user: Id, block: bool) -> Result<(), Undone> {
    match fresh_member(link, &group, &user).await {
        Ok(Some(member)) if ranked(member.rank) => return Err(Undone("protected", None)),
        Ok(Some(_)) => {}
        // NapCat 对不在群里的人回失败。
        Ok(None) | Err(CallError::Failed { .. }) => return Err(Undone("not-members", None)),
        Err(error) => return Err(undone(error)),
    }
    let error = match link.kick(&group, &user, block).await {
        Ok(()) => return Ok(()),
        Err(error @ (CallError::Failed { .. } | CallError::Timeout)) => error,
        Err(error) => return Err(undone(error)),
    };
    if gone(link, group.clone(), user.clone()).await {
        tracing::warn!(target: miyu_im::TARGET, group = %group, user = %user, error = ?error, "kick reported a failure but the member is gone; treated as done");
        return Ok(());
    }
    Err(undone(error))
}

/// 群 `group` 里的 `user` 走了没有（模块的说明）：她刚踢走的通知到了、现问回没有这个人的算走了。
async fn gone(link: &dyn Link, group: Id, user: Id) -> bool {
    for attempt in 0..TRIES {
        if attempt > 0 {
            tokio::time::sleep(APART).await;
        }
        if sync::removed_lately(group.clone(), user.clone()) {
            return true;
        }
        if matches!(
            fresh_member(link, &group, &user).await,
            Ok(None) | Err(CallError::Failed { .. })
        ) {
            return true;
        }
    }
    false
}
