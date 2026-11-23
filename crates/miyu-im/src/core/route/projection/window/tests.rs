//! 投影只留一个窗口（施工 LW-IM-7）：丢的只是又旧、又不在最近几条里的人话和旧的回合；说过话的人那张表和一条都不丢时一样（名字、
//! 终端管理员、终端管理员用过的名字，连只在窗口以外出现过的）；丢过的交出查库的范围；引用的那一条丢掉以后投影认不出发的人。

use miyu_client::events::Event;
use serde_json::json;

use super::super::Projection;
use super::super::tests::{LINE, event, qq, started, take_all};
use super::Keep;

/// 一分钟、至少留三条。
const KEEP: Keep = Keep {
    millis: 60_000,
    floor: 3,
};

/// 号是 `user` 的人第 `seconds` 秒说的第 `seq` 条：名字 `name`（没有的不写）、`admin` 的带 `owner`、引用 `reply_to`。
fn said(seq: u64, seconds: i64, user: i64, name: Option<&str>, admin: bool) -> Event {
    quoting(seq, seconds, user, (name, admin), None)
}

fn quoting(
    seq: u64,
    seconds: i64,
    user: i64,
    (name, admin): (Option<&str>, bool),
    reply_to: Option<&str>,
) -> Event {
    let mut by =
        json!({"kind": "external", "venue": "qq:30003:group:5", "id": format!("qq:{user}")});
    if admin {
        by["owner"] = json!(true);
    }
    let mut venue = json!({"msg": format!("m{seq}"), "ambient": true});
    if let Some(name) = name {
        venue["name"] = json!(name);
    }
    if let Some(reply_to) = reply_to {
        venue["reply_to"] = json!(reply_to);
    }
    let body = json!({"blocks": [{"type": "text", "text": format!("第{seq}句")}], "venue": venue});
    event(seq, seconds, "message.user", None, by, body)
}

/// 一段历史：终端管理员（10001）早先用过两个名字、后来又换一个、最后一句没写名字；别人（20002）只在很早说过话。
fn history() -> Vec<Event> {
    vec![
        said(1, 0, 10001, Some("老板"), true),
        said(2, 1, 20002, Some("小林"), false),
        said(3, 2, 10001, Some("店长"), true),
        said(4, 3, 30003, None, false),
        said(5, 600, 10001, Some("掌柜"), true),
        said(6, 601, 10001, None, true),
        said(7, 602, 40004, Some("小周"), false),
        said(8, 603, 40004, Some("小周"), false),
        said(9, 604, 50005, None, false),
    ]
}

#[test]
fn only_old_messages_beyond_the_floor_are_dropped() {
    let mut kept = Projection::new(LINE, 0).windowed(KEEP);
    take_all(&mut kept, history());
    // 第 600 秒以后的五条都在窗口里；更早的四条过了一分钟、又不在最近三条里，丢掉。
    let known: Vec<u64> = (1..=9).filter(|seq| kept.knows(*seq)).collect();
    assert_eq!(known, [5, 6, 7, 8, 9]);
    // 很安静的群：都旧了也至少留最近三条。
    let mut quiet = Projection::new(LINE, 0).windowed(KEEP);
    take_all(
        &mut quiet,
        (1..=6)
            .map(|seq| said(seq, seq as i64 * 1000, 20002, None, false))
            .collect(),
    );
    let known: Vec<u64> = (1..=6).filter(|seq| quiet.knows(*seq)).collect();
    assert_eq!(known, [4, 5, 6]);
}

#[test]
fn the_people_table_answers_like_the_whole_history() {
    let mut whole = Projection::new(LINE, 0);
    let mut kept = Projection::new(LINE, 0).windowed(KEEP);
    take_all(&mut whole, history());
    take_all(&mut kept, history());
    assert!(!kept.knows(2), "小林那一句已经丢出窗口");
    for user in [10001, 20002, 30003, 40004, 50005, 60006] {
        let who = qq(user);
        assert_eq!(kept.is_admin(&who), whole.is_admin(&who), "{user}");
        assert_eq!(kept.name_of(&who), whole.name_of(&who), "{user}");
    }
    assert_eq!(kept.admin_names(), whole.admin_names());
    assert_eq!(kept.admin_names(), ["老板", "店长", "掌柜"]);
    assert_eq!(
        kept.name_of(&qq(10001)).as_deref(),
        Some("掌柜"),
        "没写名字的那一句不抹掉"
    );
    assert_eq!(
        kept.name_of(&qq(20002)).as_deref(),
        Some("小林"),
        "只在窗口以外说过话的"
    );
    assert!(kept.is_admin(&qq(10001)));
}

