//! 事件整理成行（施工 I-160 上）：样子照 11-05 主群、私聊日志里的真事件。

use serde_json::{Value, json};

use super::{Block, Extra, Item, Judged, Score, lines};

fn items(event: Value) -> Vec<Item> {
    let names = |seq: u64| (seq == 7).then(|| "小林".to_string());
    lines(&event, "L", &names)
        .into_iter()
        .map(|line| line.item)
        .collect()
}

#[test]
fn a_message_shows_who_said_what_and_what_came_with_it() {
    let event = json!({"at": "2026-11-05T11:04:44.000Z", "kind": "message.user", "body": {
        "blocks": [{"type": "text", "text": "这表情包哪里来的"}],
        "venue": {"name": "Shorin", "media": [{"kind": "image"}, {"kind": "image"}, {"kind": "sticker"}]}}});
    assert_eq!(
        items(event),
        [Item::Heard {
            who: "Shorin".into(),
            text: "这表情包哪里来的".into(),
            extras: vec![Extra::Images(2), Extra::Sticker]
        }]
    );
    // 只转发的：占位不重复，写里面几条。
    let event = json!({"kind": "message.user", "body": {"blocks": [{"type": "text", "text": "[forward]"}],
        "venue": {"name": "Claude", "forwards": [{"nodes": [{}, {}, {}]}]}}});
    assert_eq!(
        items(event),
        [Item::Heard {
            who: "Claude".into(),
            text: String::new(),
            extras: vec![Extra::Forward(3)]
        }]
    );
}

#[test]
fn a_decision_keeps_conditions_score_and_the_judge() {
    let event = json!({"kind": "ext.im.chat.decided", "body": {"conditions": [{"bonus": 0.15, "kind": "direct"}],
        "judge": {"answer": {"reason": "直接@了她"}, "millis": 6496, "mode": "reply", "tries": 1},
        "msgs": [7], "outcome": "reply", "score": {"total": 1.325, "threshold": 0.9}}});
    assert_eq!(
        items(event),
        [Item::Decided {
            who: Some("小林".into()),
            outcome: "reply".into(),
            conditions: vec!["direct".into()],
            score: Some(Score {
                total: 1.325,
                threshold: 0.9
            }),
            judge: Some(Judged {
                reason: Some("直接@了她".into()),
                millis: Some(6496),
                unjudged: None
            }),
            why: None,
        }]
    );
    // 超时的、进站链挡下的。
    let timeout = json!({"kind": "ext.im.chat.decided", "body": {"conditions": [{"kind": "after_speaking"}],
        "judge": {"millis": 15147, "unjudged": "timeout"}, "msgs": [8], "outcome": "record"}});
    let got = items(timeout);
    let [
        Item::Decided {
            judge: Some(judge),
            who: None,
            ..
        },
    ] = got.as_slice()
    else {
        panic!("{got:?}")
    };
    assert_eq!(judge.unjudged.as_deref(), Some("timeout"));
    let limited = json!({"kind": "ext.im.chat.decided", "body": {"inbound": "record_only", "msgs": [7], "outcome": "record", "why": "rate_limited"}});
    let got = items(limited);
    let [
        Item::Decided {
            why: Some(why),
            conditions,
            ..
        },
    ] = got.as_slice()
    else {
        panic!("{got:?}")
    };
    assert_eq!((why.as_str(), conditions.len()), ("rate_limited", 0));
}

#[test]
fn blocks_get_one_id_while_written_and_when_done() {
    let start =
        json!({"kind": "model.delta", "body": {"seen": 44, "index": 0, "start": "reasoning"}});
    let more = json!({"kind": "model.delta", "body": {"seen": 44, "index": 0, "text": "用户要"}});
    let end = json!({"kind": "model.delta", "body": {"seen": 44, "index": 0, "end": true}});
    assert_eq!(
        items(start),
        [Item::Wrote {
            id: "L:44:0".into(),
            block: Block::Thinking,
            text: String::new(),
            done: false
        }]
    );
    assert_eq!(
        items(more),
        [Item::Appended {
            id: "L:44:0".into(),
            text: "用户要".into()
        }]
    );
    assert!(items(end).is_empty());
    let done = json!({"kind": "message.assistant", "body": {"seen": 44, "blocks": [
        {"type": "reasoning", "text": "用户要 WinRAR 官网"},
        {"type": "text", "text": "https://www.rarlab.com"},
        {"type": "tool_call", "name": "qq_quote", "args": "{\"msg\": \"1\"}"}]}});
    assert_eq!(
        items(done),
        [
            Item::Wrote {
                id: "L:44:0".into(),
                block: Block::Thinking,
                text: "用户要 WinRAR 官网".into(),
                done: true
            },
            Item::Wrote {
                id: "L:44:1".into(),
                block: Block::Text,
                text: "https://www.rarlab.com".into(),
                done: true
            },
            Item::Wrote {
                id: "L:44:2".into(),
                block: Block::Call("qq_quote".into()),
                text: "{\"msg\": \"1\"}".into(),
                done: true
            },
        ]
    );
}

