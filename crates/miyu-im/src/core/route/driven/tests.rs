//! 谁叫的（施工 O-31 的 `Caller::read`，T-2 起 im 照它填驱动的工具的场景）：只认平台上的身份。

use miyu_client::ids::ExternalId;
use serde_json::json;

use super::by_of;

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
        by(json!({"kind": "person", "account": "admin", "via": "qq:10001"})),
        qq("qq:10001")
    );
    assert_eq!(
        by(json!({"kind": "person", "account": "admin"})),
        None,
        "本机的人没有平台身份"
    );
    assert_eq!(
        by(json!({"kind": "module", "id": "qq:10001"})),
        None,
        "别的种类不认"
    );
    assert_eq!(by_of(&json!({})), None);
}
