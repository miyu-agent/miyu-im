//! 查询信息（施工 I-44 起，I-91 把群聊记录、群资料两个并成一个，项目主人 10-30 定照功能分）：OneBot 驱动带的 im 插件，同群管理。
//! 管只读的那几件（[`crate::tools::LOOKUPS`]）：查聊天记录、取图和文件、看头像、群成员、群公告、群文件、通讯录、发言排行。
//!
//! 工具本身在驱动的工具套里（`crate::tools`），这里只是插件的声明和总开关（`im.qq.lookup_on`，藏起来，在插件页里开关）。关了的不
//! 登记（改的时候 im 重新登记）；一个场所里关着的，那个场所的工具表里没有它们（核心 O-60）。

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use miyu_im::plugins::declared::{Links, Ui};
use miyu_im::plugins::{Builtin, Declared};

use crate::admin::switched;

/// 查询信息的编号。
pub(crate) const ID: &str = "miyu-im-qq-lookup";

/// 查询信息的总开关。
pub(crate) const ON: &str = "lookup_on";

/// 查询信息。
pub struct Lookup;

/// 三种语言的一句。
fn words(zh: &str, en: &str, ja: &str) -> BTreeMap<String, String> {
    [("zh", zh), ("en", en), ("ja", ja)]
        .into_iter()
        .map(|(lang, text)| (lang.to_string(), text.to_string()))
        .collect()
}

impl Builtin for Lookup {
    fn id(&self) -> &'static str {
        ID
    }

    fn declared(&self) -> Declared {
        Declared {
            name: words("查询信息", "Lookups", "情報の照会"),
            summary: words(
                "让 AI 查聊天记录、取消息里的图和文件、看头像、群成员、群公告、群文件、通讯录和发言排行。",
                "Lets the AI search chat history, fetch images and files from messages, and see avatars, group members, announcements, group files, contacts and the activity ranking.",
                "AI がチャット履歴を検索し、メッセージの画像やファイルを取り出し、アイコン、グループのメンバー、お知らせ、ファイル、連絡先、発言ランキングを見られるようにします。",
            ),
            links: Links {
                icon: Some("search".to_string()),
                ..Links::default()
            },
            platforms: vec!["qq".to_string()],
            venues: Vec::new(),
            needs: Vec::new(),
            default: true,
            ui: Ui::default(),
        }
    }

    fn settings(&self) -> &'static str {
        "lookup_"
    }
}

/// 查询信息的工具登记不登记：总开关开着。
pub(crate) fn offers(config: &Map<String, Value>) -> bool {
    switched(config, ON)
}