#[test]
fn a_result_is_summed_up_and_errors_kept_longer() {
    let many = (1..=5)
        .map(|n| format!("[{n}] 第 {n} 条"))
        .collect::<Vec<_>>()
        .join("\n");
    let ok = json!({"kind": "tool.result", "body": {"status": "ok", "display_name": {"zh": "网络搜索"},
        "blocks": [{"type": "text", "text": many}]}});
    assert_eq!(
        items(ok),
        [Item::Result {
            tool: "网络搜索".into(),
            ok: true,
            summary: "[1] 第 1 条 (+4)".into()
        }]
    );
    let picture = json!({"kind": "tool.result", "body": {"status": "ok", "display_name": {"zh": "获取图片和文件"},
        "blocks": [{"type": "image", "width": 1220, "height": 1941}]}});
    assert_eq!(
        items(picture),
        [Item::Result {
            tool: "获取图片和文件".into(),
            ok: true,
            summary: "1220×1941".into()
        }]
    );
    let long = "错".repeat(400);
    let error = json!({"kind": "tool.result", "body": {"status": "error", "blocks": [{"type": "text", "text": long}]}});
    let got = items(error);
    let [
        Item::Result {
            ok: false, summary, ..
        },
    ] = got.as_slice()
    else {
        panic!("{got:?}")
    };
    assert_eq!(
        summary.chars().count(),
        super::ERROR_CHARS + 1,
        "截到 300 字加一个省略号"
    );
}

#[test]
fn turns_sending_and_the_rest() {
    let started = json!({"kind": "turn.started", "cause": "q:1/respond", "body": {"trigger": 7}});
    assert_eq!(
        items(started),
        [Item::Started {
            who: Some("小林".into()),
            glance: false
        }]
    );
    let glance =
        json!({"kind": "turn.started", "cause": "glance/t117/respond", "body": {"trigger": 9}});
    assert_eq!(
        items(glance),
        [Item::Started {
            who: None,
            glance: true
        }]
    );
    let sent = json!({"kind": "venue.delivered", "body": {"text": "", "images": ["sha256:a"]}});
    assert_eq!(
        items(sent),
        [Item::Delivered {
            text: String::new(),
            images: 1
        }]
    );
    assert_eq!(
        items(json!({"kind": "ext.im.venues.failed", "body": {"why": "expired"}})),
        [Item::Failed {
            why: "expired".into()
        }]
    );
    // 施工 I-200：模型请求出错的写出模型和端点的原话（太长的截断）；成了的、作废重来的不出行。
    let failed = json!({"kind": "model.called", "body": {"result": "error", "model": "p/m",
        "error": {"class": "other", "message": "HTTP 404: no such model"}}});
    assert_eq!(
        items(failed),
        [Item::Errored {
            model: "p/m".into(),
            message: "HTTP 404: no such model".into()
        }]
    );
    assert_eq!(
        items(json!({"kind": "model.called", "body": {"result": "superseded", "model": "p/m"}})),
        []
    );
    assert_eq!(
        items(json!({"kind": "model.called", "body": {"model": "p/m", "input": 1}})),
        []
    );
    assert_eq!(
        items(json!({"kind": "turn.ended", "body": {"reason": "completed"}})),
        []
    );
    assert_eq!(
        items(json!({"kind": "turn.ended", "body": {"reason": "interrupted"}})),
        [Item::Ended {
            reason: "interrupted".into()
        }]
    );
    assert_eq!(
        items(json!({"kind": "context.injected", "body": {}})),
        [],
        "别的不出行"
    );
}