#[test]
fn old_turns_are_dropped_and_recent_ones_count_the_same() {
    let mut whole = Projection::new(LINE, 0);
    let mut kept = Projection::new(LINE, 0).windowed(KEEP);
    let events = vec![
        said(1, 0, 20002, None, false),
        started(2, 1, &[1]),
        said(3, 600, 20002, None, false),
        started(4, 601, &[3]),
        said(5, 602, 10001, None, true),
        started(6, 603, &[5]),
    ];
    take_all(&mut whole, events.clone());
    take_all(&mut kept, events);
    // 窗口以内的回合两边一样；第 1 秒那一轮一分钟以前，丢掉（限流照的窗口不超过一天，窗口不会比它短）。
    let recent = |turns: Vec<miyu_client::time::Timestamp>| -> Vec<i64> {
        turns
            .into_iter()
            .map(|at| at.unix_millis())
            .filter(|at| *at >= super::super::tests::at(600).unix_millis())
            .collect()
    };
    assert_eq!(recent(kept.turns(&[])), recent(whole.turns(&[])));
    assert_eq!(
        kept.turns(&[]).len(),
        1,
        "终端管理员的那一轮照旧不算，第 1 秒的丢掉"
    );
    assert_eq!(whole.turns(&[]).len(), 2);
}

#[test]
fn the_pruned_range_and_quotes_of_dropped_messages() {
    let mut kept = Projection::new(LINE, 0).windowed(KEEP);
    assert_eq!(kept.pruned(), None);
    take_all(&mut kept, history());
    let (first, until) = kept.pruned().expect("丢过");
    assert_eq!(first, super::super::tests::at(0));
    assert_eq!(until, super::super::tests::at(3));
    // 没给窗口的一条都不丢。
    let mut whole = Projection::new(LINE, 0);
    take_all(&mut whole, history());
    assert_eq!(whole.pruned(), None);
    // 引用了丢出窗口的那一条：窗口里认不出是谁（路由去库里查），一条都不丢的照旧认得出。
    let quoted = vec![
        quoting(10, 605, 40004, (None, false), Some("m2")),
        started(11, 606, &[10]),
    ];
    take_all(&mut kept, quoted.clone());
    take_all(&mut whole, quoted);
    let quote = |projection: &Projection| projection.origin(LINE).and_then(|origin| origin.quote);
    assert_eq!(quote(&kept).expect("有引用").sender, None);
    assert_eq!(quote(&whole).expect("有引用").sender, Some(qq(20002)));
}

/// 她发出去的一段：平台编号 `msg`，第 `seconds` 秒。
fn delivered(seq: u64, seconds: i64, msg: &str) -> Event {
    let body = json!({"line": LINE, "turn": 1, "to": [], "msg": msg, "text": "在。"});
    event(
        seq,
        seconds,
        "venue.delivered",
        None,
        json!({"kind": "module", "id": "im"}),
        body,
    )
}

#[test]
fn her_old_messages_stay_hers_and_old_recalls_change_nothing() {
    let mut whole = Projection::new(LINE, 0);
    let mut kept = Projection::new(LINE, 0).windowed(KEEP);
    let mut events = vec![delivered(100, 0, "her-1")];
    events.extend(history().into_iter().map(|one| {
        // 她的那一段排在最前：序号要比后面的人话小。
        let mut line = serde_json::to_value(&one).expect("写得成");
        line["seq"] = json!(line["seq"].as_u64().expect("有序号") + 100);
        serde_json::from_value(line).expect("读得成")
    }));
    take_all(&mut whole, events.clone());
    take_all(&mut kept, events);
    // 她发过的编号不丢：引用她很早的一句照样认得出是她的。
    assert!(kept.mine("her-1"));
    assert_eq!(kept.mine("her-1"), whole.mine("her-1"));
    // 撤回丢出窗口的一条：窗口里认不出（路由什么都不做）；一条都不丢时认得出，但「这一轮只在回它」也是假，路由同样什么都不做。
    assert_eq!(kept.withdrawn("m2"), None);
    assert_eq!(whole.withdrawn("m2"), Some((102, false)));
    // 窗口里的照旧认得出。
    assert_eq!(kept.withdrawn("m9"), whole.withdrawn("m9"));
}
