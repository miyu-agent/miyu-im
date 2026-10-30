//! 一条连接读到的事件交给跟核心的那一头（施工 I-95，发布前审查 I2-2）：读的循环不等。
//!
//! 原来读的循环把事件直接塞进跟核心的那一头的队（几条连接共用，`inbound_queue`），队满了读的循环就停在那里：NapCat 回的话也读
//! 不到，桥调的动作一律等到超时，一个群刷屏拖住所有号。现在每条连接一个转交任务、一个缓冲（`connection_events`）：读到的事件
//! 放进缓冲就接着读，转交任务照先后交过去；缓冲满了的丢掉、记一行 `WARN`。号认出来那一件（`Event::Connected`）一定交到。

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

use miyu_im::TARGET;
use miyu_im::chat::Event;
use miyu_im::driver::Gate;

/// 一条连接的转交。
pub(super) struct Handing {
    events: mpsc::Sender<Event>,
}

impl Handing {
    /// 起这条连接的转交任务：照先后交给跟核心的那一头，那一头不收了（桥在停）就停。
    pub(super) fn start(gate: &Arc<Gate>) -> Handing {
        let (events, mut waiting) = mpsc::channel(gate.tuning().connection_events);
        let gate = Arc::clone(gate);
        tokio::spawn(async move {
            while let Some(event) = waiting.recv().await {
                if !gate.inbound(event).await {
                    break;
                }
            }
        });
        Handing { events }
    }

    /// 交一件，不等：缓冲满了的丢掉、记一行。转交任务停了（桥在停）交回假。
    pub(super) fn hand(&self, event: Event) -> bool {
        match self.events.try_send(event) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                tracing::warn!(target: TARGET, "event dropped, im busy");
                true
            }
            Err(TrySendError::Closed(_)) => false,
        }
    }

    /// 交一件，一定交到（号认出来那一件）：缓冲满了等。转交任务停了交回假。
    pub(super) async fn hand_surely(&self, event: Event) -> bool {
        self.events.send(event).await.is_ok()
    }
}
