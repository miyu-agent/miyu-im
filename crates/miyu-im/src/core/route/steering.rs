//! 进站挂接点（施工 O-43 中，`onebot.md` 第一条「挂接点」第 3 条）：群里一条人说的话记下以后、判以前，问挂在 `chat.inbound`
//! 的包。问的时候跟核心的那一头不等：另起任务问，排进一条照先后交回的队（[`Route`] 的 `steering`），交回来的照先后判
//! （`called`），一条慢的不会让后面的抢到前面判。没挂的、队里也没有排着的当场判，和没有挂接点时一样。

use std::sync::Arc;

use serde_json::Value;

use super::Route;
use super::called::Heard;
use crate::chat::Platform;
use crate::core::Gone;
use crate::core::hooks::{INBOUND, Steered};

/// 问过进站挂接点、等着照先后判的一条。
pub(super) struct Steering {
    /// 群会话。
    session: String,
    /// 核心记下它的序号。
    seq: u64,
    /// 判它要的。
    heard: Heard,
    /// 挂接点的意见。
    steered: Steered,
    /// 记下的那一刻它已经在投影里（重发的，`Route::known`）。
    known: bool,
}

impl Route {
    /// 平台 `platform` 上一条记下了的群消息（会话 `session`、序号 `seq`、判它要的 `heard`）：挂了进站挂接点的另起任务问（问的是 `ask`），排进
    /// 队里；没挂、队里也没有排着的当场判。挂的没了、队里还排着的，也排进去（照常），免得抢到前面。
    ///
    /// # Errors
    ///
    /// 当场判时写不出去、核心断开（同 `called`）。
    pub(super) async fn steer(
        &mut self,
        platform: Platform,
        (session, seq): (String, u64),
        heard: Heard,
        ask: Value,
    ) -> Result<(), Gone> {
        // 重发的照记下这一刻看：问挂接点的这段时间投影会补上它。
        let known = self.known(&session, seq);
        let to = self.peers.get(&session).map(|peer| peer.to);
        let packages = self.hooks.at(INBOUND, platform, to);
        if packages.is_empty() && self.steering.is_empty() {
            return self
                .called(&session, seq, heard, (Steered::pass(), known))
                .await;
        }
        let hooks = Arc::clone(&self.hooks);
        self.steering.push_back(tokio::spawn(async move {
            let steered = match packages.is_empty() {
                true => Steered::pass(),
                false => hooks.inbound(platform, packages, ask).await,
            };
            Steering {
                session,
                seq,
                heard,
                steered,
                known,
            }
        }));
        Ok(())
    }

    /// 队里交回来的一条：照它的意见判（`called`）。
    ///
    /// # Errors
    ///
    /// 写不出去、核心断开（同 `called`）。
    pub(super) async fn steered(&mut self, steering: Steering) -> Result<(), Gone> {
        let Steering {
            session,
            seq,
            heard,
            steered,
            known,
        } = steering;
        self.called(&session, seq, heard, (steered, known)).await
    }
}
