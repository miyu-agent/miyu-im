//! 名额满了的连接回 `503`（施工 YC-10，压测报告 O24）：原来当场关掉、什么都不回，NapCat 那头只看到「没收到有效回应」，看不出是
//! 被限流。现在不读请求、直接写一段写死的 `503`，关掉写的那一头，再把对方发来的读掉、等它关（不读就关，对方收到的是连接重置，
//! 回的 `503` 也跟着丢，macOS 上尤其）。
//!
//! 回 `503` 的连接也有上限（和还没升级的连接的总数一样多），每条最多留 `handshake_seconds`：再多的照原来当场关掉，连得到端口的人
//! 开不出没有上限的任务。

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// 回的那一段：没有内容，回完就关，过一秒再连。
const ANSWER: &[u8] =
    b"HTTP/1.1 503 Service Unavailable\r\nRetry-After: 1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// 回 `503` 的名额。
pub(super) struct Refusing {
    slots: Arc<Semaphore>,
    within: Duration,
}

impl Refusing {
    /// 同时最多回 `most` 条，每条最多留 `within`。
    pub(super) fn new(most: usize, within: Duration) -> Refusing {
        Refusing {
            slots: Arc::new(Semaphore::new(most)),
            within,
        }
    }

    /// 给 `stream` 回 `503`（任务挂进 `tasks`）；回的名额也满了的当场关掉。交回回没回。
    pub(super) fn refuse(&self, stream: TcpStream, tasks: &mut JoinSet<()>) -> bool {
        let Ok(slot) = Arc::clone(&self.slots).try_acquire_owned() else {
            return false;
        };
        let within = self.within;
        tasks.spawn(async move {
            // 到时没关的放下就关了；写不进、读出错的也是放下。
            drop(tokio::time::timeout(within, answer(stream)).await);
            drop(slot);
        });
        true
    }
}

/// 写 `503`、关掉写的那一头，读到对方关为止。
async fn answer(mut stream: TcpStream) {
    if stream.write_all(ANSWER).await.is_err() || stream.shutdown().await.is_err() {
        return;
    }
    let mut rest = [0u8; 1024];
    while matches!(stream.read(&mut rest).await, Ok(read) if read > 0) {}
}
