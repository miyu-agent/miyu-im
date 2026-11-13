//! 工具结果里给人看的文件怎么认（施工 O-46；I-233 起照核心 SW-1 的样子：没有 `blob`，有 `path`、`size`，照调用编号读）：只认工具
//! 结果里 `kind` 是 `file.shown`、几格齐的；常见四种图片发成图片段。

use serde_json::json;

use super::*;

#[test]
fn files_shown_are_read_from_tool_results_only() {
    let png = json!({"kind": "file.shown", "path": "/w/a.png", "name": "a.png", "media_type": "image/png", "size": 3, "modified": 1, "width": 1, "height": 1});
    let pdf = json!({"kind": "file.shown", "path": "/w/b.pdf", "name": "b.pdf", "media_type": "application/pdf", "size": 9, "modified": 1});
    // 几格齐、可不是 `file.shown` 的不认；缺了路径、大小的不认。
    let other = json!({"kind": "file.attached", "blob": "sha256:d", "name": "d.png", "media_type": "image/png"});
    let sizeless = json!({"kind": "file.shown", "path": "/w/c.png", "name": "c.png", "media_type": "image/png"});
    let pathless =
        json!({"kind": "file.shown", "name": "e.png", "media_type": "image/png", "size": 1});
    let raw = json!({"kind": "tool.result", "body": {"call_id": "call_12_1", "effects": [png, {"kind": "file.read", "path": "/w/x"}, other, pdf, sizeless, pathless]}});
    let files = shown_in(&raw);
    assert_eq!(
        files
            .iter()
            .map(|file| (file.name.as_str(), file.is_image()))
            .collect::<Vec<_>>(),
        [("a.png", true), ("b.pdf", false)]
    );
    assert_eq!(
        files[0].source,
        Source::Shown {
            call: "call_12_1".to_string(),
            size: 3
        }
    );
    // 没有调用编号的读不了（`session.shown` 照它找）：不认。
    let callless = json!({"kind": "tool.result", "body": {"effects": [png.clone()]}});
    assert!(shown_in(&callless).is_empty());
    let message =
        json!({"kind": "message.assistant", "body": {"call_id": "call_12_1", "effects": [png]}});
    assert!(shown_in(&message).is_empty(), "只认工具结果");
    assert!(shown_in(&json!({"kind": "tool.result", "body": {}})).is_empty());
}

#[test]
fn only_the_four_common_images_go_as_pictures() {
    let file = |media_type: &str| Shown {
        source: Source::Shown {
            call: "call_1_1".to_string(),
            size: 1,
        },
        name: "x".to_string(),
        media_type: media_type.to_string(),
    };
    for image in ["image/png", "image/jpeg", "image/gif", "image/webp"] {
        assert!(file(image).is_image(), "{image}");
    }
    for other in [
        "image/svg+xml",
        "application/pdf",
        "text/plain",
        "text/html",
        "application/octet-stream",
    ] {
        assert!(!file(other).is_image(), "{other}");
    }
}
