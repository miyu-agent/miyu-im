//! 一阵活动过去、安静下来以后把堆里空着的还给系统（施工 LW-IM-1，`onebot.md` 第一条「在哪」`settle.rs` 一行；照核心的 `settle.rs`）。
//!
//! glibc 放掉的块多半留在自己手里，尤其是各线程的分配区：灌进一阵群消息、判过、写过库以后，放掉的留着不还，桥一直多占着
//! （LW-IM-1 量过：5 个群灌 5000 条以后线程分配区驻留 21 MB，活着的只有 5.6 MB）。原来只在起来补完日志以后还一次。
//!
//! 所以跟核心的管道上每读到一行（核心的回应、推来的事件、来问的请求；平台来的消息、页面的操作都会引出这几样）叫一声
//! （[`mark`]）；叫过以后 `quiet`（`bridge.json` 的 `settle_millis`）这么久再没有动静，在阻塞线程上调一次 `release`（还堆，
//! LW-IM-3 起先放掉记录库的页缓存）。没有动静的时候这里一直睡着，不定时醒。
//!
//! 叫的一头是整个进程共用的一份：堆本来就是整个进程的；测试里一个进程起几个桥，每个桥的这一份都照它醒，多还几次不碍事。

use std::sync::OnceLock;
use std::time::Duration;

use tokio::sync::watch;

use crate::TARGET;
use crate::serve::Failure;

/// 整个进程共用的那一声。
fn activity() -> &'static watch::Sender<()> {
    static ACTIVITY: OnceLock<watch::Sender<()>> = OnceLock::new();
    ACTIVITY.get_or_init(|| watch::channel(()).0)
}

/// 有动静了：叫一声，不等。没有人在盯的时候什么都不做。
pub(crate) fn mark() {
    activity().send_replace(());
}

/// 盯着：每次安静下来调一次 `release`（在阻塞线程上），再接着等下一阵。一直盯下去，桥停下时随一组任务一起停；交回的类型照
/// 那一组（`crate::serve`）。要在 tokio 的运行时里调。
pub(crate) async fn keep(
    quiet: Duration,
    release: impl Fn() -> bool + Clone + Send + 'static,
) -> Option<Failure> {
    watching(activity().subscribe(), quiet, release).await
}

/// [`keep`] 盯的是 `changes`（测试里给自己的一份，不和别的测试叫的混在一起）。
async fn watching(
    mut changes: watch::Receiver<()>,
    quiet: Duration,
    release: impl Fn() -> bool + Clone + Send + 'static,
) -> Option<Failure> {
    loop {
        if changes.changed().await.is_err() {
            return None;
        }
        while let Ok(Ok(())) = tokio::time::timeout(quiet, changes.changed()).await {}
        if !settled(tokio::task::spawn_blocking(release.clone()).await) {
            return None;
        }
    }
}

/// 还了一次以后怎么收，交回要不要接着盯（同核心）：还了的记一行 `DEBUG`；崩了的记 `ERROR heap settle panicked`，接着盯；
/// 被取消的是运行时在收尾（停下时阻塞任务排不上），记一行 `DEBUG`，不再盯。
fn settled(result: Result<bool, tokio::task::JoinError>) -> bool {
    match result {
        Ok(released) => {
            tracing::debug!(target: TARGET, released, "heap settled");
            true
        }
        Err(error) if error.is_panic() => {
            tracing::error!(target: TARGET, panic = true, "heap settle panicked");
            true
        }
        Err(_) => {
            tracing::debug!(target: TARGET, "heap settle cancelled");
            false
        }
    }
}

#[cfg(test)]
mod tests;
