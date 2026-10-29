//! 陌生人的私聊（施工 O-60，`onebot.md`「陌生人的私聊」）：真核心拉起真桥、假 NapCat，头经 `cards.save` 打开「其他私聊」卡。打开
//! 了的陌生人私聊她会回；卡片的限流满了的、睡觉时间里的只记下、不回；卡片的附加提示词只进陌生人的会话，终端管理员的私聊不套这张
//! 卡；卡片再关上，聊过的陌生人也不再接。卡片关着（出厂）的陌生人照旧不接（`whitelist.rs`）。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

/// 起来（系统的规则 `rules`）、等 NapCat 连上，存卡片 `other_privates`（「其他群」照测试的老样子开着）。
async fn up_with(lines: &Lines, rules: &str, other_privates: Value) -> (Home, Answering) {
    let (home, napcat, _) = started_by(Arc::new(lines.clone()), rules, MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    save(&mut head, other_privates).await;
    (home, napcat)
}

async fn up(lines: &Lines, other_privates: Value) -> (Home, Answering) {
    up_with(lines, "", other_privates).await
}

async fn save(head: &mut Head, other_privates: Value) {
    let cards = json!({"cards": {
        "other_groups": {"on": true, "final_only": false},
        "other_privates": other_privates,
    }});
    let saved = head.page_call_with("cards.save", cards).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
}

/// 现在前后一小时都在睡觉时间里的私聊规则。
fn asleep_now() -> String {
    let now = jiff::Zoned::now();
    let minute = i64::from(now.hour()) * 60 + i64::from(now.minute());
    let clock = |minute: i64| {
        let minute = minute.rem_euclid(24 * 60);
        format!("{:02}:{:02}", minute / 60, minute % 60)
    };
    format!(
        "[[rule]]\nmatch = {{ kind = \"private\" }}\nsleep = \"{}-{}\"\n",
        clock(minute - 60),
        clock(minute + 60)
    )
}

/// 下一个发给 `user` 的私聊的字。
async fn private_reply(napcat: &mut Answering, user: i64) -> String {
    loop {
        let action = napcat.action().await;
        if action["action"] == "send_private_msg" && action["params"]["user_id"] == user {
            return action["params"]["message"][0]["data"]["text"]
                .as_str()
                .expect("有字")
                .to_string();
        }
    }
}

#[tokio::test]
async fn an_opened_card_lets_strangers_talk_to_her() {
    let lines = Lines::new([Line::says("你好呀")]);
    let (home, mut napcat) = up(&lines, json!({"on": true})).await;
    napcat.send(private_frame(STRANGER, 1, json!([plain("你好")])));
    assert_eq!(private_reply(&mut napcat, STRANGER).await, "你好呀");
    let events = until_events(
        &home.root,
        &format!("qq:30003:private:{STRANGER}"),
        |events| !of_kind(events, "turn.ended").is_empty(),
    )
    .await;
    assert_eq!(
        said(&events).iter().map(words).collect::<Vec<_>>(),
        ["你好"]
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_stranger_over_the_rate_is_only_recorded() {
    let lines = Lines::new([Line::says("一")]);
    let (home, mut napcat) = up(&lines, json!({"on": true, "rate": "1/60s"})).await;
    napcat.send(private_frame(STRANGER, 1, json!([plain("第一句")])));
    assert_eq!(private_reply(&mut napcat, STRANGER).await, "一");
    napcat.send(private_frame(STRANGER, 2, json!([plain("第二句")])));
    let events = until_events(
        &home.root,
        &format!("qq:30003:private:{STRANGER}"),
        |events| said(events).len() == 2,
    )
    .await;
    let second = &said(&events)[1];
    assert_eq!(words(second), "第二句");
    assert_eq!(
        second["body"]["venue"]["ambient"], true,
        "只记下、不开回合：{second}"
    );
    assert!(
        second["body"]["venue"].get("asleep").is_none(),
        "限流不是睡觉：{second}"
    );
    assert_eq!(of_kind(&events, "turn.started").len(), 1, "只开了一轮");
    assert!(napcat.pending().is_none(), "不出声");
    assert!(run_log(&home.root).contains("stranger recorded only"));
    stopped(home).await;
}

#[tokio::test]
async fn the_strangers_card_does_not_reach_the_terminal_admin() {
    // 附加提示词只进陌生人的会话：终端管理员先私聊两句（第一句问出他是终端管理员，「其他私聊」卡排开他；第二句他的会话找过了，
    // 照这一刻的规则放提示词），陌生人再私聊。
    let lines = Lines::new([
        Line::says("管理员好"),
        Line::says("又来了"),
        Line::says("陌生人好"),
    ]);
    let (home, mut napcat) = up(
        &lines,
        json!({"on": true, "extra_prompt": "只给陌生人看的话"}),
    )
    .await;
    napcat.send(private_frame(ADMIN, 1, json!([plain("在吗")])));
    assert_eq!(private_reply(&mut napcat, ADMIN).await, "管理员好");
    napcat.send(private_frame(ADMIN, 2, json!([plain("还在吗")])));
    assert_eq!(private_reply(&mut napcat, ADMIN).await, "又来了");
    napcat.send(private_frame(STRANGER, 3, json!([plain("你好")])));
    assert_eq!(private_reply(&mut napcat, STRANGER).await, "陌生人好");
    let requests = lines.requests();
    let system = |n: usize| format!("{:?}", requests[n].system);
    assert!(!system(0).contains("只给陌生人看的话"), "终端管理员的不套");
    assert!(!system(1).contains("只给陌生人看的话"), "认出来以后也不套");
    assert!(system(2).contains("只给陌生人看的话"), "陌生人的套");
    stopped(home).await;
}

#[tokio::test]
async fn a_stranger_in_the_sleep_time_is_only_recorded() {
    let lines = Lines::new([Line::says("不该说话")]);
    let (home, mut napcat) = up_with(&lines, &asleep_now(), json!({"on": true})).await;
    napcat.send(private_frame(STRANGER, 1, json!([plain("睡了吗")])));
    let events = until_events(
        &home.root,
        &format!("qq:30003:private:{STRANGER}"),
        |events| !said(events).is_empty(),
    )
    .await;
    let venue = &said(&events)[0]["body"]["venue"];
    assert_eq!(
        (&venue["ambient"], &venue["asleep"]),
        (&json!(true), &json!(true)),
        "{venue}"
    );
    assert!(of_kind(&events, "turn.started").is_empty(), "不开回合");
    assert!(napcat.pending().is_none(), "不出声");
    stopped(home).await;
}

#[tokio::test]
async fn a_closed_card_stops_strangers_she_talked_to() {
    // 两种关法：后台页存整份卡片（`cards.save`），整页存「其他私聊」那一组（`cards.save_one`）。
    for by_page in [false, true] {
        let lines = Lines::new([Line::says("你好呀"), Line::says("不该说话")]);
        let (home, mut napcat) = up(&lines, json!({"on": true})).await;
        napcat.send(private_frame(STRANGER, 1, json!([plain("你好")])));
        assert_eq!(private_reply(&mut napcat, STRANGER).await, "你好呀");
        let mut head = Head::connect(&home.root).await;
        if by_page {
            let values = json!({"card": {"kind": "other_privates"}, "values": {"on": false}});
            let saved = head.page_call_with("cards.save_one", values).await;
            assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
        } else {
            save(&mut head, json!({"on": false})).await;
        }
        napcat.send(private_frame(STRANGER, 2, json!([plain("还在吗")])));
        until_logged(&home.root, |log| {
            log.contains("not admin or whitelisted, not taken")
        })
        .await;
        let events = venue_events(&home.root, &format!("qq:30003:private:{STRANGER}"));
        assert_eq!(
            said(&events).iter().map(words).collect::<Vec<_>>(),
            ["你好"],
            "第二句不进会话（{by_page}）"
        );
        assert!(napcat.pending().is_none(), "不出声");
        stopped(home).await;
    }
}
