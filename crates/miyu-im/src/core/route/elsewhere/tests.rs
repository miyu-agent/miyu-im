//! 参数里写的场所（设计 13，`super::aimed`）：只认终端里能用、要说场所的那几件；`group`、`private` 至多一个，号照整数或者数字的字。

use serde_json::json;

use super::{Aimed, aimed};
use crate::chat::{Id, To};
use miyu_client::tooltext::Access;

use crate::toolset::{Call, Decl, Planned, Reach, ToolSet};

/// 一套假的：`history` 终端里能用、可写可不写，`members` 要说场所，`avatar` 不找场所，`mute` 只在场所里。
struct Set;

static TOOLS: [Decl; 4] = [
    Decl {
        name: "history",
        access: Access::Read,
        venues: &["local", "private", "group"],
    },
    Decl {
        name: "members",
        access: Access::Read,
        venues: &["local", "group"],
    },
    Decl {
        name: "avatar",
        access: Access::Read,
        venues: &["local", "private", "group"],
    },
    Decl {
        name: "mute",
        access: Access::Venue,
        venues: &["group"],
    },
];

impl ToolSet for Set {
    fn dir(&self) -> &'static str {
        "test"
    }
    fn tools(&self) -> &'static [Decl] {
        &TOOLS
    }
    fn answers(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &[]
    }
    fn plan(&self, call: Call) -> Planned {
        Planned::Now(call.answers.say("x", &[], false))
    }
    fn reach(&self, tool: &str) -> Reach {
        match tool {
            "avatar" => Reach::Anywhere,
            "history" => Reach::Either,
            _ => Reach::Chat,
        }
    }
}

#[test]
fn a_chat_named_by_group_or_private_is_read_and_both_or_junk_is_bad() {
    let group = |n: i64| Aimed::At(To::Group(Id::from(n)));
    assert_eq!(aimed(&Set, "members", &json!({})), Aimed::Here);
    assert_eq!(
        aimed(&Set, "members", &json!({"group": 130515298})),
        group(130515298)
    );
    // Telegram 的群号是负的（施工 I-220）。
    assert_eq!(
        aimed(&Set, "members", &json!({"group": -1001234567890_i64})),
        group(-1001234567890)
    );
    assert_eq!(
        aimed(&Set, "members", &json!({"group": "130515298"})),
        group(130515298)
    );
    assert_eq!(
        aimed(&Set, "history", &json!({"private": 2433516202_i64})),
        Aimed::At(To::Private(Id::from(2433516202_i64)))
    );
    for bad in [
        json!({"group": 1, "private": 2}),
        json!({"group": 0}),
        json!({"group": "abc"}),
        json!({"group": true}),
    ] {
        assert_eq!(aimed(&Set, "members", &bad), Aimed::Bad, "{bad}");
    }
}

#[test]
fn tools_that_do_not_go_to_a_chat_keep_their_own_group() {
    // 头像的 `group` 是「那个群的头像」，禁言只在场所里：都不当成去别的场所。
    assert_eq!(aimed(&Set, "avatar", &json!({"group": 5})), Aimed::Here);
    assert_eq!(aimed(&Set, "mute", &json!({"group": 5})), Aimed::Here);
}
