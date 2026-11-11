//! OneBot（QQ）给 AI 的工具（施工 T-1 起，2026-10-20 项目主人定：不同平台的工具最终调的接口可能完全不一样，平台的工具在驱动里，
//! im 不提供工具）：照 im 的口子（`miyu_im::toolset::ToolSet`）交一套，说明在包文件夹的 `data/onebot/tools/`，答的话在
//! `data/onebot/tool-results/`。im 照会话填好场景交过来，这里认参数、去 QQ 上办。
//!
//! 施工 T-1 挪过来的是头像、群成员（`lookup`，施工 O-54），T-2 是撤回、禁言、戳一戳（`acting`、`target`，施工 O-31），T-3 是取图片
//! 文件（`fetch`，施工 O-33），T-4 是不回复、@ 人、主动发消息（施工 O-26、O-55、O-56；不回复、主动发消息由 im 照它的口子办，
//! `mention` 认成员在这里）。挪完 im 不提供工具。

use std::time::Duration;

use miyu_client::tooltext::Access;
use miyu_im::toolset::common::{glance, history, ranking, send_file, usage};
use miyu_im::toolset::{Answers, Call, Decl, Effect, Planned, Reach, ToolSet, send_request};
use miyu_im::tuning::Tuning;
use serde_json::{Map, Value};

mod acting;
mod args;
mod card;
mod contacts;
mod fetch;
mod group_files;
mod log;
mod lookup;
mod mention;
mod moderate;
mod notices;
mod quote;
mod react;
mod target;
#[cfg(test)]
mod tests;

/// 「这一轮不说话」（施工 O-26，「提供者和不说话」第 3 条）：im 的读的一头当场答，她的回复里有这件的调用块，这一轮就不发了。
pub const SKIP_REPLY: &str = "qq_skip_reply";

/// @ 指定的人（施工 O-55）：只给群。
pub const MENTION: &str = "qq_mention";

/// 主动发消息（施工 O-56）：发给她的好友、她在的群。
pub const CHAT_SEND: &str = "qq_send";

/// 撤回（施工 O-31）：撤叫她做的那条引用的那一条，或者她给的编号。
pub const RECALL: &str = "qq_recall";

/// 禁言（施工 O-31）：只给群。
pub const MUTE: &str = "qq_mute";

/// 戳一戳（施工 O-31）。
pub const POKE: &str = "qq_poke";

/// 踢人（施工 Q-2，群管理）：只给群。
pub const KICK: &str = "qq_kick";

/// 查管理记录（施工 Q-4，群管理）：只给群。
pub const MODERATION_LOG: &str = "qq_moderation_log";

/// 通讯录（施工 I-12）：私聊、群都给。
pub const CONTACTS: &str = "qq_contacts";

/// 群文件（施工 I-12）：只给群。
pub const GROUP_FILES: &str = "qq_group_files";

/// 读群公告（施工 I-37）：只给群。
pub const NOTICES: &str = "qq_notices";

/// 改群名片（施工 I-36）：只给群。
pub const SET_CARD: &str = "qq_set_card";

/// 引用指定的一条（施工 I-31）：私聊、群都给。
pub const QUOTE: &str = "qq_quote";

/// 给消息贴表情（施工 I-12）：只给群。
pub const REACT: &str = "qq_react";

/// 发言排行（施工 I-20）：只给群。
pub const ACTIVITY_RANKING: &str = "qq_activity_ranking";

/// 查用量（施工 I-206）：只给群，这个群、某个人一段时间里用了多少 token、花了多少。
pub const TOKEN_USAGE: &str = "qq_token_usage";

/// 查聊天记录（施工 C-1 下）：私聊、群都给，只读；跨场所要终端管理员叫的那一轮。
pub const CHAT_HISTORY: &str = "qq_chat_history";

/// 取图、视频、文件（施工 O-33）：私聊、群都给。
pub const FETCH_MEDIA: &str = "qq_fetch_media";

