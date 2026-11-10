//! 叫她做的那条（施工 O-31，`onebot.md` 第一条「平台工具（一）」第 3 条）：投影另记人说的话的引用和 @；主线这一轮她回的那一条
//! 交出来（`turn.joined` 并进来的换上，这一轮完了没有）；引用的是她的、是谁的、桥不认识的；终端管理员照他说过的话认。

use serde_json::json;

use super::Projection;
use super::tests::{LINE, ended, event, joined, qq, started, take_all};
use crate::toolset::{Origin, Quote};

/// 号是 `user` 的人说的第 `seq` 条：引用 `reply_to`、@ 了 `mentions`；`admin` 的带 `owner`。
fn quoting(
    seq: u64,
    user: i64,
    reply_to: Option<&str>,
    mentions: &[i64],
    admin: bool,
) -> miyu_client::events::Event {
    let mut by =
        json!({"kind": "external", "venue": "qq:30003:group:5", "id": format!("qq:{user}")});
    if admin {
        by["owner"] = json!(true);
    }
    let mut venue = json!({"msg": seq.to_string(), "ambient": true});
    if let Some(reply_to) = reply_to {
        venue["reply_to"] = json!(reply_to);
    }
    if !mentions.is_empty() {
        let mentions: Vec<String> = mentions.iter().map(|one| format!("qq:{one}")).collect();
        venue["mentions"] = json!(mentions);
    }
    let body = json!({"blocks": [{"type": "text", "text": "嗨"}], "venue": venue});
    event(seq, 0, "message.user", None, by, body)
}

/// 她发出去的一段：平台编号 `msg`。
fn delivered(seq: u64, msg: &str) -> miyu_client::events::Event {
    let body = json!({"line": "01a0d78c-ca52-7d19-8b64-0e3f5a7c2d91", "turn": 1, "to": [], "msg": msg, "text": "在。"});
    event(
        seq,
        0,
        "venue.delivered",
        None,
        json!({"kind": "module", "id": "im"}),
        body,
    )
}

#[test]
fn the_origin_is_the_message_she_answers_with_its_quote_and_mentions() {
    let mut projection = Projection::new(LINE, 0);
    take_all(
        &mut projection,
        vec![
            quoting(1, 10001, None, &[], true),
            delivered(2, "90001"),
            quoting(3, 20002, Some("1"), &[20003], false),
        ],
    );
    assert_eq!(projection.origin(LINE), None, "主线不在跑");
    take_all(&mut projection, vec![started(4, 0, &[3])]);
    assert_eq!(
        projection.origin(LINE),
        Some(Origin {
            sender: qq(20002),
            quote: Some(Quote {
                msg: "1".to_string(),
                sender: Some(qq(10001)),
            }),
            mentions: vec![qq(20003)],
        })
    );
    // 并进来的换上：引用她的。
    take_all(
        &mut projection,
        vec![
            quoting(5, 20003, Some("90001"), &[], false),
            joined(6, 4, &[5]),
        ],
    );
    assert_eq!(
        projection.origin(LINE),
        Some(Origin {
            sender: qq(20003),
            quote: Some(Quote {
                msg: "90001".to_string(),
                sender: None,
            }),
            mentions: Vec::new(),
        })
    );
    take_all(&mut projection, vec![ended(7, 4)]);
    assert_eq!(projection.origin(LINE), None, "这一轮完了");
    // 引用桥不认识的：不知道是谁的。
    take_all(
        &mut projection,
        vec![
            quoting(8, 20002, Some("777"), &[], false),
            started(9, 0, &[8]),
        ],
    );
    let quote = projection.origin(LINE).and_then(|origin| origin.quote);
    assert_eq!(
        quote,
        Some(Quote {
            msg: "777".to_string(),
            sender: None,
        })
    );
}

#[test]
fn the_terminal_admin_is_known_by_what_he_said_here() {
    let mut projection = Projection::new(LINE, 0);
    take_all(
        &mut projection,
        vec![
            quoting(1, 20002, None, &[], false),
            quoting(2, 10001, None, &[], true),
        ],
    );
    assert!(projection.is_admin(&qq(10001)));
    assert!(!projection.is_admin(&qq(20002)));
    assert!(!projection.is_admin(&qq(30003)), "没说过话的认不出");
}

/// 一笔放行了的判断：判的是 `msgs`，条件是 `kinds`。
fn judged(seq: u64, msgs: &[u64], kinds: &[&str]) -> miyu_client::events::Event {
    let hits: Vec<serde_json::Value> = kinds
        .iter()
        .map(|kind| json!({"kind": kind, "bonus": 0.0}))
        .collect();
    let body = json!({"msgs": msgs, "inbound": "pass", "conditions": hits, "outcome": "reply"});
    event(
        seq,
        0,
        "ext.im.chat.decided",
        None,
        json!({"kind": "module", "id": "im"}),
        body,
    )
}

#[test]
fn only_a_turn_a_manager_called_her_into_counts_as_commanded() {
    // 施工 I-204（core1 审出来的）：删群文件这种收不回来的事，只认管理的人直接叫她开的那一轮。
    let manager = |who: &miyu_client::ids::ExternalId| *who == qq(20009);
    let mut projection = Projection::new(LINE, 0);
    take_all(
        &mut projection,
        vec![
            quoting(1, 20002, None, &[], false),
            judged(2, &[1], &["direct"]),
            quoting(3, 20009, None, &[], false),
            judged(4, &[3], &["direct"]),
            quoting(5, 20009, None, &[], false),
            judged(6, &[5], &["sampled"]),
            quoting(7, 10001, None, &[], true),
            judged(8, &[7], &["direct"]),
        ],
    );
    assert!(!projection.commanded(LINE, manager), "没在跑的不算");
    // 管理的人直接叫她。
    take_all(&mut projection, vec![started(9, 0, &[3])]);
    assert!(projection.commanded(LINE, manager));
    // 普通群友的一条并进来：这一轮在回的有他的话，不算了。
    take_all(&mut projection, vec![joined(10, 9, &[1])]);
    assert!(!projection.commanded(LINE, manager), "并进来群友的话");
    take_all(&mut projection, vec![ended(11, 9)]);
    // 群友叫起来的一轮，管理的人并进来：照开这一轮的那一条，也不算。
    take_all(
        &mut projection,
        vec![started(12, 0, &[1]), joined(13, 12, &[3])],
    );
    assert!(!projection.commanded(LINE, manager), "群友开的一轮");
    take_all(&mut projection, vec![ended(14, 12)]);
    // 概率抽中的是管理的人的话，可他没叫她。
    take_all(&mut projection, vec![started(15, 0, &[5])]);
    assert!(!projection.commanded(LINE, manager), "抽中的不是叫她");
    take_all(&mut projection, vec![ended(16, 15)]);
    // 没有触发的一轮（看一眼、定时任务）。
    take_all(&mut projection, vec![started(17, 0, &[])]);
    assert!(!projection.commanded(LINE, manager), "没人叫的一轮");
    take_all(&mut projection, vec![ended(18, 17)]);
    // 终端管理员直接叫她（照他说的话带的 owner 认，不靠名单）。
    take_all(&mut projection, vec![started(19, 0, &[7])]);
    assert!(projection.commanded(LINE, |_| false));
}
