//! 出站队列的纯逻辑（施工 O-25 中，`onebot.md` 第一条「出站队列」第 3 到 8 条）：钟是停住的，此刻都由测试交进去。门开着的照
//! 先后交出来、关着的留着；排到了期限的作废（门关着也是），先交过期的；会话各排各的；该醒的时刻是最早的过期、禁言到期；禁言到
//! 此刻加几秒；NapCat 的回应算成功还是失败、为什么。

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
        queue.push("s", at(0), *item);
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
    assert_eq!(queue.sessions(), ["s"]);
    let taken = queue.take("s", at(1000), true);
    assert_eq!(taken.ready, ["一", "二", "三"], "照入队的先后");
    assert!(taken.expired.is_empty());
    assert!(queue.sessions().is_empty(), "交完了不再排");
}

#[test]
fn waiting_until_the_deadline_expires_even_behind_a_shut_door() {
    let mut queue = Queue::new(EXPIRE);
    queue.push("s", at(0), "一");
    queue.push("s", at(30_000), "二");
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
    queue.push("s", at(0), "旧");
    queue.push("s", at(50_000), "新");
    let taken = queue.take("s", at(61_000), true);
    assert_eq!(taken.expired, ["旧"]);
    assert_eq!(taken.ready, ["新"], "过期的不交，没过期的照交");
}

#[test]
fn sessions_wait_apart() {
    let mut queue = Queue::new(EXPIRE);
    queue.push("甲", at(0), "一");
    queue.push("乙", at(0), "二");
    queue.push("甲", at(1), "三");
    assert_eq!(queue.sessions(), ["乙", "甲"]);
    let taken = queue.take("甲", at(10), true);
    assert_eq!(taken.ready, ["一", "三"]);
    assert_eq!(queue.sessions(), ["乙"], "别的会话的不动");
    let taken = queue.take("丙", at(10), true);
    assert!(
        taken.ready.is_empty() && taken.expired.is_empty(),
        "没排的什么都没有"
    );
}

#[test]
fn it_wakes_at_the_first_deadline_or_the_end_of_a_mute() {
    let empty: Queue<&str> = Queue::new(EXPIRE);
    assert_eq!(empty.wake(|_| Some(at(5))), None, "没排着的不醒");
    let mut queue = Queue::new(EXPIRE);
    queue.push("甲", at(10_000), "一");
    queue.push("甲", at(20_000), "二");
    queue.push("乙", at(15_000), "三");
    assert_eq!(queue.wake(|_| None), Some(at(70_000)), "最早的过期");
    let muted = |session: &str| (session == "乙").then(|| at(30_000));
    assert_eq!(queue.wake(muted), Some(at(30_000)), "禁言先到期");
    let later = |_: &str| Some(at(90_000));
    assert_eq!(queue.wake(later), Some(at(70_000)), "过期先到");
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
