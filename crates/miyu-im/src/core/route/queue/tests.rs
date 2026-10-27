//! 出站队列的纯逻辑（施工 O-25 中，`onebot.md` 第一条「出站队列」第 3 到 8 条）：钟是停住的，此刻都由测试交进去。门开着的照
//! 先后交出来、关着的留着；排到了期限的作废（门关着也是），先交过期的；会话各排各的；该醒的时刻是最早的过期、禁言到期；禁言到
//! 此刻加几秒；NapCat 的回应算成功还是失败、为什么。带停顿的（施工 I-26）没到不交、后面的跟着等，该醒的时刻算上它。一个会话一次
//! 交一段，回了才交下一段（施工 I-48）。

use std::time::Duration;

use miyu_client::time::Timestamp;

use super::{DISCONNECTED, EXPIRED, Ending, Queue, REJECTED, TIMEOUT, ending, until};
use crate::chat::CallError;

/// 期限：一分钟。
const EXPIRE: Duration = Duration::from_secs(60);

/// 第 `millis` 毫秒的时刻。
fn at(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(1_760_000_000_000 + millis).expect("在范围里")
}

/// 一个排着 `items` 的队（都在第 0 毫秒入队，会话 `s`）。
fn queued(items: &[&'static str]) -> Queue<&'static str> {
    let mut queue = Queue::new(EXPIRE);
    for item in items {
        queue.push("s", (at(0), Duration::ZERO), *item);
    }
    queue
}

#[test]
fn an_open_door_hands_out_in_order_and_a_shut_one_keeps_them() {
    let mut queue = queued(&["一", "二", "三"]);
    let taken = queue.take("s", at(1000), false);
    assert!(
        taken.expired.is_empty() && taken.ready.is_empty(),
        "关着的留着"
    );
    assert_eq!(queue.lines(), ["s"]);
    let taken = queue.take("s", at(1000), true);
    assert_eq!(taken.ready, ["一"], "一次一段，照入队的先后");
    assert!(taken.expired.is_empty());
    assert!(
        queue.take("s", at(1000), true).ready.is_empty(),
        "前一段还没回，下一段等着（施工 I-48）"
    );
    for next in ["二", "三"] {
        queue.answered("s");
        assert_eq!(
            queue.take("s", at(1000), true).ready,
            [next],
            "回了才交下一段"
        );
    }
    queue.answered("s");
    assert!(queue.lines().is_empty(), "交完了不再排");
}

#[test]
fn a_piece_out_still_lets_the_ones_behind_expire() {
    // 施工 I-48：交出去的那一段还没回，排着的过了期照样作废，不等它回。
    let mut queue = Queue::new(EXPIRE);
    queue.push("s", (at(0), Duration::ZERO), "一");
    queue.push("s", (at(0), Duration::ZERO), "二");
    assert_eq!(queue.take("s", at(0), true).ready, ["一"]);
    let taken = queue.take("s", at(60_000), true);
    assert_eq!((taken.expired, taken.ready), (vec!["二"], vec![]));
}

#[test]
fn waiting_until_the_deadline_expires_even_behind_a_shut_door() {
    let mut queue = Queue::new(EXPIRE);
    queue.push("s", (at(0), Duration::ZERO), "一");
    queue.push("s", (at(30_000), Duration::ZERO), "二");
    let taken = queue.take("s", at(59_999), false);
    assert!(taken.expired.is_empty(), "差一毫秒不算");
    let taken = queue.take("s", at(60_000), false);
    assert_eq!(taken.expired, ["一"], "入队时刻加期限不晚于此刻就算过了");
    assert!(taken.ready.is_empty());
    let taken = queue.take("s", at(89_999), true);
    assert_eq!((taken.expired, taken.ready), (vec![], vec!["二"]));
}

#[test]
fn the_expired_come_out_before_the_ready() {
    let mut queue = Queue::new(EXPIRE);
    queue.push("s", (at(0), Duration::ZERO), "旧");
    queue.push("s", (at(50_000), Duration::ZERO), "新");
    let taken = queue.take("s", at(61_000), true);
    assert_eq!(taken.expired, ["旧"]);
    assert_eq!(taken.ready, ["新"], "过期的不交，没过期的照交");
}

#[test]
fn sessions_wait_apart() {
    let mut queue = Queue::new(EXPIRE);
    queue.push("甲", (at(0), Duration::ZERO), "一");
    queue.push("乙", (at(0), Duration::ZERO), "二");
    queue.push("甲", (at(1), Duration::ZERO), "三");
    assert_eq!(queue.lines(), ["乙", "甲"]);
    let taken = queue.take("甲", at(10), true);
    assert_eq!(taken.ready, ["一"]);
    assert_eq!(queue.lines(), ["乙", "甲"], "别的场所的不动");
    assert_eq!(
        queue.take("乙", at(10), true).ready,
        ["二"],
        "甲的那一段还没回，不挡乙的"
    );
    queue.answered("甲");
    assert_eq!(queue.take("甲", at(10), true).ready, ["三"]);
    assert!(queue.lines().is_empty(), "交完了不再排");
    let taken = queue.take("丙", at(10), true);
    assert!(
        taken.ready.is_empty() && taken.expired.is_empty(),
        "没排的什么都没有"
    );
}

#[test]
fn it_wakes_at_the_first_deadline_or_the_end_of_a_mute() {
    let empty: Queue<&str> = Queue::new(EXPIRE);
    assert_eq!(empty.wake(at(0), |_| Some(at(5))), None, "没排着的不醒");
    let mut queue = Queue::new(EXPIRE);
    queue.push("甲", (at(10_000), Duration::ZERO), "一");
    queue.push("甲", (at(20_000), Duration::ZERO), "二");
    queue.push("乙", (at(15_000), Duration::ZERO), "三");
    assert_eq!(
        queue.wake(at(20_000), |_| None),
        Some(at(70_000)),
        "最早的过期"
    );
    let muted = |session: &str| (session == "乙").then(|| at(30_000));
    assert_eq!(
        queue.wake(at(20_000), muted),
        Some(at(30_000)),
        "禁言先到期"
    );
    let later = |_: &str| Some(at(90_000));
    assert_eq!(queue.wake(at(20_000), later), Some(at(70_000)), "过期先到");
}

#[test]
fn a_paused_piece_waits_and_holds_the_ones_behind() {
    let mut queue = Queue::new(EXPIRE);
    let pause = Duration::from_secs(5);
    queue.push("s", (at(0), Duration::ZERO), "一");
    queue.push("s", (at(0), pause), "二");
    queue.push("s", (at(0), Duration::ZERO), "三");
    assert_eq!(
        queue.take("s", at(1000), true).ready,
        ["一"],
        "第二段没到点"
    );
    assert_eq!(
        queue.wake(at(1000), |_| None),
        Some(at(5000)),
        "醒在第二段到点"
    );
    queue.answered("s");
    assert!(
        queue.take("s", at(4999), true).ready.is_empty(),
        "第三段不抢先"
    );
    assert_eq!(queue.take("s", at(5000), true).ready, ["二"]);
    queue.answered("s");
    assert_eq!(queue.take("s", at(5000), true).ready, ["三"]);
    // 停顿从前一段最早交的时刻接着算：前一段第 5 秒交，后一段再停 5 秒，在第 10 秒。
    queue.push("t", (at(0), Duration::ZERO), "零");
    assert_eq!(queue.take("t", at(0), true).ready, ["零"]);
    queue.answered("t");
    queue.push("t", (at(0), pause), "甲");
    queue.push("t", (at(1000), pause), "乙");
    assert!(queue.take("t", at(4999), true).ready.is_empty());
    assert_eq!(queue.take("t", at(5000), true).ready, ["甲"]);
    queue.answered("t");
    assert!(queue.take("t", at(9999), true).ready.is_empty());
    assert_eq!(queue.take("t", at(10_000), true).ready, ["乙"]);
}

#[test]
fn a_new_reply_counts_its_pause_from_the_last_piece_handed_out() {
    // 施工 I-58（10-27 项目主人：并着回的几句还是秒发）：排空了以后新的一段从这个场所上一次交出去的时刻算停顿。
    let mut queue = Queue::new(EXPIRE);
    let typed = Duration::from_secs(3);
    queue.push("群", (at(0), typed), "没交过的不等");
    assert_eq!(queue.take("群", at(0), true).ready, ["没交过的不等"]);
    queue.answered("群");
    // 刚交过（第 0 秒）：另一句在第 1 秒到，等到第 3 秒。
    queue.push("群", (at(1000), typed), "刚交过的等");
    assert!(queue.take("群", at(2999), true).ready.is_empty());
    assert_eq!(queue.take("群", at(3000), true).ready, ["刚交过的等"]);
    queue.answered("群");
    // 早就交了（第 3 秒）：第 60 秒到的不用等。
    queue.push("群", (at(60_000), typed), "早就交了的不等");
    assert_eq!(queue.take("群", at(60_000), true).ready, ["早就交了的不等"]);
    // 别的场所各算各的。
    queue.push("私聊", (at(60_000), typed), "别处");
    assert_eq!(queue.take("私聊", at(60_000), true).ready, ["别处"]);
}

#[test]
fn a_shut_door_does_not_wake_for_pieces_already_due() {
    let mut queue = Queue::new(EXPIRE);
    queue.push("s", (at(0), Duration::from_secs(5)), "一");
    assert_eq!(
        queue.wake(at(6000), |_| None),
        Some(at(60_000)),
        "到了点的不算，只等过期"
    );
    let taken = queue.take("s", at(6000), false);
    assert!(taken.ready.is_empty() && taken.expired.is_empty());
}

#[test]
fn a_mute_lasts_from_now() {
    assert_eq!(until(at(0), 600), Some(at(600_000)));
    assert_eq!(until(at(0), 0), Some(at(0)));
    assert_eq!(until(at(0), u64::MAX), None, "算出来超出范围的没有");
}

/// 平台回了没成，它自己那句话是 `message`（驱动去掉了首尾空白，`onebot::failed`）。
fn failed(message: &str) -> Result<Option<i64>, CallError> {
    Err(CallError::Failed {
        said: "said".to_string(),
        message: message.to_string(),
    })
}

#[test]
fn a_reply_ends_as_sent_or_failed_and_why() {
    assert_eq!(ending(&Ok(Some(7))), Ending::Sent(Some(7)));
    assert_eq!(ending(&Ok(None)), Ending::Sent(None));
    assert_eq!(
        ending(&failed("风控拦了")),
        Ending::Failed(REJECTED, Some("风控拦了".to_string())),
        "照它自己那句话，不照 said"
    );
    let long = "失".repeat(250);
    assert_eq!(
        ending(&failed(&long)),
        Ending::Failed(REJECTED, Some("失".repeat(200))),
        "截到 200 个字符"
    );
    assert_eq!(ending(&failed("")), Ending::Failed(REJECTED, None));
    assert_eq!(
        ending(&Err(CallError::Timeout)),
        Ending::Failed(TIMEOUT, None)
    );
    assert_eq!(
        ending(&Err(CallError::Closed)),
        Ending::Failed(DISCONNECTED, None)
    );
    assert_eq!(
        (REJECTED, TIMEOUT, DISCONNECTED, EXPIRED),
        ("rejected", "timeout", "disconnected", "expired"),
        "写进 `failed` 的 `why`"
    );
}
