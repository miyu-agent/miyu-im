//! 桥手里的状态变了（施工 I-6 上从 `Arc<Notify>` 换来）：NapCat 连上、断开、认出号、问到实现，推来的换了端口、配置，页面服务
//! 换了端口，都叫一声。状态文件（`crate::status_file`）照它重写，im 的页面（`crate::site`）照它推给开着的通道。一个计数，
//! 叫一声加一；拿着的照 `watch` 等它变（叫了几声都只醒一次，醒了照这一刻的样子说）。

use tokio::sync::watch;

/// 变了的计数。
pub(crate) struct Changed {
    revision: watch::Sender<u64>,
}

impl Changed {
    /// 还没变过。
    pub(crate) fn new() -> Changed {
        let (revision, _) = watch::channel(0);
        Changed { revision }
    }

    /// 叫一声：拿着的都醒。
    pub(crate) fn notify(&self) {
        self.revision
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }

    /// 等它变：从这一刻起的叫声都醒得到。
    pub(crate) fn watch(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }
}