/// 往这个场所传她工作区里的一份文件（施工 I-130）：私聊、群都给，路径交 NapCat 自己读、传。
pub const SEND_FILE: &str = "qq_send_file";

/// 定下一次什么时候看一眼（设计 05，施工 I-89 下）：私聊、群都给，只在开着时不时看一眼的场所里有（`GLANCE_PLUGIN`）。
pub const GLANCE: &str = "qq_glance";

/// 取头像（施工 O-54）：一个人的、一个群的，存进工作区。
pub const AVATAR: &str = "qq_avatar";

/// 查群成员（施工 O-54）：只给群。
pub const MEMBERS: &str = "qq_members";

/// 查询信息插件管的八件（施工 I-44、I-91）：只读，插件关着的场所没有它们。
pub const LOOKUPS: [&str; 9] = [
    CHAT_HISTORY,
    FETCH_MEDIA,
    AVATAR,
    MEMBERS,
    NOTICES,
    GROUP_FILES,
    CONTACTS,
    ACTIVITY_RANKING,
    TOKEN_USAGE,
];

/// 私聊、群都给。
const VENUES: &[&str] = &["private", "group"];

/// 本机的会话也给（施工 I-40，项目主人 10-26：终端、网页里叫她去 QQ 上给人发一句；设计 13 起查看那几件也给，终端里照参数
/// `group`、`private` 找那个场所）：核心照对外发消息问人。
const EVERYWHERE: &[&str] = &["local", "private", "group"];

/// 有哪几件，照登记的先后。撤回、禁言、戳一戳在平台上留下后果，`avatar` 在平台上取东西、写进这个会话的工作区，都是在场所里做的事
/// （`venue`）；`members` 只读。
const TOOLS: [Decl; 22] = [
    Decl {
        name: SKIP_REPLY,
        access: Access::Read,
        venues: VENUES,
    },
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
        name: KICK,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: MODERATION_LOG,
        access: Access::Read,
        venues: &["group"],
    },
    Decl {
        name: POKE,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: FETCH_MEDIA,
        access: Access::Venue,
        venues: EVERYWHERE,
    },
    Decl {
        name: AVATAR,
        access: Access::Venue,
        venues: EVERYWHERE,
    },
    Decl {
        name: MEMBERS,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: MENTION,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: NOTICES,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: SET_CARD,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: QUOTE,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: CHAT_SEND,
        access: Access::Venue,
        venues: EVERYWHERE,
    },
    Decl {
        name: CONTACTS,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: GROUP_FILES,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: REACT,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: ACTIVITY_RANKING,
        access: Access::Read,
        venues: &["group"],
    },
    Decl {
        name: TOKEN_USAGE,
        access: Access::Read,
        venues: &["group"],
    },
    Decl {
        name: CHAT_HISTORY,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: GLANCE,
        access: Access::Read,
        venues: VENUES,
    },
    Decl {
        name: SEND_FILE,
        access: Access::Venue,
        venues: VENUES,
    },
];

