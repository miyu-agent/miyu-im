//! Telegram 给 AI 的工具（施工 I-183 下下，设计 07 第 3.5 节；2026-11-10 项目主人定第一版做「说话」和「记录」这一套）：照宿主的口子
//! （`miyu_im::toolset::ToolSet`）交一套，说明在包文件夹的 `data/telegram/tools/`，答的话在 `data/telegram/tool-results/`。
//!
//! 七件：不回复、主动发消息、引用、@ 人、查聊天记录、发言排行、定下次看一眼；I-185 下上多撤回、贴表情（`acts`）。参数的读法、查记录、排行、看一眼和 OneBot 同用宿主的
//! `miyu_im::toolset::common`；引用、@ 人认人在这里（Telegram 没有「取一条消息」「列群成员」，照 im 自己的聊天记录认）。贴表情、撤回、
//! I-185 下下多传文件（`tg_send_file`，宿主的 `common::send_file`，同 QQ）；群管理、群资料、戳一戳、群名片 Telegram 第一版不做（设计 07 第 3.5 节）。

use miyu_client::tooltext::Access;
use std::time::Duration;

use miyu_im::toolset::common::{glance, history, ranking, send_file, usage};
use miyu_im::toolset::{Answers, Call, Decl, Effect, Planned, Reach, ToolSet, send_request};
use serde_json::{Map, Value};

mod acts;
mod mention;
mod quote;
#[cfg(test)]
mod tests;

/// 「这一轮不说话」：im 的读的一头当场答，她的回复里有这件的调用块，这一轮就不发了。
pub const SKIP_REPLY: &str = "tg_skip_reply";

/// 主动发消息：发给和机器人私聊过的人、机器人在的群。本机的会话也给。
pub const CHAT_SEND: &str = "tg_send";

/// 引用指定的一条：私聊、群都给。
pub const QUOTE: &str = "tg_quote";

/// @ 指定的人：只给群。
pub const MENTION: &str = "tg_mention";

/// 查聊天记录：私聊、群都给，只读；跨场所要终端管理员叫的那一轮。
pub const CHAT_HISTORY: &str = "tg_chat_history";

/// 发言排行：只给群。
pub const ACTIVITY_RANKING: &str = "tg_activity_ranking";

/// 查用量（施工 I-206）：只给群。
pub const TOKEN_USAGE: &str = "tg_token_usage";

/// 定下一次什么时候看一眼：只在开着时不时看一眼的场所里有（`GLANCE_PLUGIN`）。
pub const GLANCE: &str = "tg_glance";

/// 撤回（施工 I-185 下上）：撤她给的编号，没给的撤叫她做的那条引用的那一条。
pub const RECALL: &str = "tg_recall";

/// 贴表情（施工 I-185 下上）。
pub const REACT: &str = "tg_react";

/// 往这个场所传她工作区里的一份文件（施工 I-185 下下）：放开的会话不限工作区（LV-4）。
pub const SEND_FILE: &str = "tg_send_file";

/// `user` 写错时答的。
const BAD_USER: &str = "user must be a Telegram user id or a name";

/// 私聊、群都给。
const VENUES: &[&str] = &["private", "group"];

/// 本机的会话也给（同 QQ 的主动发消息，施工 I-40）。
const EVERYWHERE: &[&str] = &["local", "private", "group"];

/// 有哪几件，照登记的先后。
const TOOLS: [Decl; 11] = [
    Decl {
        name: SKIP_REPLY,
        access: Access::Read,
        venues: VENUES,
    },
    Decl {
        name: CHAT_SEND,
        access: Access::Venue,
        venues: EVERYWHERE,
    },
    Decl {
        name: QUOTE,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: MENTION,
        access: Access::Venue,
        venues: &["group"],
    },
    Decl {
        name: CHAT_HISTORY,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: ACTIVITY_RANKING,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: TOKEN_USAGE,
        access: Access::Read,
        venues: EVERYWHERE,
    },
    Decl {
        name: GLANCE,
        access: Access::Read,
        venues: VENUES,
    },
    Decl {
        name: RECALL,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: REACT,
        access: Access::Venue,
        venues: VENUES,
    },
    Decl {
        name: SEND_FILE,
        access: Access::Venue,
        venues: EVERYWHERE,
    },
];

