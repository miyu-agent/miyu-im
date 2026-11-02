//! 动手之前先现问对方（施工 I-97，项目主人 11-02：QQ 照 OneBot 收得下禁言管理员的请求，界面上显示禁了，实际没成；管理员动不了管理员）：
//! 不在群里的不做（`not-members`）；群主谁都动不了，管理员只有她是群主时动得了（`protected`）。禁言、解禁、踢人都照它。

use miyu_im::chat::{CallError, Id, Link, Rank};

use super::super::acting::{Undone, undone};
use crate::onebot::fresh_member;

/// 对方身份是 `rank`，她是群主没有（`owner`）：动不动得了（纯逻辑）。
pub(super) fn shielded(rank: Option<Rank>, owner: bool) -> bool {
    match rank {
        Some(Rank::Owner) => true,
        Some(Rank::Admin) => !owner,
        _ => false,
    }
}

/// 群 `group` 里的 `user` 能不能动（模块的说明），`owner` 是她在这个群是群主。问不到的照问不到答。
pub(super) async fn reachable(
    link: &dyn Link,
    (group, user): (&Id, &Id),
    owner: bool,
) -> Result<(), Undone> {
    match fresh_member(link, group, user).await {
        Ok(Some(member)) if shielded(member.rank, owner) => Err(Undone("protected", None)),
        Ok(Some(_)) => Ok(()),
        // NapCat 对不在群里的人回失败。
        Ok(None) | Err(CallError::Failed { .. }) => Err(Undone("not-members", None)),
        Err(error) => Err(undone(error)),
    }
}

#[cfg(test)]
mod tests;
