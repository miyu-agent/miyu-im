//! 实时日志的一行说成话（设计 10，施工 I-160 上）：照桥的字表，中英日三份都有每一句；时刻换成本机的钟点。

use serde_json::{Value, json};

use miyu_im::texts::Texts;
use miyu_im::watch::render::{Shown, clock, shown};
use miyu_im::watch::{Block, Extra, Item, Judged, Line, Score, lines};
use miyu_store::resources::ResourceRoot;

use crate::support::*;

fn texts(language: &str) -> Texts {
    Texts::load(
        ResourceRoot::at(resources()),
        package_in(resources()),
        language,
    )
    .expect("读得出来")
}

/// 事件说成话：东八区，触发的第 7 条是小林的，`qq_quote` 的显示名是「引用」。
fn said(event: Value, texts: &Texts) -> Vec<Shown> {
    let names = |seq: u64| (seq == 7).then(|| "小林".to_string());
    let tool = |name: &str| (name == "qq_quote").then(|| "引用".to_string());
    lines(&event, "L", &names)
        .iter()
        .map(|line| shown(line, 480, texts, &tool))
        .collect()
}

fn texts_of(shown: &[Shown]) -> Vec<String> {
    shown
        .iter()
        .map(|one| format!("{} {}", one.at, one.text))
        .collect()
}

#[test]
fn a_turn_reads_like_what_she_did() {
    let zh = texts("zh");
    let heard = json!({"at": "2026-11-05T11:33:42.000Z", "kind": "message.user", "body": {
        "blocks": [{"type": "text", "text": "@Miyu 给出 winrar 官网链接"}], "venue": {"name": "小林"}}});
    let decided = json!({"at": "2026-11-05T11:33:48.000Z", "kind": "ext.im.chat.decided", "body": {
        "conditions": [{"kind": "direct"}], "judge": {"answer": {"reason": "直接@了她"}, "millis": 6496},
        "msgs": [7], "outcome": "reply", "score": {"total": 1.33, "threshold": 0.9}}});
    let started =
        json!({"at": "2026-11-05T11:33:48.000Z", "kind": "turn.started", "body": {"trigger": 7}});
    let wrote = json!({"at": "2026-11-05T11:33:53.000Z", "kind": "message.assistant", "body": {"seen": 3, "blocks": [
        {"type": "reasoning", "text": "给链接就行"},
        {"type": "tool_call", "name": "qq_quote", "args": "{\"msg\": \"717275606\"}"},
        {"type": "text", "text": "https://www.rarlab.com"}]}});
    let result = json!({"at": "2026-11-05T11:33:53.500Z", "kind": "tool.result", "body": {"status": "ok",
        "display_name": {"zh": "引用"}, "blocks": [{"type": "text", "text": "Your next message will quote it.\n"}]}});
    let sent = json!({"at": "2026-11-05T11:33:53.414Z", "kind": "venue.delivered", "body": {"text": "https://www.rarlab.com"}});
    let all: Vec<Shown> = [heard, decided, started, wrote, result, sent]
        .into_iter()
        .flat_map(|event| said(event, &zh))
        .collect();
    assert_eq!(
        texts_of(&all),
        [
            "19:33:42 小林：@Miyu 给出 winrar 官网链接",
            "19:33:48 判断 小林：要回 · 叫了 AI · 1.33 门槛 0.90 · 判官 6.5 秒：直接@了她",
            "19:33:48 ▶ 开始回复 小林",
            "19:33:53 想：给链接就行",
            "19:33:53 调用 引用 {\"msg\": \"717275606\"}",
            "19:33:53 说：https://www.rarlab.com",
            "19:33:53 结果 引用：Your next message will quote it.",
            "19:33:53 发出：https://www.rarlab.com",
        ]
    );
    let kinds: Vec<&str> = all.iter().map(|one| one.kind).collect();
    assert_eq!(
        kinds,
        [
            "heard",
            "decided",
            "started",
            "thinking",
            "call",
            "said",
            "result",
            "delivered"
        ]
    );
    assert_eq!(all[3].id.as_deref(), Some("L:3:0"), "块带编号");
}

