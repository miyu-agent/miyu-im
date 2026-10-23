//! 假 NapCat 回的群公告（施工 I-37，从 `answering.rs` 挪来：那个文件到了行数上限）。

use serde_json::{Value, json};

/// 假 NapCat 回的群公告（`_get_group_notice`，施工 I-37）：两条，照 NapCat 的样子（`message.text`）。
pub const NOTICES: [(i64, i64, &str); 2] = [
    (40001, 1_760_000_000, "群规：不刷屏"),
    (40004, 1_761_177_600, "周六十点东门集合"),
];

/// `_get_group_notice`（施工 I-37）：照 [`NOTICES`] 回。
pub fn group_notices(action: &Value) -> Value {
    let data: Vec<Value> = NOTICES
        .iter()
        .map(|(sender, time, text)| json!({"sender_id": sender, "publish_time": time, "message": {"text": text, "images": []}}))
        .collect();
    json!({"status": "ok", "retcode": 0, "data": data, "message": "", "wording": "", "echo": action["echo"]})
}
