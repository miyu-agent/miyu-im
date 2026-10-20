//! 场所卡片那一层（`onebot.md` 第一条「场所卡片」，施工 O-58 上）：后台页上一个场所一张卡，白名单里的群一张、白名单成员
//! 一人一张，「其他群」一张兜底（出厂关着）。卡片由桥存（[`store`]），拼成规则交群聊内核读，来处和配置项那一层一样是
//! [`Source::Settings`](miyu_chat::Source::Settings)，排在 `60-settings.toml` 后面。
//!
//! - 一张卡拼成一份规则文件、一条规则（[`Cards::files`]）：文件名里写着是哪一张卡（[`Card::parse`]），问题、来处照它说成
//!   「哪一张卡」。先后照文件名：「其他群」在最前面（匹配名单以外的群：`not_group`，名单里的群不继承它设的格），每个群一条，
//!   每个私聊一条。
//! - 每一格可选，没写的照下面几层（配置项、出厂、系统的规则）。群卡没写 `on` 的回复（在白名单里），「其他群」「其他私聊」（施工
//!   O-60：陌生人的私聊，不管白名单成员和认出来的终端管理员）没写的不回复。
//! - 插嘴四档：`never` 拼成 `discipline = "when-called"`，别的三档拼成 `discipline = "chatty"` 加 `chatty.probability`
//!   （[`PROBABILITY`]，「施工时定的」）。长文（施工 O-59 上）、仅发送最终消息（施工 O-57）拼进同一张 `outbound` 表。去句号
//!   O-58 补二挪成配置项（`configured`）：是通用的，不进卡片。
//! - 字照 JSON 的写法写进去（同配置项那一层）：JSON 的转义都是 TOML 基本字符串认的。

pub mod defaults;
pub mod edit;
mod lines;

pub use lines::OTHER_GROUPS_RATE;
pub mod page;
mod store;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use miyu_chat::{File, Problem, Rules};

use lines::{Matcher, defaults_lines, file, group_lines, other_private_lines, private_lines};
pub use store::{load, path, save};

/// 卡片拼出来的文件名的开头：排在配置项那一份 `60-settings.toml` 后面。
const PREFIX: &str = "70-card-";

/// 插嘴三档的千分比：`few`、`some`（出厂的 `chatty.probability`）、`many`。
const PROBABILITY: [(Chatty, u32); 3] =
    [(Chatty::Few, 20), (Chatty::Some, 50), (Chatty::Many, 120)];

/// 插嘴：不插嘴（只在叫她时回）、少、偶尔、常。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chatty {
    /// 只在叫她时回。
    Never,
    /// 少插嘴。
    Few,
    /// 偶尔（出厂）。
    Some,
    /// 常插嘴。
    Many,
}

/// 长文怎么发（施工 O-59 上）：拆成几条、转成图、合并转发（O-59 下）。拼成规则的 `outbound.long_text`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LongText {
    /// 拆成几条。
    Split,
    /// 转成图（出厂）。
    Image,
    /// 合并转发（施工 O-59 下）。
    Forward,
}

impl LongText {
    /// 规则里的写法。
    fn name(self) -> &'static str {
        match self {
            LongText::Split => "split",
            LongText::Image => "image",
            LongText::Forward => "forward",
        }
    }
}

/// 群卡、「其他群」卡能设的几格：都可选，没写的照下面几层。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupSet {
    /// 接不接话：拼成 `allow`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    /// 人格。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// 对话模型（施工 O-58 补三）：`<供应商>/<模型>` 或 `@<池>`，拼成 `model`；没写的跟着配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 预设（施工 I-16）：预设的编号，拼成 `preset`；改了照 `session.configure` 换到已有的会话上（`route::policy`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// 附加提示词。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_prompt: Option<String>,
    /// 触发词。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    /// 插嘴。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatty: Option<Chatty>,
    /// 管理的人：平台身份 `qq:<号>`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managers: Option<Vec<String>>,
    /// 睡觉时间：写法同规则的 `sleep`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep: Option<String>,
    /// 限流：写法同规则的 `rate`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<String>,
    /// 她看得到 QQ 号。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_ids: Option<bool>,
    /// 回复时带多少条群聊记录（施工 O-62）：拼成 `recent`，1 到 1000。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent: Option<u32>,
    /// 长文怎么发（施工 O-59 上）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_text: Option<LongText>,
    /// 超过几个字算长文，0 是不算。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_chars: Option<u32>,
    /// 仅发送最终消息（施工 O-57）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_only: Option<bool>,
    /// 插件开关（施工 I-8 上）：包的编号到开没开；没写的照兜底卡、插件声明的默认（`crate::plugins::Toggles`）。不拼进规则。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<BTreeMap<String, bool>>,
    /// 别的规则键（施工 I-7 中一：会话浮窗上没有单独一格的那些）：`表.项`（出厂参数里的）或 `lines::TOP` 里的顶层键，值照 JSON
    /// 写；和上面哪一格是同一个键的，照这里的（`lines`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<BTreeMap<String, Value>>,
}