#[test]
fn the_missed_question_shows_why_it_was_only_recorded() {
    // 11-05 主群：主人问「这表情包哪里来的」，一个条件都不成立；前一条「？」判官超时。
    let zh = texts("zh");
    let quiet = json!({"at": "2026-11-05T11:04:44.000Z", "kind": "ext.im.chat.decided", "body": {
        "conditions": [], "msgs": [7], "outcome": "record", "route": "record"}});
    let timeout = json!({"at": "2026-11-05T11:04:30.000Z", "kind": "ext.im.chat.decided", "body": {
        "conditions": [{"kind": "after_speaking"}], "judge": {"millis": 15147, "unjudged": "timeout"},
        "msgs": [7], "outcome": "record"}});
    let limited = json!({"at": "2026-11-05T08:36:33.000Z", "kind": "ext.im.chat.decided", "body": {
        "msgs": [7], "outcome": "record", "why": "rate_limited"}});
    let all: Vec<Shown> = [quiet, timeout, limited]
        .into_iter()
        .flat_map(|event| said(event, &zh))
        .collect();
    assert_eq!(
        texts_of(&all),
        [
            "19:04:44 判断 小林：只记下 · 没叫 AI",
            "19:04:30 判断 小林：只记下 · 刚说过话 · 判官等了 15.1 秒没回",
            "16:36:33 判断 小林：只记下 · 没叫 AI · 限流",
        ]
    );
}

#[test]
fn every_line_has_words_in_all_three_languages() {
    let items = vec![
        Item::Heard {
            who: "a".into(),
            text: "b".into(),
            extras: vec![
                Extra::Images(2),
                Extra::Sticker,
                Extra::Forward(3),
                Extra::Other("file".into()),
            ],
        },
        Item::Decided {
            who: Some("a".into()),
            outcome: "reply".into(),
            conditions: [
                "direct",
                "continuation",
                "followup",
                "after_speaking",
                "moderation",
            ]
            .map(String::from)
            .to_vec(),
            score: Some(Score {
                total: 1.0,
                threshold: 0.9,
            }),
            judge: Some(Judged {
                reason: Some("r".into()),
                millis: Some(1000),
                unjudged: None,
            }),
            why: Some("rate_limited".into()),
        },
        Item::Decided {
            who: None,
            outcome: "record".into(),
            conditions: Vec::new(),
            score: None,
            judge: Some(Judged {
                reason: None,
                millis: Some(25_000),
                unjudged: Some("timeout".into()),
            }),
            why: Some("off".into()),
        },
        Item::Decided {
            who: None,
            outcome: "record".into(),
            conditions: Vec::new(),
            score: None,
            judge: Some(Judged {
                reason: None,
                millis: None,
                unjudged: Some("rate_full".into()),
            }),
            why: Some("quota_full".into()),
        },
        Item::Started {
            who: Some("a".into()),
            glance: false,
        },
        Item::Started {
            who: None,
            glance: true,
        },
        Item::Started {
            who: None,
            glance: false,
        },
        Item::Wrote {
            id: "x".into(),
            block: Block::Thinking,
            text: "t".into(),
            done: true,
        },
        Item::Wrote {
            id: "x".into(),
            block: Block::Text,
            text: "t".into(),
            done: true,
        },
        Item::Wrote {
            id: "x".into(),
            block: Block::Call("qq_quote".into()),
            text: "{}".into(),
            done: true,
        },
        Item::Result {
            tool: "t".into(),
            ok: true,
            summary: "s".into(),
        },
        Item::Result {
            tool: "t".into(),
            ok: false,
            summary: "s".into(),
        },
        Item::Delivered {
            text: "t".into(),
            images: 0,
        },
        Item::Delivered {
            text: String::new(),
            images: 2,
        },
        Item::Failed {
            why: "expired".into(),
        },
        Item::Failed {
            why: "rejected".into(),
        },
        Item::Failed {
            why: "timeout".into(),
        },
        Item::Failed {
            why: "disconnected".into(),
        },
        Item::Recalled { who: "a".into() },
        Item::Compacted,
        Item::Ended {
            reason: "interrupted".into(),
        },
        Item::Ended {
            reason: "error".into(),
        },
    ];
    let tool = |_: &str| None;
    for language in ["zh", "en", "ja"] {
        let texts = texts(language);
        for item in &items {
            let said = shown(
                &Line {
                    at: 0,
                    item: item.clone(),
                },
                0,
                &texts,
                &tool,
            );
            assert!(
                !said.text.contains("software/"),
                "{language} 少一句：{said:?}"
            );
            // 带下划线的编号哪种语言都不该露出来；英文的说法本来就是这些词，只查中日。
            let raw: &[&str] = match language {
                "en" => &["after_speaking", "rate_limited", "rate_full", "quota_full"],
                _ => &[
                    "reply",
                    "record",
                    "direct",
                    "after_speaking",
                    "rate_limited",
                    "expired",
                    "interrupted",
                ],
            };
            for word in raw {
                assert!(!said.text.contains(word), "{language} 词没换：{said:?}");
            }
        }
    }
}

#[test]
fn clock_is_local_and_wraps_the_day() {
    assert_eq!(clock(0, 480), "08:00:00");
    assert_eq!(clock(-1000, 0), "23:59:59");
    assert_eq!(clock(86_399_000, 60), "00:59:59");
}
