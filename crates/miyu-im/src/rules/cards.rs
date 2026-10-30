//! 场所卡片那一层（`onebot.md` 第一条「场所卡片」，施工 O-58 上）：后台页上一个场所一张卡，白名单里的群一张、白名单成员
//! 一人一张，「其他群」一张兜底（出厂关着）。卡片由桥存（[`store`]），拼成规则交群聊内核读，来处和配置项那一层一样是
//! [`Source::Settings`](miyu_chat::Source::Settings)，排在 `60-settings.toml` 后面。
//!
//! - 一张卡拼成一份规则文件、一条规则（[`Cards::files`]）：文件名里写着是哪一张卡（[`Card::parse`]），问题、来处照它说成
//!   「哪一张卡」。先后照文件名：「其他群」在最前面（匹配名单以外的群：`not_group`，名单里的群不继承它设的格），每个群一条，
//!   每个私聊一条。
//! - 每一格可选，没写的照下面几层（配置项、出厂、系统的规则）。群卡没写 `on` 的回复（在白名单里），「其他群」「其他私聊」（施工
//!   O-60：陌生人的私聊，不管白名单成员和认出来的终端管理员）没写的不回复。
//! - 插嘴四档（`chatty`）施工 I-88 删了（设计 05 第二节第 7 条）：旧卡片上写着的照没写读、不拼（[`retired`]），存的时候去掉。
//! - 长文（施工 O-59 上）、仅发送最终消息（施工 O-57）拼进同一张 `outbound` 表。去句号
//!   O-58 补二挪成配置项（`configured`）：是通用的，不进卡片。
//! - 字照 JSON 的写法写进去（同配置项那一层）：JSON 的转义都是 TOML 基本字符串认的。

mod card;
pub mod defaults;
pub mod edit;
mod lines;
mod scoped;

pub use lines::OTHER_GROUPS_RATE;
pub mod page;
mod store;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use miyu_chat::{File, Problem, Rules};

use crate::chat::Id;

use card::toml_id;
pub use card::{Card, OLD_PLATFORM, Scope, platform_ok};
use lines::{Matcher, defaults_lines, file, group_lines, other_private_lines, private_lines};
pub use scoped::others_key;
use scoped::scope_of_key;
pub use store::{load, path, save};

/// 卡片拼出来的文件名的开头：排在配置项那一份 `60-settings.toml` 后面。
const PREFIX: &str = "70-card-";

/// 删掉的一格（施工 I-88 的插嘴 `chatty`）：旧卡片上写着什么都收下、丢掉，存的时候不写。
fn retired<'de, D: serde::Deserializer<'de>>(written: D) -> Result<(), D::Error> {
    serde::de::IgnoredAny::deserialize(written).map(|_| ())
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
    /// 插嘴（施工 I-88 删了）：旧卡片上写着的照没写读，存的时候不写。
    #[serde(rename = "chatty", skip_serializing, deserialize_with = "retired")]
    pub retired_chatty: (),
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
    /// 哪个平台的群（施工 I-80）：没写的是 [`OLD_PLATFORM`]，写盘时它不写（只用 QQ 的人 `cards.json` 不变）。
    #[serde(default = "old_platform", skip_serializing_if = "is_old")]
    pub platform: String,
    /// 只管哪个号（施工 I-81 中，设计 07 第 7.6 节）：没写的管整个平台。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<Id>,
    /// 群号。
    pub id: Id,
    /// 设的几格。
    #[serde(default)]
    pub set: GroupSet,
}

/// 一个白名单成员的私聊卡。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateCard {
    /// 哪个平台的人（施工 I-80，同 [`GroupCard::platform`]）。
    #[serde(default = "old_platform", skip_serializing_if = "is_old")]
    pub platform: String,
    /// 只管哪个号（同 [`GroupCard::bot`]）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot: Option<Id>,
    /// 对方的号。
    pub id: Id,
    /// 设的几格。
    #[serde(default)]
    pub set: PrivateSet,
}