/// 答的话：名字和认的字段（`failed`、`unanswered`、`unreachable` 几件共用）。
const ANSWERS: [(&str, &[&str]); 71] = [
    ("uploading", &["name", "size"]),
    ("not-local", &[]),
    ("no-file", &["path"]),
    ("outside", &["path"]),
    ("link", &["path"]),
    ("unsendable", &[]),
    ("skipped", &[]),
    ("glance-set", &["minutes"]),
    ("activity", &["count", "days", "list"]),
    ("no-activity", &["days"]),
    ("usage", &["hours", "tokens", "requests", "cost", "list"]),
    ("usage-user", &["hours", "who", "tokens", "cost"]),
    ("no-usage", &["hours"]),
    ("no-usage-user", &["hours", "who"]),
    ("mentioned", &["who"]),
    ("not-members", &["who"]),
    ("quoted", &[]),
    ("renamed", &["who", "card"]),
    ("renamed-me", &["card"]),
    ("notices", &["count", "list"]),
    ("no-notices", &[]),
    ("history", &["count"]),
    ("no-history", &[]),
    ("history-admin-only", &[]),
    ("managers-only", &[]),
    ("group-file-deleted", &[]),
    ("sent", &[]),
    ("no-item", &["count"]),
    ("not-fetchable", &[]),
    ("too-many", &["count"]),
    ("recalled", &[]),
    ("muted", &["who", "duration"]),
    ("unmuted", &["who"]),
    ("poked", &["who"]),
    ("no-quote", &[]),
    ("many-targets", &[]),
    ("bad-seconds", &["min", "max"]),
    ("admin-off", &[]),
    ("kicked", &["who"]),
    ("kicked-blocked", &["who"]),
    ("log-events", &["shown", "total"]),
    ("log-ranking", &["period"]),
    ("log-empty", &[]),
    ("ranking-off", &[]),
    ("contacts", &["shown", "total", "kind", "list"]),
    ("no-contacts", &[]),
    ("group-files", &["shown", "total", "list"]),
    ("no-group-files", &[]),
    ("reacted", &[]),
    ("not-found", &[]),
    ("saved", &["path"]),
    ("unfetched", &["detail"]),
    ("not-saved", &["detail"]),
    ("members", &["shown", "total", "list"]),
    ("no-members", &[]),
    ("no-target", &[]),
    ("no-recipient", &[]),
    ("failed", &["detail"]),
    ("unanswered", &[]),
    ("unreachable", &[]),
    // 施工 I-81 下下：本机的会话里主动发消息，同一个平台连着几个号又没写、写的没连着的，说有哪几个号。
    ("which-bot", &["bots"]),
    // 终端里用场所工具（设计 13）：结果前头写是哪个场所；没说、说错、那个场所关着、别人叫的想去别的场所。
    ("chat-in-group", &["group"]),
    ("chat-in-private", &["user"]),
    ("chat-needed", &[]),
    ("chat-bad", &[]),
    ("chat-off", &[]),
    ("only-here", &[]),
    // 施工 I-17：参数写错的说哪个、私聊里调到只给群的、群友叫的踢人要确认、不踢群主管理员、一次几个人的头一行。
    ("bad-args", &["detail"]),
    ("group-only", &[]),
    ("protected", &["who"]),
    ("batch", &["done", "total"]),
];

/// 驱动带的插件给场所会话记一句的模板：名字和认的字段（施工 Q-3）。
const FACTS: [(&str, &[&str]); 4] = [
    ("muted-by", &["who", "by", "duration"]),
    ("spam-muted", &["who", "duration"]),
    ("unmuted-by", &["who", "by"]),
    ("kicked-by", &["who", "by"]),
];

