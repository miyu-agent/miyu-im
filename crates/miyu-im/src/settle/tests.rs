//! 安静下来还堆（施工 LW-IM-1）：叫过以后安静够了还一次；一阵里叫几次只还一次；不叫不还；一阵接一阵各还一次。钟停住了跑。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::watch;

use super::watching;

const QUIET: Duration = Duration::from_secs(2);

/// 起一份盯着自己那一声的，交回叫的一头和还了几次。
fn started() -> (watch::Sender<()>, Arc<AtomicUsize>) {
    let (sender, changes) = watch::channel(());
    let count = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&count);
    tokio::spawn(watching(changes, QUIET, move || {
        counted.fetch_add(1, Ordering::SeqCst);
        true
    }));
    (sender, count)
}

/// 让钟走 `by`，中间让别的任务（含阻塞线程上那一次）跑完。
async fn pass(by: Duration) {
    tokio::time::sleep(by).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    // 阻塞线程上的那一次不受停住的钟管，等它真跑完。
    tokio::time::sleep(Duration::from_millis(1)).await;
}

async fn released(count: &AtomicUsize, wanted: usize) -> usize {
    for _ in 0..200 {
        if count.load(Ordering::SeqCst) >= wanted {
            break;
        }
        pass(Duration::from_millis(1)).await;
    }
    count.load(Ordering::SeqCst)
}

#[tokio::test(start_paused = true)]
async fn one_burst_is_released_once_after_it_settles() {
    let (sender, count) = started();
    pass(Duration::from_millis(10)).await;
    for _ in 0..5 {
        sender.send_replace(());
        pass(Duration::from_millis(500)).await;
    }
    assert_eq!(count.load(Ordering::SeqCst), 0, "还在动，不还");
    pass(QUIET).await;
    assert_eq!(released(&count, 1).await, 1, "安静够了还一次");
    pass(QUIET * 10).await;
    assert_eq!(count.load(Ordering::SeqCst), 1, "没动静不再还");
}

#[tokio::test(start_paused = true)]
async fn nothing_happening_releases_nothing() {
    let (_sender, count) = started();
    pass(QUIET * 30).await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn each_burst_is_released_once() {
    let (sender, count) = started();
    pass(Duration::from_millis(10)).await;
    sender.send_replace(());
    pass(QUIET * 2).await;
    assert_eq!(released(&count, 1).await, 1);
    sender.send_replace(());
    pass(QUIET * 2).await;
    assert_eq!(released(&count, 2).await, 2);
}