/// 私聊卡能设的几格：睡觉、限流本来就不管白名单成员，不给。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrivateSet {
    /// 渲染时写发的人的平台身份（施工 I-16 照旧版：私聊也有）：拼成 `show_ids`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_ids: Option<bool>,
    /// 人格。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// 对话模型（施工 O-58 补三）：`<供应商>/<模型>` 或 `@<池>`，拼成 `model`；没写的跟着配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 预设（施工 I-16）：预设的编号，拼成 `preset`；改了照 `session.configure` 换到已有的会话上（`route::policy`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// 附加提示词。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_prompt: Option<String>,
    /// 长文怎么发（施工 O-59 上）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_text: Option<LongText>,
    /// 超过几个字算长文，0 是不算。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_chars: Option<u32>,
    /// 仅发送最终消息（施工 O-57）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_only: Option<bool>,
    /// 插件开关（施工 I-8 上）：包的编号到开没开；没写的照兜底卡、插件声明的默认（`crate::plugins::Toggles`）。不拼进规则。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<BTreeMap<String, bool>>,
    /// 别的规则键（施工 I-7 中一：会话浮窗上没有单独一格的那些）：`表.项`（出厂参数里的）或 `lines::TOP` 里的顶层键，值照 JSON
    /// 写；和上面哪一格是同一个键的，照这里的（`lines`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<BTreeMap<String, Value>>,
}

/// 「其他私聊」卡能设的几格（施工 O-60）：陌生人（不是终端管理员、不在白名单里的人）的私聊。回复没写的不回复。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OtherPrivateSet {
    /// 回复不回复：拼成 `allow`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    /// 人格。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// 对话模型（施工 O-58 补三）：`<供应商>/<模型>` 或 `@<池>`，拼成 `model`；没写的跟着配置。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 预设（施工 I-16）：预设的编号，拼成 `preset`；改了照 `session.configure` 换到已有的会话上（`route::policy`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// 附加提示词。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_prompt: Option<String>,
    /// 限流：写法同规则的 `rate`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<String>,
    /// 睡觉时间（施工 I-16）：写法同规则的 `sleep`（`off` 是不睡）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleep: Option<String>,
    /// 渲染时写发的人的平台身份（施工 I-16）：拼成 `show_ids`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_ids: Option<bool>,
    /// 长文怎么发。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_text: Option<LongText>,
    /// 超过几个字算长文，0 是不算。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_chars: Option<u32>,
    /// 仅发送最终消息。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_only: Option<bool>,
    /// 插件开关（施工 I-8 上）：包的编号到开没开；没写的照兜底卡、插件声明的默认（`crate::plugins::Toggles`）。不拼进规则。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<BTreeMap<String, bool>>,
    /// 别的规则键（施工 I-7 中一：会话浮窗上没有单独一格的那些）：`表.项`（出厂参数里的）或 `lines::TOP` 里的顶层键，值照 JSON
    /// 写；和上面哪一格是同一个键的，照这里的（`lines`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<BTreeMap<String, Value>>,
}

/// 一个群的卡。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupCard {
    /// 群号。
    pub id: u64,
    /// 设的几格。
    #[serde(default)]
    pub set: GroupSet,
}

/// 一个白名单成员的私聊卡。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateCard {
    /// 对方的号。
    pub id: u64,
    /// 设的几格。
    #[serde(default)]
    pub set: PrivateSet,
}

/// 全部卡片：`cards.json` 存的、`cards.get` 交的、`cards.save` 收的都是这个样子。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Cards {
    /// 「其他群」：白名单以外的群。
    pub other_groups: GroupSet,
    /// 白名单里的群，一群一张。
    pub groups: Vec<GroupCard>,
    /// 白名单成员，一人一张。
    pub privates: Vec<PrivateCard>,
    /// 「其他私聊」：陌生人的私聊（施工 O-60）。
    pub other_privates: OtherPrivateSet,
    /// 会话默认值（施工 I-7 下二，设置页）：群、私聊都套，排在别的卡下面，卡上没写的格照它。「是否回复」不归它管（写了也不拼）。
    /// 一格都没写的不存这一项（老版本的 im 照旧读得了 `cards.json`）。
    #[serde(skip_serializing_if = "unset")]
    pub defaults: GroupSet,
}

/// 一格都没写。
fn unset(set: &GroupSet) -> bool {
    *set == GroupSet::default()
}