/// 别的平台、一个号的「其他群」「其他私聊」（施工 I-80、I-81 中）：整个 [`OLD_PLATFORM`] 的照旧在 [`Cards::other_groups`]、
/// [`Cards::other_privates`]。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Others {
    /// 「其他群」。
    #[serde(skip_serializing_if = "unset")]
    pub groups: GroupSet,
    /// 「其他私聊」。
    #[serde(skip_serializing_if = "private_unset")]
    pub privates: OtherPrivateSet,
}

/// 全部卡片：`cards.json` 存的、`cards.get` 交的、`cards.save` 收的都是这个样子。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Cards {
    /// [`OLD_PLATFORM`] 的「其他群」：白名单以外的群。
    pub other_groups: GroupSet,
    /// 白名单里的群，一群一张。
    pub groups: Vec<GroupCard>,
    /// 白名单成员，一人一张。
    pub privates: Vec<PrivateCard>,
    /// [`OLD_PLATFORM`] 的「其他私聊」：陌生人的私聊（施工 O-60）。
    pub other_privates: OtherPrivateSet,
    /// 别的平台、一个号的「其他群」「其他私聊」（施工 I-80、I-81 中）：键是平台名（整个平台）或 `<平台>:<机器人号>`（只管那个号，
    /// [`others_key`]），值是那两张；没写的平台照两张空卡（出厂不回复），没写的号照整个平台的。空的不写。
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub others: BTreeMap<String, Others>,
    /// 会话默认值（施工 I-7 下二，设置页）：群、私聊都套，排在别的卡下面，卡上没写的格照它。「是否回复」不归它管（写了也不拼）。
    /// 不分平台：管的是她这个人。一格都没写的不存这一项（老版本的 im 照旧读得了 `cards.json`）。
    #[serde(skip_serializing_if = "unset")]
    pub defaults: GroupSet,
}

/// 一格都没写。
fn unset(set: &GroupSet) -> bool {
    *set == GroupSet::default()
}

/// 「其他私聊」一格都没写。
fn private_unset(set: &OtherPrivateSet) -> bool {
    *set == OtherPrivateSet::default()
}

/// 卡片上没写平台的平台（serde 的默认）。
fn old_platform() -> String {
    OLD_PLATFORM.to_string()
}

/// 是 [`OLD_PLATFORM`]：写盘时不写。
fn is_old(platform: &String) -> bool {
    platform == OLD_PLATFORM
}

