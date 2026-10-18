//! 群里的命令回执撤回（施工 O-25 上，`onebot.md` 第一条「斜杠命令」第 7 条）：NapCat 回了编号以后，等够给的时候才撤，撤的是
//! 那个编号，经那时的连接；那时没连着的不撤（没回编号的不撤由 `sending.rs` 管，O-25 中）。钟是停住的（`start_paused`），等多久
//! 照它算，不照真的时间。

use std::sync::Arc;
use std::time::Duration;

use miyu_client::ids::VenueId;
use tokio::sync::mpsc;

use super::super::Peer;
use super::recall;
use crate::chat::To;
use crate::chat::fake::{Asked, Did, Fake, fake, next};
use crate::chat::{Bots, Link};

/// 机器人的号。
const BOT: i64 = 30003;

/// 测试里给的：等 3 秒再撤。
const AFTER: Duration = Duration::from_secs(3);

/// 一条连着的机器人号：交回号到连接、假连接、它收到的要做的事。
fn connected() -> (Arc<Bots>, Arc<Fake>, mpsc::Receiver<Asked>) {
    let (link, asked) = fake();
    let bots = Arc::new(Bots::default());
    assert!(bots.insert(BOT, link.clone()).is_none());
    (bots, link, asked)
}

/// 发到群 5 的那个场所。
fn peer() -> Peer {
    Peer {
        bot: BOT,
        to: To::Group(5),
        venue: VenueId::parse("qq:group:5").expect("合写法"),
    }
}

#[tokio::test(start_paused = true)]
async fn the_receipt_is_recalled_only_after_the_wait() {
    let (bots, _link, mut asked) = connected();
    let task = tokio::spawn(recall(77, peer(), bots, AFTER));
    tokio::time::sleep(AFTER - Duration::from_millis(1)).await;
    assert!(asked.try_recv().is_err(), "还没到时候");
    let deleted = next(&mut asked).await;
    assert_eq!(
        deleted.did,
        Did::Recall("Number(77)".to_string()),
        "撤的是回的那个编号，照整数"
    );
    deleted.answer(true);
    task.await.expect("没崩");
}

#[tokio::test(start_paused = true)]
async fn without_a_connection_then_nothing_is_recalled() {
    let (bots, link, mut asked) = connected();
    let task = tokio::spawn(recall(78, peer(), Arc::clone(&bots), AFTER));
    let link: Arc<dyn Link> = link;
    bots.remove(BOT, &link);
    task.await.expect("没崩");
    assert!(asked.try_recv().is_err(), "那时没连着的不撤");
}
