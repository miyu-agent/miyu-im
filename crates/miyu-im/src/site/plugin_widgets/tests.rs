//! 页面交的场所换成场所编号（`super::venue_id`）：群、私聊照平台拼，写错的不收。

use serde_json::json;

use super::venue_id;
use crate::chat::Platform;

#[test]
fn page_venues_turn_into_venue_ids() {
    let qq = Platform::new("qq");
    assert_eq!(
        venue_id(qq, &json!({"type": "group", "id": "777"})).as_deref(),
        Some("qq:group:777")
    );
    assert_eq!(
        venue_id(qq, &json!({"type": "private", "id": "20004"})).as_deref(),
        Some("qq:private:20004")
    );
    for wrong in [
        json!({"type": "channel", "id": "1"}),
        json!({"type": "group", "id": "abc"}),
        json!({"type": "group", "id": 777}),
        json!({"type": "group"}),
        json!(null),
    ] {
        assert_eq!(venue_id(qq, &wrong), None, "{wrong}");
    }
}