impl Cards {
    /// 卡片上提到的平台和 `mounted`（挂着的驱动的）合起来，加上 [`OLD_PLATFORM`]，照名字排、不重。
    pub(super) fn platforms<'a>(&'a self, mounted: &[&'a str]) -> BTreeSet<&'a str> {
        let mut all: BTreeSet<&str> = mounted.iter().copied().collect();
        all.insert(OLD_PLATFORM);
        all.extend(self.groups.iter().map(|card| card.platform.as_str()));
        all.extend(self.privates.iter().map(|card| card.platform.as_str()));
        all.extend(self.others.keys().map(|key| {
            key.split_once(':')
                .map_or(key.as_str(), |(platform, _)| platform)
        }));
        all
    }

    /// 拼成规则文件：一张卡一份、一条规则，照文件名的先后（[`Card::parse`] 认得回来）。同一个号写了两张的只拼第一张。
    /// 「其他私聊」不管的号一个都不排（查写法用；场所规则那一层照 [`Cards::files_for`]）。
    pub fn files(&self) -> Vec<File> {
        self.files_for(&[], &[])
    }

    /// 同 [`Cards::files`]：挂着的平台 `mounted` 每个都拼一份「其他群」「其他私聊」（没写过的照空卡，出厂不回复，施工 I-80）；
    /// 「其他私聊」不管 `known` 这几个人（平台上的身份 `<平台>:<号>`：白名单成员、认出来的终端管理员，施工 O-60）。
    ///
    /// 有自己卡片的号（施工 I-81 中，设计 07 第 7.6 节）各拼一份自己的「其他群」「其他私聊」（没写的照整个平台的），整个平台的
    /// 那两份排开这些号（`not_bot`）；号自己的「其他群」排开的是整个平台的群卡加上它自己的群卡。
    pub fn files_for(&self, mounted: &[&str], known: &[String]) -> Vec<File> {
        let mut files = Vec::new();
        // 会话默认值写了的才拼（施工 I-7 下二），排在最前。
        if !unset(&self.defaults) {
            files.push(self.defaults_file());
        }
        for platform in self.platforms(mounted) {
            let listed = |bot: Option<&Id>| -> Vec<String> {
                self.groups
                    .iter()
                    .filter(|card| card.platform == platform)
                    .filter(|card| card.bot.is_none() || card.bot.as_ref() == bot)
                    .map(|card| toml_id(&card.id))
                    .collect()
            };
            let strangers: Vec<String> = known
                .iter()
                .filter_map(|person| person.split_once(':'))
                .filter(|(on, _)| *on == platform)
                .filter_map(|(_, number)| Id::new(number))
                .map(|id| toml_id(&id))
                .collect();
            let bots = self.bots_on(platform);
            let spared: Vec<String> = bots.iter().map(toml_id).collect();
            let whole = Scope::platform(platform);
            files.push(file(
                Card::OtherGroups(whole.clone()),
                &Matcher::Others(listed(None).join(", "), spared.join(", ")),
                &group_lines(&self.other_groups_of(&whole), false),
            ));
            files.push(file(
                Card::OtherPrivates(whole.clone()),
                &Matcher::Strangers(strangers.join(", "), spared.join(", ")),
                &other_private_lines(&self.other_privates_of(&whole)),
            ));
            for bot in bots {
                let one = Scope::of(platform, Some(bot.clone()));
                files.push(file(
                    Card::OtherGroups(one.clone()),
                    &Matcher::Others(listed(Some(&bot)).join(", "), String::new()),
                    &group_lines(&self.other_groups_of(&one), false),
                ));
                files.push(file(
                    Card::OtherPrivates(one.clone()),
                    &Matcher::Strangers(strangers.join(", "), String::new()),
                    &other_private_lines(&self.other_privates_of(&one)),
                ));
            }
        }
        let mut seen = BTreeSet::new();
        for card in &self.groups {
            let which = Card::Group(Scope::of(&card.platform, card.bot.clone()), card.id.clone());
            if seen.insert(which.clone()) {
                files.push(file(which, &Matcher::Own, &group_lines(&card.set, true)));
            }
        }
        for card in &self.privates {
            let which = Card::Private(Scope::of(&card.platform, card.bot.clone()), card.id.clone());
            if seen.insert(which.clone()) {
                files.push(file(which, &Matcher::Own, &private_lines(&card.set)));
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

    /// 卡片写错的地方：拼出来的几份单独交群聊内核读，交回问题（文件名照 [`Card::parse`] 认得出是哪一张卡）。平台名不合写法的
    /// 照卡片那一格报（`platform`）。
    pub fn problems(&self) -> Vec<Problem> {
        Rules::parse(&self.files()).problems
    }

    /// 写错了的平台名（施工 I-80）：卡片上、`others` 里不合写法的；`others` 里写了 [`OLD_PLATFORM`] 的（它的写在
    /// `other_groups`、`other_privates`）；`others` 的键里机器人号写错的（施工 I-81 中）照整个键报。
    pub fn bad_platforms(&self) -> Vec<String> {
        let mut bad: BTreeSet<String> = BTreeSet::new();
        let named = self
            .groups
            .iter()
            .map(|card| &card.platform)
            .chain(self.privates.iter().map(|card| &card.platform));
        for platform in named {
            if !platform_ok(platform) {
                bad.insert(platform.clone());
            }
        }
        for key in self.others.keys() {
            match scope_of_key(key) {
                Some(scope) if platform_ok(&scope.platform) => {}
                _ => {
                    bad.insert(key.clone());
                }
            }
        }
        if self.others.contains_key(OLD_PLATFORM) {
            bad.insert(OLD_PLATFORM.to_string());
        }
        bad.into_iter().collect()
    }
}

#[cfg(test)]
mod tests;
