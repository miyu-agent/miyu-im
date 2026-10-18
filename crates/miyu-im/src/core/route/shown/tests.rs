//! 工具结果里给人看的文件怎么认（施工 O-46）：只认工具结果里 `kind` 是 `file.shown`、三格齐的；常见四种图片发成图片段。

use serde_json::json;

use super::*;

#[test]
fn files_shown_are_read_from_tool_results_only() {
    let png = json!({"kind": "file.shown", "path": "/w/a.png", "blob": "sha256:a", "name": "a.png", "media_type": "image/png", "size": 3, "width": 1, "height": 1});
    let pdf = json!({"kind": "file.shown", "path": "/w/b.pdf", "blob": "sha256:b", "name": "b.pdf", "media_type": "application/pdf", "size": 9});
    // 三格齐、可不是 `file.shown` 的不认。
    let other = json!({"kind": "file.attached", "blob": "sha256:d", "name": "d.png", "media_type": "image/png"});
    let raw = json!({"kind": "tool.result", "body": {"effects": [png, {"kind": "file.read", "path": "/w/x"}, other, pdf, {"kind": "file.shown", "blob": "sha256:c"}]}});
    let files = shown_in(&raw);
    assert_eq!(
        files
            .iter()
            .map(|file| (file.name.as_str(), file.is_image()))
            .collect::<Vec<_>>(),
        [("a.png", true), ("b.pdf", false)]
    );
    assert_eq!(files[0].blob, "sha256:a");
    let message = json!({"kind": "message.assistant", "body": {"effects": [png]}});
    assert!(shown_in(&message).is_empty(), "只认工具结果");
    assert!(shown_in(&json!({"kind": "tool.result", "body": {}})).is_empty());
}

#[test]
fn only_the_four_common_images_go_as_pictures() {
    let file = |media_type: &str| Shown {
        blob: "sha256:a".to_string(),
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
