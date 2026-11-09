//! 引用照交出去那一刻的群（施工 I-64，10-28 项目主人：隔了好几条消息却不引用）：她回 A 的那句照打字的工夫停着没发，B 叫她、
//! C 插了一句；她接着回 B。群里看到的是 A、B、C、她回 A、她回 B，回 B 前面最后一条是她自己的，照规则引用 B 那一条。原来写好那一刻
//! 就定了带不带，那时回 A 还没发出去，算成群里最后一条是 C 的、B 之后只隔一条，不引用。真核心拉起真桥、假 NapCat。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群号。
const GROUP: i64 = 3702;

/// 群里的几个人。
const A: i64 = 20041;
const B: i64 = 20042;
const C: i64 = 20043;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[
    (BOT, "米尤", "miyu"),
    (A, "阿甲", "a"),
    (B, "阿乙", "b"),
    (C, "阿丙", "c"),
];

/// 叫她才回（不过判官），出站照出厂（隔 4 条才引用，15 秒以后才 @），照真人的样子发（出厂开）。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\ndiscipline = \"when-called\"\n";

/// `user` 在群里说第 `message` 条，`at_her` 的 @ 她。
fn says(napcat: &Answering, user: i64, message: i64, text: &str, at_her: bool) {
    let (_, card, nickname) = MEMBERS
        .iter()
        .find(|(id, _, _)| *id == user)
        .expect("认得的成员");
    let mut words = Vec::new();
    if at_her {
        words.push(at(BOT));
    }
    words.push(plain(&format!(" {text}")));
    napcat.send(group_frame(
        GROUP,
        user,
        message,
        json!(words),
        (card, nickname),
    ));
}

/// 一条消息里引用的平台编号。
fn quoted(segments: &[Value]) -> Option<String> {
    segments
        .iter()
        .find(|segment| segment["type"] == "reply")
        .and_then(|segment| segment["data"]["id"].as_str().map(str::to_string))
}

#[tokio::test]
async fn a_reply_quotes_when_her_own_earlier_line_went_out_just_before_it() {
    let lines = Lines::new([Line::says("在"), Line::says("回A"), Line::says("回B")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    // 先说过一句：后面那句照打字的工夫从这一句发出去算着等（施工 I-58）。
    says(&napcat, A, 100, "miyu", true);
    let hello = napcat.group_message(GROUP).await;
    assert!(format!("{hello:?}").contains('在'), "{hello:?}");
    says(&napcat, A, 101, "在吗", true);
    within("她回 A", async {
        while lines.requests().len() < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    // 回 A 照打字的工夫停着（两秒多），这时 B 叫她、C 插一句。
    says(&napcat, B, 201, "有空吗", true);
    says(&napcat, C, 301, "今天好热", false);
    let first = napcat.group_message(GROUP).await;
    let second = napcat.group_message(GROUP).await;
    let text = |segments: &[Value]| {
        segments
            .iter()
            .filter_map(|segment| segment["data"]["text"].as_str())
            .collect::<String>()
    };
    assert!(text(&first).contains("回A"), "{first:?}");
    assert!(text(&second).contains("回B"), "{second:?}");
    assert_eq!(
        quoted(&second).as_deref(),
        Some("201"),
        "回 B 前面最后一条是她自己的，引用 B 那一条：{second:?}\n{}",
        run_log(&home.root)
    );
    assert!(
        run_log(&home.root).contains("reply aimed"),
        "交出去时记了一行带没带引用"
    );
    stopped(home).await;
}

#[tokio::test]
async fn her_place_in_the_group_is_where_she_handed_it_over() {
    // 同上，回 B 那句等回 A 发出去、回执回来以后才说完：她回 A 那句在群里排在 B、C 后面（交出去那一刻），不照说它的那一刻算。
    let (release_b, hold_b) = tokio::sync::oneshot::channel();
    let lines = Lines::new([
        Line::says("在"),
        Line::says("回A"),
        Line::says("回B").released_by(hold_b),
    ]);
    let (home, mut napcat, _) = started_by(Arc::new(lines.clone()), RULES, MEMBERS).await;
    says(&napcat, A, 100, "miyu", true);
    napcat.group_message(GROUP).await;
    says(&napcat, A, 101, "在吗", true);
    within("她回 A", async {
        while lines.requests().len() < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    says(&napcat, B, 201, "有空吗", true);
    says(&napcat, C, 301, "今天好热", false);
    let first = napcat.group_message(GROUP).await;
    assert!(format!("{first:?}").contains("回A"), "{first:?}");
    // 回 A 发出去了、回执也回来了，再让回 B 说完。
    tokio::time::sleep(Duration::from_millis(300)).await;
    release_b.send(()).expect("还在等");
    let second = napcat.group_message(GROUP).await;
    assert_eq!(
        quoted(&second).as_deref(),
        Some("201"),
        "{second:?}\n{}",
        run_log(&home.root)
    );
    stopped(home).await;
}
