//! 谁叫的（施工 O-31 的 `Caller::read`，T-2 起 im 照它填驱动的工具的场景）：只认平台上的身份。

use miyu_client::ids::ExternalId;
use serde_json::json;

use super::{by_of, counted, open};

#[test]
fn the_caller_is_read_from_the_platform_identity_only() {
    let by = |by: serde_json::Value| by_of(&json!({"by": by}));
    let qq = |id: &str| ExternalId::parse(id).ok();
    assert_eq!(
        by(json!({"kind": "external", "id": "qq:40001", "role": "member"})),
        qq("qq:40001")
    );
    // 私聊里终端管理员记成本人：照经的平台身份认。
    assert_eq!(
        by(json!({"kind": "person", "via": "qq:10001"})),
        qq("qq:10001")
    );
    assert_eq!(by(json!({"kind": "person"})), None, "本机的人没有平台身份");
    assert_eq!(
        by(json!({"kind": "module", "id": "qq:10001"})),
        None,
        "别的种类不认"
    );
    assert_eq!(by_of(&json!({})), None);
}

#[test]
fn a_turn_counts_its_own_calls_and_a_new_turn_starts_over() {
    // 原来 `fetch_media` 的一轮取了几次（施工 O-33），T-3 起 im 照这个给驱动的每件工具数，挡不挡由驱动定。
    let mut kept = (Some(7), 0);
    for n in 0..4 {
        assert_eq!(counted(&mut kept, Some(7)), n);
    }
    assert_eq!(counted(&mut kept, Some(9)), 0, "换了一轮");
    assert_eq!(kept, (Some(9), 1));
    let mut idle = (None, 0);
    assert_eq!(counted(&mut idle, None), 0, "主线没在跑的也数");
    assert_eq!(counted(&mut idle, None), 1);
}

#[test]
fn only_the_owner_in_a_fully_open_session_is_open() {
    // 施工 LV-4（核心 TC-1）：主人本人叫的、完全放开、不是只读，三样都有才放开；少一格、旧核心不交的都算没放开。
    let call = |owner: bool, permission: serde_json::Value| {
        open(&json!({"owner": owner, "permission": permission}))
    };
    assert!(call(true, json!({"level": "full", "read_only": false})));
    assert!(call(true, json!({"level": "full"})), "没写只读的不算只读");
    assert!(
        !call(false, json!({"level": "full", "read_only": false})),
        "外人"
    );
    assert!(
        !call(true, json!({"level": "full", "read_only": true})),
        "只读"
    );
    assert!(
        !call(true, json!({"level": "workspace", "read_only": false})),
        "旧的级别"
    );
    assert!(!open(&json!({"owner": true})), "旧核心不交权限");
}
