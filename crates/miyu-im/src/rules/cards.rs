//! 场所卡片那一层（`onebot.md` 第一条「场所卡片」，施工 O-58 上）：后台页上一个场所一张卡，白名单里的群一张、白名单成员
//! 一人一张，「其他群」一张兜底（出厂关着）。卡片由桥存（[`store`]），拼成规则交群聊内核读，来处和配置项那一层一样是
//! [`Source::Settings`]，排在 `60-settings.toml` 后面。
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
pub mod page;
mod store;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use miyu_chat::{File, Problem, Rules, Source};

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
    /// 长文怎么发（施工 O-59 上）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_text: Option<LongText>,
    /// 超过几个字算长文，0 是不算。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_chars: Option<u32>,
    /// 仅发送最终消息（施工 O-57）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_only: Option<bool>,
}

/// 私聊卡能设的几格：睡觉、限流本来就不管白名单成员，不给。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrivateSet {
    /// 人格。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
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
    /// 附加提示词。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_prompt: Option<String>,
    /// 限流：写法同规则的 `rate`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<String>,
    /// 长文怎么发。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_text: Option<LongText>,
    /// 超过几个字算长文，0 是不算。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_chars: Option<u32>,
    /// 仅发送最终消息。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_only: Option<bool>,
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
}

/// 一张卡是哪一张：拼出来的文件名里写着。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Card {
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

    /// 卡片写错的地方：拼出来的几份单独交群聊内核读，交回问题（文件名照 [`Card::parse`] 认得出是哪一张卡）。
    pub fn problems(&self) -> Vec<Problem> {
        Rules::parse(&self.files()).problems
    }
}

/// 一张卡匹配哪些场所。
enum Matcher {
    /// 「其他群」：群，但不是名单里的这几个（群号照 `, ` 连好）。名单里的群不继承它设的格（施工 O-58 上）。
    Others(String),
    /// 群卡、私聊卡：只配它自己。
    Own,
    /// 「其他私聊」：私聊，但不是这几个号（白名单成员、认出来的终端管理员，照 `, ` 连好，施工 O-60）。
    Strangers(String),
}

/// 一张卡的那一份：`[[rule]]`、匹配条件、设的几格。
fn file(card: Card, matcher: &Matcher, lines: &[String]) -> File {
    let matcher = match (card, matcher) {
        (_, Matcher::Others(listed)) => format!("{{ kind = \"group\", not_group = [{listed}] }}"),
        (_, Matcher::Strangers(known)) => format!("{{ kind = \"private\", not_user = [{known}] }}"),
        (Card::Group(id), Matcher::Own) => format!("{{ kind = \"group\", group = [{id}] }}"),
        (Card::Private(id), Matcher::Own) => format!("{{ kind = \"private\", user = [{id}] }}"),
        (Card::OtherGroups, Matcher::Own) => "{ kind = \"group\" }".to_string(),
        (Card::OtherPrivates, Matcher::Own) => "{ kind = \"private\" }".to_string(),
    };
    let mut text = format!("[[rule]]\nmatch = {matcher}\n");
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    File {
        source: Source::Settings,
        name: card.file_name(),
        text,
    }
}

/// 群卡的几行：`allow` 一定写（群卡没写的接，「其他群」没写的不接），别的设了才写。
fn group_lines(set: &GroupSet, listed: bool) -> Vec<String> {
    let mut lines = vec![format!("allow = {}", set.on.unwrap_or(listed))];
    push_text(&mut lines, "persona", set.persona.as_deref());
    push_text(&mut lines, "extra_prompt", set.extra_prompt.as_deref());
    if let Some(words) = &set.keywords {
        lines.push(format!("keywords = {}", texts(words)));
    }
    match set.chatty {
        Some(Chatty::Never) => lines.push("discipline = \"when-called\"".to_string()),
        Some(level) => {
            let per_mille = PROBABILITY
                .iter()
                .find_map(|(known, value)| (*known == level).then_some(*value))
                .unwrap_or_default();
            lines.push("discipline = \"chatty\"".to_string());
            lines.push(format!("chatty = {{ probability = {per_mille} }}"));
        }
        None => {}
    }
    if let Some(managers) = &set.managers {
        lines.push(format!("managers = {}", texts(managers)));
    }
    push_text(&mut lines, "sleep", set.sleep.as_deref());
    push_text(&mut lines, "rate", set.rate.as_deref());
    if let Some(show) = set.show_ids {
        lines.push(format!("show_ids = {show}"));
    }
    push_outbound(&mut lines, set.final_only, (set.long_text, set.long_chars));
    lines
}

/// 私聊卡的几行：设了才写。
fn private_lines(set: &PrivateSet) -> Vec<String> {
    let mut lines = Vec::new();
    push_text(&mut lines, "persona", set.persona.as_deref());
    push_text(&mut lines, "extra_prompt", set.extra_prompt.as_deref());
    push_outbound(&mut lines, set.final_only, (set.long_text, set.long_chars));
    lines
}

/// 「其他私聊」的几行（施工 O-60）：`allow` 一定写（没写的不回复），别的设了才写。
fn other_private_lines(set: &OtherPrivateSet) -> Vec<String> {
    let mut lines = vec![format!("allow = {}", set.on.unwrap_or(false))];
    push_text(&mut lines, "persona", set.persona.as_deref());
    push_text(&mut lines, "extra_prompt", set.extra_prompt.as_deref());
    push_text(&mut lines, "rate", set.rate.as_deref());
    push_outbound(&mut lines, set.final_only, (set.long_text, set.long_chars));
    lines
}

/// 设了的一段字写成一行。
fn push_text(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{key} = {}", quoted(value)));
    }
}

/// 仅发送最终消息、长文写成出站参数的一张表（设了才写，一格都没设的不写这一行）。去句号是通用的配置项（O-58 补二）。
fn push_outbound(
    lines: &mut Vec<String>,
    final_only: Option<bool>,
    (long_text, long_chars): (Option<LongText>, Option<u32>),
) {
    let mut items = Vec::new();
    if let Some(final_only) = final_only {
        items.push(format!("final_only = {final_only}"));
    }
    if let Some(long_text) = long_text {
        items.push(format!("long_text = \"{}\"", long_text.name()));
    }
    if let Some(chars) = long_chars {
        items.push(format!("long_chars = {chars}"));
    }
    if !items.is_empty() {
        lines.push(format!("outbound = {{ {} }}", items.join(", ")));
    }
}

/// 字的列表写成 TOML 的数组。
fn texts(list: &[String]) -> String {
    let items: Vec<String> = list.iter().map(|item| quoted(item)).collect();
    format!("[{}]", items.join(", "))
}

/// 照 JSON 的写法加上引号：JSON 的转义都是 TOML 基本字符串认的。
fn quoted(text: &str) -> String {
    Value::from(text).to_string()
}

#[cfg(test)]
mod tests;
