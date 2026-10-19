//! 出站挂接点（施工 O-43 下，`onebot.md` 第一条「挂接点」第 4 条）：她的一段话（群里、私聊）过出站链以前，问挂在
//! `chat.outbound` 的包，换了的照换的、说不发的不发。问的时候跟核心的那一头不等：另起任务问，排进一条照先后交回的队（[`Route`]
//! 的 `voicing`），交回来的照先后过出站链、入队（[`Route::voiced`]），她的几段话不会乱了先后。出站链要的情形（这一回合发过的、
//! 去重）在交回来以后才算，排着的前一段入队了，后一段照它去重。没挂的、队里也没有排着的当场过链，和没有挂接点时一样。

use std::sync::Arc;

use super::Route;
use super::projection::Speaking;
use crate::TARGET;
use crate::core::Gone;
use crate::core::hooks::{OUTBOUND, Voiced};

/// 她的这一段话是哪一种：过出站链要的情形照它算。
pub(super) enum Voice {
    /// 群里（主线或者支线）：投影交出的这一段的情形。
    Group(Speaking),
    /// 私聊：第几个回合（去重照这一回合发过的）。
    Private {
        /// 回合编号。
        turn: u64,
    },
}

/// 问过出站挂接点、等着照先后过链的一段。
pub(super) struct Voicing {
    /// 会话。
    session: String,
    /// 哪一种。
    voice: Voice,
    /// 挂接点的结果。
    voiced: Voiced,
}

impl Route {
    /// 她在会话 `session` 新说的一段话 `text`（`voice` 是哪一种）：挂了出站挂接点的另起任务问，排进队里；没挂、队里也没有排着
    /// 的当场过链。不知道发到哪的也当场办（记一行、不发）。挂的没了、队里还排着的，也排进去（照原样），免得抢到前面。
    ///
    /// # Errors
    ///
    /// 当场入队时写不出去、核心断开。
    pub(super) async fn voice(
        &mut self,
        session: &str,
        voice: Voice,
        text: String,
    ) -> Result<(), Gone> {
        let to = self.peers.get(session).map(|peer| peer.to);
        let packages = self.hooks.at(OUTBOUND, to);
        let peer = self.peers.get(session).cloned();
        let Some(peer) = peer.filter(|_| !packages.is_empty() || !self.voicing.is_empty()) else {
            return self.voiced_now(session, voice, &text).await;
        };
        let hooks = Arc::clone(&self.hooks);
        let session = session.to_string();
        self.voicing.push_back(tokio::spawn(async move {
            let voiced = match packages.is_empty() {
                true => Voiced::Say(text),
                false => {
                    hooks
                        .outbound(packages, peer.venue.as_str(), peer.bot, text)
                        .await
                }
            };
            Voicing {
                session,
                voice,
                voiced,
            }
        }));
        Ok(())
    }

    /// 队里交回来的一段：说不发的记一行 `reply dropped`（`why=hooked`、哪个包，不记原文，不退信：是别的包的意思），别的照它过链、
    /// 入队。
    ///
    /// # Errors
    ///
    /// 入队时写不出去、核心断开。
    pub(super) async fn voiced(&mut self, voicing: Voicing) -> Result<(), Gone> {
        let Voicing {
            session,
            voice,
            voiced,
        } = voicing;
        match voiced {
            Voiced::Say(text) => self.voiced_now(&session, voice, &text).await,
            Voiced::Drop(package) => {
                if let Some(peer) = self.peers.get(&session) {
                    tracing::info!(target: TARGET, venue = %peer.venue, why = "hooked", package, "reply dropped");
                }
                Ok(())
            }
        }
    }

    /// 照 `voice` 过出站链、入队（群里的 `speak_now`，私聊的 `say_privately_now`）。
    async fn voiced_now(&mut self, session: &str, voice: Voice, text: &str) -> Result<(), Gone> {
        match voice {
            Voice::Group(speaking) => self.speak_now(session, speaking, text).await,
            Voice::Private { turn } => self.say_privately_now(session, turn, text).await,
        }
    }
}