/// 一张卡是哪一张：拼出来的文件名里写着。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Card {
    /// 会话默认值（施工 I-7 下二）。
    Defaults,
    /// 「其他群」。
    OtherGroups,
    /// 「其他私聊」（施工 O-60）。
    OtherPrivates,
    /// 群号是它的那个群。
    Group(u64),
    /// 对方的号是它的那个私聊。
    Private(u64),
}

impl Card {
    /// 拼出来的文件名：`70-card-a.toml`、`70-card-b.toml`、`70-card-g-<群号>.toml`、`70-card-p-<号>.toml`，照字排就是「其他群」、
    /// 「其他私聊」、群、私聊。
    fn file_name(self) -> String {
        match self {
            Card::Defaults => format!("{PREFIX}0.toml"),
            Card::OtherGroups => format!("{PREFIX}a.toml"),
            Card::OtherPrivates => format!("{PREFIX}b.toml"),
            Card::Group(id) => format!("{PREFIX}g-{id}.toml"),
            Card::Private(id) => format!("{PREFIX}p-{id}.toml"),
        }
    }

    /// 照文件名认是哪一张卡：不是卡片拼的文件的是空的。
    pub fn parse(file: &str) -> Option<Card> {
        let rest = file.strip_prefix(PREFIX)?.strip_suffix(".toml")?;
        match rest {
            "0" => return Some(Card::Defaults),
            "a" => return Some(Card::OtherGroups),
            "b" => return Some(Card::OtherPrivates),
            _ => {}
        }
        let (kind, id) = rest.split_once('-')?;
        let id = id.parse().ok()?;
        match kind {
            "g" => Some(Card::Group(id)),
            "p" => Some(Card::Private(id)),
            _ => None,
        }
    }

    /// 交给页面的写法：`{"kind": "other_groups"}`、`{"kind": "group", "id": …}`、`{"kind": "private", "id": …}`。
    pub fn json(self) -> Value {
        match self {
            Card::Defaults => serde_json::json!({"kind": "defaults"}),
            Card::OtherGroups => serde_json::json!({"kind": "other_groups"}),
            Card::OtherPrivates => serde_json::json!({"kind": "other_privates"}),
            Card::Group(id) => serde_json::json!({"kind": "group", "id": id}),
            Card::Private(id) => serde_json::json!({"kind": "private", "id": id}),
        }
    }
}

impl Cards {
    /// 拼成规则文件：一张卡一份、一条规则，照文件名的先后（[`Card::parse`] 认得回来）。同一个号写了两张的只拼第一张。
    /// 「其他私聊」不管的号一个都不排（查写法用；场所规则那一层照 [`Cards::files_except`]）。
    pub fn files(&self) -> Vec<File> {
        self.files_except(&[])
    }

    /// 同 [`Cards::files`]，「其他私聊」不管 `known` 这几个号（白名单成员、认出来的终端管理员，施工 O-60）。
    pub fn files_except(&self, known: &[u64]) -> Vec<File> {
        let listed: Vec<String> = self.groups.iter().map(|card| card.id.to_string()).collect();
        let others = Matcher::Others(listed.join(", "));
        let known: Vec<String> = known.iter().map(u64::to_string).collect();
        let mut files = vec![
            file(
                Card::OtherGroups,
                &others,
                &group_lines(&self.other_groups, false),
            ),
            file(
                Card::OtherPrivates,
                &Matcher::Strangers(known.join(", ")),
                &other_private_lines(&self.other_privates),
            ),
        ];
        // 会话默认值写了的才拼（施工 I-7 下二），排在最前。
        if !unset(&self.defaults) {
            files.insert(0, self.defaults_file());
        }
        let mut seen = std::collections::BTreeSet::new();
        for card in &self.groups {
            if seen.insert(Card::Group(card.id)) {
                files.push(file(
                    Card::Group(card.id),
                    &Matcher::Own,
                    &group_lines(&card.set, true),
                ));
            }
        }
        for card in &self.privates {
            if seen.insert(Card::Private(card.id)) {
                files.push(file(
                    Card::Private(card.id),
                    &Matcher::Own,
                    &private_lines(&card.set),
                ));
            }
        }
        files
    }

    /// 会话默认值拼成的那一份（施工 I-7 下二：别的卡的默认值照它和下面几层算）。
    pub fn defaults_file(&self) -> File {
        file(
            Card::Defaults,
            &Matcher::All,
            &defaults_lines(&self.defaults),
        )
    }

    /// 卡片写错的地方：拼出来的几份单独交群聊内核读，交回问题（文件名照 [`Card::parse`] 认得出是哪一张卡）。
    pub fn problems(&self) -> Vec<Problem> {
        Rules::parse(&self.files()).problems
    }
}

#[cfg(test)]
mod tests;