/// 驱动带的插件问模型的提示词模板：名字和认的字段（施工 Q-7）。
const PROMPTS: [(&str, &[&str]); 2] = [
    ("approval-system", &[]),
    ("approval-request", &["rule", "request"]),
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
    /// `fetch_media` 一轮最多取几次（`fetch_per_turn`）。
    pub(crate) per_turn: usize,
    /// 一张图最多多少字节（`image_max_mb`，施工 I-23）：`fetch_media` 取的图、头像照它。
    pub(crate) image_limit: usize,
    /// 传文件最多等多久（`upload_seconds`，施工 I-130）：`outbox/` 里比它还老的照没删掉的清。
    pub(crate) upload: Duration,
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
            per_turn: tuning.fetch_per_turn,
            image_limit: tuning.image_limit(),
            upload: tuning.upload(),
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

    fn facts(&self) -> &'static [(&'static str, &'static [&'static str])] {
        // 群管理同步别的管理员做的（施工 Q-3，`admin::sync`）、刷屏处理（施工 Q-5，`admin::spam`）。
        &FACTS
    }

    fn prompts(&self) -> &'static [(&'static str, &'static [&'static str])] {
        // 入群审批（施工 Q-7，`admin::approval`）。
        &PROMPTS
    }

    fn silence(&self) -> Option<&'static str> {
        Some(SKIP_REPLY)
    }

    fn reach(&self, tool: &str) -> Reach {
        // 终端里用（设计 13）：通讯录、头像不找场所；查聊天记录不写是查全部；别的要说是哪个群、哪个私聊。
        match tool {
            // 发一句的 `group`、`user` 就是发到哪，自己认（施工 I-40），不当成去别的场所。
            CONTACTS | AVATAR | CHAT_SEND => Reach::Anywhere,
            CHAT_HISTORY => Reach::Either,
            _ => Reach::Chat,
        }
    }

    fn plugin(&self, tool: &str) -> Option<&'static str> {
        // 群管理的三件（施工 S-1 上c，核心 O-60）、查询信息的八件（施工 I-44、I-91）：插件在这个场所关着的，工具表里没有它们。
        match tool {
            MUTE | KICK | MODERATION_LOG => Some(crate::admin::ID),
            GLANCE => Some(miyu_im::toolset::GLANCE_PLUGIN),
            tool if LOOKUPS.contains(&tool) => Some(crate::lookups::ID),
            _ => None,
        }
    }

    fn offers(&self, tool: &str, config: &Map<String, Value>) -> bool {
        // 群管理的工具照它的开关（施工 Q-1、Q-2），查询信息照它的总开关（施工 I-44、I-91）。
        match tool {
            MUTE => crate::admin::offers_mute(config),
            KICK => crate::admin::offers_kick(config),
            MODERATION_LOG => crate::admin::offers_log(config),
            tool if LOOKUPS.contains(&tool) => crate::lookups::offers(config),
            _ => true,
        }
    }

    fn local(&self, tool: &str, args: &Value, answers: &Answers) -> Option<Planned> {
        // 本机的会话只有主动发消息（施工 I-40）：同场所会话里那一条路，交给 im 发。
        (tool == CHAT_SEND).then(|| match send_request(args) {
            Some((to, text)) => Planned::Host(Effect::Send { to, text }),
            None => Planned::Now(answers.say("no-recipient", &[], true)),
        })
    }

    fn plan(&self, call: Call) -> Planned {
        match call.tool.as_str() {
            RECALL | MUTE | POKE | KICK => acting::acting(call),
            MODERATION_LOG => log::log(call),
            CONTACTS => contacts::contacts(self.members, call),
            GROUP_FILES => group_files::group_files(self, call),
            REACT => react::react(call),
            ACTIVITY_RANKING => ranking::ranking(&call),
            TOKEN_USAGE => usage::usage(call, "user must be a QQ number or a name"),
            GLANCE => glance::glance(&call),
            FETCH_MEDIA => fetch::fetch_media(call, self.per_turn, (self.wait, self.image_limit)),
            MENTION => mention::mention(call),
            QUOTE => quote::quote(call),
            SET_CARD => card::set_card(call),
            NOTICES => notices::notices_of(call),
            CHAT_HISTORY => history::history(call, "user must be a QQ number or a name"),
            SEND_FILE => send_file::send_file(&call, self.upload),
            CHAT_SEND => match send_request(&call.args) {
                Some((to, text)) => Planned::Host(Effect::Send { to, text }),
                // 没写发给谁（施工 I-16：以前借禁言那句「要对方引用或 @」，把她带偏去反复试）：回这个会话直接写字。
                None => Planned::Now(call.answers.say("no-recipient", &[], true)),
            },
            // 「这一轮不说话」im 的读的一头当场答，到不了这里；照说不会。
            SKIP_REPLY => Planned::Now(call.answers.say("skipped", &[], false)),
            AVATAR => lookup::avatar(self, call),
            MEMBERS => lookup::members(self, call),
            // im 只交这一套里有的：照说不会。
            _ => Planned::Now(call.answers.say("unreachable", &[], true)),
        }
    }
}
