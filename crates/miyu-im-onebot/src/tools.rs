//! OneBot（QQ）给 AI 的工具（施工 T-1 起，2026-10-20 项目主人定：不同平台的工具最终调的接口可能完全不一样，平台的工具在驱动里，
//! im 不提供工具）：照 im 的口子（`miyu_im::toolset::ToolSet`）交一套，说明在包文件夹的 `data/onebot/tools/`，答的话在
//! `data/onebot/tool-results/`。im 照会话填好场景交过来，这里认参数、去 QQ 上办。
//!
//! 施工 T-1 挪过来的是头像、群成员（`lookup`，施工 O-54），T-2 是撤回、禁言、戳一戳（`acting`、`target`，施工 O-31）。

use std::time::Duration;

use miyu_client::tooltext::Access;
use miyu_im::toolset::{Call, Decl, Planned, ToolSet};
use miyu_im::tuning::Tuning;

mod acting;
mod lookup;
mod target;

/// 撤回（施工 O-31）：撤叫她做的那条引用的那一条，或者她给的编号。
pub const RECALL: &str = "recall";

/// 禁言（施工 O-31）：只给群。
pub const MUTE: &str = "mute";

/// 戳一戳（施工 O-31）。
pub const POKE: &str = "poke";

/// 取头像（施工 O-54）：一个人的、一个群的，存进工作区。
pub const AVATAR: &str = "avatar";

/// 查群成员（施工 O-54）：只给群。
pub const MEMBERS: &str = "members";

/// 私聊、群都给。
const VENUES: &[&str] = &["private", "group"];

/// 有哪几件，照登记的先后。撤回、禁言、戳一戳在平台上留下后果，`avatar` 在平台上取东西、写进这个会话的工作区，都是在场所里做的事
/// （`venue`）；`members` 只读。
const TOOLS: [Decl; 5] = [
    Decl {
        name: RECALL,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: MUTE,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: POKE,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: AVATAR,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: MEMBERS,
        access: Access::Read,
        venues: &["group"],
    },
];

/// 答的话：名字和认的字段（`failed`、`unanswered`、`unreachable` 几件共用）。
const ANSWERS: [(&str, &[&str]); 18] = [
    ("recalled", &[]),
    ("muted", &["who", "duration"]),
    ("unmuted", &["who"]),
    ("poked", &["who"]),
    ("no-quote", &[]),
    ("many-targets", &[]),
    ("bad-seconds", &[]),
    ("not-found", &[]),
    ("unknown", &["name"]),
    ("saved", &["path"]),
    ("unfetched", &["detail"]),
    ("not-saved", &["detail"]),
    ("members", &["shown", "total", "list"]),
    ("no-members", &[]),
    ("no-target", &[]),
    ("failed", &["detail"]),
    ("unanswered", &[]),
    ("unreachable", &[]),
];

/// 照 `bridge.json` 的参数造的一套。
#[derive(Debug, Clone)]
pub struct QqTools {
    /// 头像的地址模板：一个人的、一个群的（`avatar_user_url`、`avatar_group_url`，`{id}` 换成号）。
    pub(crate) avatars: (String, String),
    /// 群成员最多列几个（`members_shown`）。
    pub(crate) members: usize,
    /// 下东西最多等多久（`fetch_seconds`）。
    pub(crate) wait: Duration,
}

impl QqTools {
    /// 照 `bridge.json` 的参数 `tuning` 造。
    pub fn new(tuning: &Tuning) -> QqTools {
        QqTools {
            avatars: (
                tuning.avatar_user_url.clone(),
                tuning.avatar_group_url.clone(),
            ),
            members: tuning.members_shown,
            wait: tuning.fetch(),
        }
    }
}

impl ToolSet for QqTools {
    fn dir(&self) -> &'static str {
        "onebot"
    }

    fn tools(&self) -> &'static [Decl] {
        &TOOLS
    }

    fn answers(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &ANSWERS
    }

    fn plan(&self, call: Call) -> Planned {
        match call.tool.as_str() {
            RECALL | MUTE | POKE => acting::acting(call),
            AVATAR => lookup::avatar(self, call),
            MEMBERS => lookup::members(self, call),
            // im 只交这一套里有的：照说不会。
            _ => Planned::Now(call.answers.say("unreachable", &[], true)),
        }
    }
}
