//! 引用的那一条丢出投影的窗口以后认发的人（施工 LW-IM-7，`projection/window.rs`、`records/quoted.rs`，`onebot.md` 第一条「群里怎么
//! 叫她」第 1 条）：叫她做的那一条（`origin`）引用了一条，投影里认不出是谁发的，这个群的投影又丢过人话的，照平台编号去聊天记录库
//! 里找；没丢过的认不出就是不知道（和原来一样），不查。查不到的（库被清理过、没记上、打不开）照「不知道是谁」。
//!
//! 时刻：投影记的是核心记下这一条的时刻，库里记的是平台给的时刻，两边差几秒；范围前后各放宽 [`SLACK`]。

use std::path::PathBuf;

use miyu_client::ids::ExternalId;

use super::Route;
use crate::TARGET;
use crate::toolset::Origin;

/// 查记录库时时间范围前后各放宽多久，毫秒：平台的时刻和核心记下的时刻差不到它（导进来的更早的记录差得远，照样排除在外）。
const SLACK: i64 = 10 * 60 * 1000;

/// 要去库里查的一次：库、场所、引用的平台编号、时间范围（毫秒）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Lookup {
    /// 库文件。
    path: PathBuf,
    /// 场所编号。
    venue: String,
    /// 引用的那一条的平台编号。
    msg: String,
    /// 时刻的范围。
    range: (i64, i64),
}

impl Route {
    /// 群会话 `session`（场所 `venue`）里叫她做的那一条 `origin` 要不要去库里查引用的发的人：要的交回怎么查。
    pub(super) fn quote_lookup(
        &self,
        session: &str,
        venue: &str,
        origin: Option<&Origin>,
    ) -> Option<Lookup> {
        let quote = origin?
            .quote
            .as_ref()
            .filter(|quote| quote.sender.is_none())?;
        let (first, until) = self.groups.get(session)?.pruned()?;
        Some(Lookup {
            path: self.records.path().to_path_buf(),
            venue: venue.to_string(),
            msg: quote.msg.clone(),
            range: (
                first.unix_millis().saturating_sub(SLACK),
                until.unix_millis().saturating_add(SLACK),
            ),
        })
    }
}

impl Lookup {
    /// 去库里查（在阻塞线程上，只读地开）；查不到、出错的是空的。
    pub(super) async fn sender(self) -> Option<ExternalId> {
        let found = tokio::task::spawn_blocking(move || {
            let connection = rusqlite::Connection::open_with_flags(
                &self.path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            crate::records::quoted::sender_of(&connection, &self.venue, &self.msg, self.range)
        })
        .await;
        match found {
            Ok(Ok(Some(sender))) => ExternalId::parse(&sender).ok(),
            Ok(Ok(None)) => None,
            Ok(Err(error)) => {
                tracing::debug!(target: TARGET, error = %error, "quoted sender not looked up");
                None
            }
            Err(error) => {
                tracing::debug!(target: TARGET, error = %error, "quoted sender lookup ended");
                None
            }
        }
    }
}

/// 把查到的 `sender` 填进 `origin` 的引用。
pub(super) fn with_sender(
    mut origin: Option<Origin>,
    sender: Option<ExternalId>,
) -> Option<Origin> {
    if let Some(quote) = origin.as_mut().and_then(|origin| origin.quote.as_mut()) {
        quote.sender = sender;
    }
    origin
}

#[cfg(test)]
mod tests;