/// 答的话：名字和认的字段。宿主替这一套答的（发成了没有、排行、看一眼、够不着）也在这里。
const ANSWERS: [(&str, &[&str]); 40] = [
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
    ("not-found", &[]),
    ("history", &["count"]),
    ("no-history", &[]),
    ("history-admin-only", &[]),
    ("sent", &[]),
    ("no-target", &[]),
    ("no-recipient", &[]),
    ("which-bot", &["bots"]),
    // 终端里用场所工具（设计 13，施工 I-220）：同 OneBot 那六句。
    ("chat-in-group", &["group"]),
    ("chat-in-private", &["user"]),
    ("chat-needed", &[]),
    ("chat-bad", &[]),
    ("chat-off", &[]),
    ("only-here", &[]),
    ("failed", &["detail"]),
    ("unanswered", &[]),
    ("unreachable", &[]),
    ("bad-args", &["detail"]),
    ("group-only", &[]),
    // 撤回、贴表情（施工 I-185 下上）。
    ("recalled", &[]),
    ("reacted", &[]),
    ("no-quote", &[]),
    // 传文件（施工 I-185 下下，宿主的 `common::send_file`）。
    ("uploading", &["name", "size"]),
    ("still-uploading", &["name"]),
    ("no-file", &["path"]),
    ("outside", &["path"]),
    ("link", &["path"]),
    ("not-local", &[]),
    ("unsendable", &[]),
];

/// Telegram 的一套工具。
#[derive(Debug, Clone, Copy)]
pub struct TgTools {
    /// 传文件最多等多久（`bridge.json` 的 `upload_seconds`）：`outbox/` 里比它还老的是没删掉的，顺手清。
    upload: Duration,
}

impl TgTools {
    /// 照 `bridge.json` 的参数（传文件最多等多久 `upload`）造。
    pub fn new(upload: Duration) -> TgTools {
        TgTools { upload }
    }
}

impl ToolSet for TgTools {
    fn dir(&self) -> &'static str {
        "telegram"
    }

    fn tools(&self) -> &'static [Decl] {
        &TOOLS
    }

    fn answers(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &ANSWERS
    }

    fn silence(&self) -> Option<&'static str> {
        Some(SKIP_REPLY)
    }

    fn reach(&self, tool: &str) -> Reach {
        // 发一句的 `group`、`user` 就是发到哪，自己认（施工 I-40），不当成去别的场所（设计 13）。
        match tool {
            CHAT_SEND => Reach::Anywhere,
            // 查聊天记录不写场所是查全部（设计 13，施工 I-220，同 OneBot）。
            CHAT_HISTORY => Reach::Either,
            _ => Reach::Chat,
        }
    }

    fn offers(&self, tool: &str, config: &Map<String, Value>) -> bool {
        // 连接表里一个令牌都没有的不登记（没接 Telegram 的人，本机会话的工具面上不多一件 `tg_send`）；添了连接宿主照新配置重新登记。
        let _ = tool;
        connected(config)
    }

    fn plugin(&self, tool: &str) -> Option<&'static str> {
        (tool == GLANCE).then_some(miyu_im::toolset::GLANCE_PLUGIN)
    }

    fn local(&self, tool: &str, args: &Value, answers: &Answers) -> Option<Planned> {
        // 本机的会话只有主动发消息：交给 im 照这个平台连着的号发（同 QQ，施工 I-40、I-81 下下）。
        (tool == CHAT_SEND).then(|| sending(args, answers))
    }

    fn plan(&self, call: Call) -> Planned {
        match call.tool.as_str() {
            CHAT_SEND => sending(&call.args, &call.answers),
            QUOTE => quote::quote(call),
            MENTION => mention::mention(call),
            CHAT_HISTORY => history::history(call, BAD_USER),
            ACTIVITY_RANKING => ranking::ranking(&call),
            TOKEN_USAGE => usage::usage(call, BAD_USER),
            GLANCE => glance::glance(&call),
            RECALL => acts::recall(call),
            REACT => acts::react(call),
            SEND_FILE => send_file::send_file(&call, self.upload),
            // 「这一轮不说话」im 的读的一头当场答，到不了这里；照说不会。
            SKIP_REPLY => Planned::Now(call.answers.say("skipped", &[], false)),
            // im 只交这一套里有的：照说不会。
            _ => Planned::Now(call.answers.say("unreachable", &[], true)),
        }
    }
}

/// 配置 `config` 的连接表里写了令牌（纯逻辑）：`tokens` 里有一项不是空的。
fn connected(config: &Map<String, Value>) -> bool {
    config
        .get("tokens")
        .and_then(Value::as_array)
        .is_some_and(|tokens| tokens.iter().any(|token| !token.is_null() && token != ""))
}

/// 主动发消息：写对了交 im 发，没写发给谁的叫她在这个会话里直接写。
fn sending(args: &Value, answers: &Answers) -> Planned {
    match send_request(args) {
        Some((to, text)) => Planned::Host(Effect::Send { to, text }),
        None => Planned::Now(answers.say("no-recipient", &[], true)),
    }
}
