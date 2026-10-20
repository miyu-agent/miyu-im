//! 假 NapCat 照实测的毛病踢人（施工 I-17，旧版 08-30 实测）：踢 [`KICKED_ANYWAY`] 踢成了，却回 `status=failed, retcode=100`；之后问他
//! 的群成员信息回失败（人不在群里了）。别的动作不管，交回空的照常回。

use std::collections::HashSet;

use serde_json::{Value, json};

/// 踢成了却回失败的那一个号。
pub const KICKED_ANYWAY: i64 = 40008;

/// 照毛病回 `action`：踢 [`KICKED_ANYWAY`] 的记进走了的 `gone`、回失败；问走了的人的回失败。别的是空的。
pub fn kicking(action: &Value, gone: &mut HashSet<i64>) -> Option<Value> {
    let user = action["params"]["user_id"].as_i64()?;
    let failed = |message: &str| json!({"status": "failed", "retcode": 100, "data": null, "message": message, "wording": "", "echo": action["echo"]});
    match action["action"].as_str()? {
        "set_group_kick" if user == KICKED_ANYWAY => {
            gone.insert(user);
            Some(failed("kick member failed: u_x9Kq2"))
        }
        "get_group_member_info" if gone.contains(&user) => Some(failed("群成员不存在")),
        _ => None,
    }
}
