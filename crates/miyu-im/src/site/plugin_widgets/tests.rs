//! 页面交的场所换成场所编号（`super::venue_id`）：群、私聊照平台、机器人号拼（施工 I-81），写错的不收。

use serde_json::json;

use super::venue_id;
use crate::chat::{Id, Platform};

#[test]
fn page_venues_turn_into_venue_ids() {
    let qq = Platform::new("qq");
    let bot = Id::from(10001);
    assert_eq!(
        venue_id(qq, &bot, &json!({"type": "group", "id": "777"})).as_deref(),
        Some("qq:10001:group:777")
    );
    assert_eq!(
        venue_id(qq, &bot, &json!({"type": "private", "id": "20004"})).as_deref(),
        Some("qq:10001:private:20004")
    );
    for wrong in [
        json!({"type": "channel", "id": "1"}),
        json!({"type": "group", "id": "a b"}),
        json!({"type": "group", "id": 777}),
        json!({"type": "group"}),
        json!(null),
    ] {
        assert_eq!(venue_id(qq, &bot, &wrong), None, "{wrong}");
    }
}
