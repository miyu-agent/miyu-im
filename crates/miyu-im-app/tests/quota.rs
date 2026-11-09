//! 使用限制（设计 08，施工 I-135）：真核心照开关拉起真的桥，假 NapCat 发群消息。
//!
//! - 每人：使用限制页的设置（im 配置目录里的 `limits.toml`）默认 5 小时 1 个 token：小林叫她，头一回回了，记上用量以后再叫只记下
//!   （`quota_reached`），不出声。
//! - 会话：卡片上的 `limit_5h = 1`，用量照核心的账本（这里查到的不留，`usage_refresh_seconds` 是 0）：别人叫她迟早只记下；终端
//!   管理员不受它管，照回。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群号。
const GROUP: i64 = 4343;

/// 叫她的人。
const LIN: i64 = 20002;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 场所：这个群。
fn venue() -> String {
    format!("qq:{BOT}:group:{GROUP}")
}

/// 她回的几句：够用。
fn lines() -> Lines {
    Lines::new((0..10).map(|_| Line::says("在")))
}

/// `user` 在群里 @ 她：第 `message` 条。
fn calls(napcat: &Answering, user: i64, message: i64) {
    let words = json!([at(BOT), plain(" 在吗")]);
    napcat.send(group_frame(GROUP, user, message, words, ("某人", "x")));
}

/// 等到判过 `n` 条，交回最后一条判断。
async fn decided(home: &Home, n: usize) -> Value {
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "ext.im.chat.decided").len() >= n
    })
    .await;
    of_kind(&events, "ext.im.chat.decided")[n - 1].clone()
}

#[tokio::test]
async fn a_person_over_the_limit_is_only_recorded() {
    let lines = lines();
    let rules = "[[rule]]\nmatch = { kind = \"group\" }\n";
    let (home, mut napcat, _) = started_tuned(Arc::new(lines), rules, &json!({}), |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let config = miyu_im::dirs::config(&home.root);
    std::fs::create_dir_all(&config).expect("建得了");
    std::fs::write(config.join("limits.toml"), "[default]\nlimit_5h = 1\n").expect("写得了");
    calls(&napcat, LIN, 1);
    assert_eq!(napcat.group_reply(GROUP).await, "在", "头一回还没用过");
    until_events(&home.root, &venue(), |events| {
        !of_kind(events, "turn.ended").is_empty()
    })
    .await;
    calls(&napcat, LIN, 2);
    let second = decided(&home, 2).await;
    assert_eq!(second["body"]["inbound"], "record_only", "{second}");
    assert_eq!(second["body"]["why"], "quota_reached", "{second}");
    assert!(napcat.pending().is_none(), "不出声");
    stopped(home).await;
}

#[tokio::test]
async fn a_full_chat_records_others_and_still_answers_the_admin() {
    let lines = lines();
    let rules = "[[rule]]\nmatch = { kind = \"group\" }\nlimit_5h = 1\n";
    let tuned = json!({"usage_refresh_seconds": 0});
    let (home, mut napcat, _) = started_tuned(Arc::new(lines), rules, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    // 会话的用量在后台查、先照上一份判：头几回照常回，查到了用量以后只记下。
    let mut reached = None;
    for message in 1..=8 {
        calls(&napcat, LIN, message);
        let one = decided(&home, usize::try_from(message).expect("小")).await;
        if one["body"]["why"] == "quota_reached" {
            reached = Some(message);
            break;
        }
        assert_eq!(napcat.group_reply(GROUP).await, "在");
        until_events(&home.root, &venue(), |events| {
            of_kind(events, "turn.ended").len() >= usize::try_from(message).expect("小")
        })
        .await;
    }
    let reached = reached.expect("八回以内查到用量、满了");
    assert!(reached > 1, "头一回还没用过");
    calls(&napcat, ADMIN, 100);
    assert_eq!(napcat.group_reply(GROUP).await, "在", "终端管理员不受它管");
    stopped(home).await;
}
