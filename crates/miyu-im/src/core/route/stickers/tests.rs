//! 工具结果里她发出去的图（施工 O-51）：只认工具结果的 `sent`；名字照 `name`、`alt`，都没有的照是不是表情包写；`blob` 不是字的不要。

use serde_json::json;

use super::{Sent, sent_in};

#[test]
fn only_a_tool_results_sent_images_are_taken() {
    let raw = json!({"kind": "tool.result", "body": {"sent": [
        {"blob": "a", "media_type": "image/gif", "width": 1, "height": 1, "sticker": true, "alt": "doge"},
        {"blob": "b", "media_type": "image/png", "width": 1, "height": 1, "name": "cat.png", "alt": "猫"},
        {"blob": "c", "media_type": "image/png", "width": 1, "height": 1, "sticker": true},
        {"blob": "d", "media_type": "image/png", "width": 1, "height": 1},
        {"blob": 5, "media_type": "image/png", "width": 1, "height": 1},
    ]}});
    let sent = |blob: &str, name: &str, sticker: bool| Sent {
        blob: blob.to_string(),
        name: name.to_string(),
        sticker,
        voice: None,
    };
    assert_eq!(
        sent_in(&raw),
        [
            sent("a", "doge", true),
            sent("b", "cat.png", false),
            sent("c", "sticker", true),
            sent("d", "image", false),
        ]
    );
    assert!(sent_in(&json!({"kind": "tool.call", "body": {"sent": [{"blob": "a"}]}})).is_empty());
    assert!(sent_in(&json!({"kind": "tool.result", "body": {}})).is_empty());
}

#[test]
fn a_spoken_voice_in_sent_is_a_voice_and_a_wrong_one_is_dropped() {
    // 施工 I-250（核心 SD-1）：`voice: true` 的那一段照语音发，格式照写的；不是音频的不认。
    let raw = serde_json::json!({"kind": "tool.result", "body": {"sent": [
        {"blob": "sha256:a", "media_type": "audio/mpeg", "voice": true, "name": "say-01.mp3", "duration_ms": 1200},
        {"blob": "sha256:b", "media_type": "audio/wav", "voice": true},
        {"blob": "sha256:c", "media_type": "image/png", "voice": true},
        {"blob": "sha256:d", "alt": "图"},
    ]}});
    let sent = sent_in(&raw);
    let shown: Vec<_> = sent
        .iter()
        .map(|one| (one.blob.as_str(), one.name.as_str(), one.voice.as_deref()))
        .collect();
    assert_eq!(
        shown,
        [
            ("sha256:a", "say-01.mp3", Some("audio/mpeg")),
            ("sha256:b", "voice", Some("audio/wav")),
            ("sha256:d", "图", None),
        ]
    );
}
