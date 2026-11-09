//! 一个 Telegram 机器人的连接（宿主的 `Link`，施工 I-183 中上）：经 Bot API 调用办宿主要做的事。这一步只有骨架：认得出是哪个
//! 机器人、叫什么；发消息随 I-183 中下，别的照 `Link` 的默认答「做不到」（宿主照能力表先查）。

use std::sync::atomic::{AtomicBool, Ordering};

use miyu_im::chat::{CallError, Implementation, Link, Outgoing, Pending, Reply, To};

/// 一个机器人的连接。
#[derive(Debug)]
pub struct Bot {
    /// 机器人的名字：`getMe` 的 `first_name`（加 `last_name`）。
    pub(crate) name: String,
    /// 关掉了（配置换了、桥在停）：再调的都算断了。
    closed: AtomicBool,
}

impl Bot {
    /// 照 `getMe` 回的名字造一个（调用口随 I-183 中下发消息时带上）。
    pub(crate) fn new(name: String) -> Bot {
        Bot {
            name,
            closed: AtomicBool::new(false),
        }
    }
}

impl Link for Bot {
    fn begin<'a>(&'a self, to: To, outgoing: Outgoing<'a>) -> Reply<'a, Pending> {
        let _ = (to, outgoing);
        let closed = self.closed.load(Ordering::Relaxed);
        // 发消息随 I-183 中下：这一步还不发。
        Box::pin(async move {
            Err(if closed {
                CallError::Closed
            } else {
                CallError::Unsupported
            })
        })
    }

    fn nickname(&self) -> Reply<'_, Option<String>> {
        let name = (!self.name.is_empty()).then(|| self.name.clone());
        Box::pin(async move { Ok(name) })
    }

    fn implementation(&self) -> Option<Implementation> {
        // 对端就是 Telegram，没有「哪个实现」这回事（页面照用户名画，不照实现）。
        None
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}
