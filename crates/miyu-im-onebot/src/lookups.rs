//! 群聊记录、群资料（施工 I-44，设计 06 第 3.7 条）：OneBot 驱动带的两个 im 插件，同群管理。工具照提供者分块：
//!
//! - 群聊记录 `miyu-im-qq-records`：翻聊天记录、取图和文件、看头像（`qq_chat_history`、`qq_fetch_media`、`qq_avatar`）。
//! - 群资料 `miyu-im-qq-info`：群成员、群公告、群文件、联系人、活跃排行（`qq_members`、`qq_notices`、`qq_group_files`、
//!   `qq_contacts`、`qq_activity_ranking`）。
//!
//! 工具本身在驱动的工具套里（`crate::tools`），这里只是插件的声明和总开关（`im.qq.records_on`、`im.qq.info_on`，藏起来，在插件
//! 页里开关）。关了的不登记（改的时候 im 重新登记）；一个场所里关着的，那个场所的工具表里没有它们（核心 O-60）。

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use miyu_im::plugins::declared::{Links, Ui};
use miyu_im::plugins::{Builtin, Declared};

use crate::admin::switched;

/// 群聊记录的编号。
pub(crate) const RECORDS: &str = "miyu-im-qq-records";

/// 群资料的编号。
pub(crate) const INFO: &str = "miyu-im-qq-info";

/// 群聊记录的总开关。
pub(crate) const RECORDS_ON: &str = "records_on";

/// 群资料的总开关。
pub(crate) const INFO_ON: &str = "info_on";

/// 群聊记录。
pub struct Records;

/// 群资料。
pub struct Info;

/// 三种语言的一句。
fn words(zh: &str, en: &str, ja: &str) -> BTreeMap<String, String> {
    [("zh", zh), ("en", en), ("ja", ja)]
        .into_iter()
        .map(|(lang, text)| (lang.to_string(), text.to_string()))
        .collect()
}

/// 两个插件一样的声明：名字、说明、图标不同。
fn declared(
    name: BTreeMap<String, String>,
    summary: BTreeMap<String, String>,
    icon: &str,
) -> Declared {
    Declared {
        name,
        summary,
        links: Links {
            icon: Some(icon.to_string()),
            ..Links::default()
        },
        platforms: vec!["qq".to_string()],
        needs: Vec::new(),
        default: true,
        ui: Ui::default(),
    }
}

impl Builtin for Records {
    fn id(&self) -> &'static str {
        RECORDS
    }

    fn declared(&self) -> Declared {
        declared(
            words("群聊记录", "Chat records", "チャット記録"),
            words(
                "让 AI 翻聊天记录、取消息里的图和文件、看头像。",
                "Lets the AI look through chat history, fetch images and files from messages, and see avatars.",
                "AI がチャット履歴を見返し、メッセージの画像やファイルを取り出し、アイコンを見られるようにします。",
            ),
            "messages-square",
        )
    }

    fn settings(&self) -> &'static str {
        "records_"
    }
}

impl Builtin for Info {
    fn id(&self) -> &'static str {
        INFO
    }

    fn declared(&self) -> Declared {
        declared(
            words("群资料", "Group info", "グループ情報"),
            words(
                "让 AI 看群成员、群公告、群文件、联系人和活跃排行。",
                "Lets the AI see group members, announcements, group files, contacts and the activity ranking.",
                "AI がグループのメンバー、お知らせ、ファイル、連絡先、アクティブランキングを見られるようにします。",
            ),
            "contact",
        )
    }

    fn settings(&self) -> &'static str {
        "info_"
    }
}

/// 群聊记录的工具登记不登记：总开关开着。
pub(crate) fn offers_records(config: &Map<String, Value>) -> bool {
    switched(config, RECORDS_ON)
}

/// 群资料的工具登记不登记：总开关开着。
pub(crate) fn offers_info(config: &Map<String, Value>) -> bool {
    switched(config, INFO_ON)
}
