//! `/model`、`/persona`、`/preset`（施工 I-53，项目主人 10-27：QQ 里打 `/model` 没反应。核心没有这些命令，终端里的是终端自己
//! 做的，交给核心回 `unknown_command`，照普通的话交给她，她多半不理）：桥自己办，不交核心。只给终端管理员和场所规则 `managers`
//! 里的人（听得到被拒的那一句的那几个，`hears_refusals`），别人打了照原来交核心。
//!
//! 项目主人 10-27 又定（施工 I-57）：搜和换分开，换只认确切的名字，免得模糊找错了就换掉。11-10 又定（施工 I-191：名字太长抄不了，
//! 只列 10 个看不全）：全列、带编号，照编号换；同一天（施工 I-194）：人格、预设同理，三条命令一个样。下面拿 `/model` 说：
//!
//! - `/model`：说这个会话现在用哪个（卡片上选了的写它，没选的写「跟随默认（…）」），再一行一条说怎么用。
//! - `/model search [<关键词>]`：能选的编号（从 1 起）列出来，没写关键词的全列，写了的列对得上的，编号照全部里的那个；正在用的
//!   那个标出来。模糊：照空白拆词，每个词都要有，先后不论，字照先后零散出现也算（同后台选模型，I-50）；比的时候不分大小写、只看
//!   字母数字（`opus5.5` 对得上 `opus-5-5`）。
//! - `/model <编号>` 或 `/model <名字>`：写进这个会话的卡片（同后台卡片上选，`cards.save_one`；项目主人 10-27 定：改卡片，不只管
//!   这一段对话）。名字要确切（不分大小写）；模型只写了最后一个 `/` 后面那一段、又只有一个对得上的也算。对不上的说没有，提示去搜。
//! - `/model 默认`（`default` 同）：卡片上的这一格去掉，跟随默认。
//!
//! 能选的：模型照核心 `model.list` 列的池（`@<池>`，在前）和模型（`<供应商>/<模型>`）；人格照 `persona.list`、预设照
//! `preset.list`（写错了的不列），列的时候写「名字 (编号)」，换的时候名字、编号都认。办的那一半在 [`super::picking`]。

use serde_json::Value;

/// 去掉卡片上这一格的说法。
const DEFAULTS: [&str; 3] = ["默认", "default", "既定"];

/// 搜的子命令。
const SEARCH: &str = "search";

/// 哪一条命令。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// `/model`。
    Model,
    /// `/persona`。
    Persona,
    /// `/preset`。
    Preset,
}

impl Kind {
    /// 三条命令。
    pub(super) const ALL: [Kind; 3] = [Kind::Model, Kind::Persona, Kind::Preset];

    /// 卡片上的格、规则的键、字的前缀（`model-now`……）、日志里的命令名。
    pub(super) fn key(self) -> &'static str {
        match self {
            Kind::Model => "model",
            Kind::Persona => "persona",
            Kind::Preset => "preset",
        }
    }

    /// 核心列能选的那个方法。
    pub(super) fn list(self) -> &'static str {
        match self {
            Kind::Model => "model.list",
            Kind::Persona => "persona.list",
            Kind::Preset => "preset.list",
        }
    }
}

/// 能选的一个：写进卡片的 `id`，给人看的名字（模型的就是 `id`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Choice {
    pub(super) id: String,
    pub(super) name: String,
}

impl Choice {
    /// 列出来、回执里的写法：名字和编号不一样的写「名字 (编号)」。
    pub(super) fn shown(&self) -> String {
        if self.name.is_empty() || self.name == self.id {
            self.id.clone()
        } else {
            format!("{} ({})", self.name, self.id)
        }
    }
}

/// 命令后面的词说的是哪一种（见模块开头）。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Asked<'a> {
    /// 什么都没写：现在用哪个。
    Now,
    /// 搜这个关键词（可能是空的）。
    Search(&'a str),
    /// 去掉卡片上的这一格。
    Clear,
    /// 换成这个编号、名字。
    Set(&'a str),
}

/// 命令后面的词 `query`（去过首尾空白）说的是哪一种。
pub(super) fn asked(query: &str) -> Asked<'_> {
    if query.is_empty() {
        return Asked::Now;
    }
    if DEFAULTS.iter().any(|word| word.eq_ignore_ascii_case(query)) {
        return Asked::Clear;
    }
    let mut parts = query.splitn(2, char::is_whitespace);
    match (parts.next(), parts.next()) {
        (Some(head), rest) if head.eq_ignore_ascii_case(SEARCH) => {
            Asked::Search(rest.unwrap_or_default().trim())
        }
        _ => Asked::Set(query),
    }
}

/// `text` 是不是这三条命令里的一条：是的交回哪一条、后面的词（去掉首尾空白，可能是空的）。
pub(super) fn picked(text: &str) -> Option<(Kind, &str)> {
    let rest = text.trim_start().strip_prefix('/')?;
    Kind::ALL.into_iter().find_map(|kind| {
        let rest = rest.strip_prefix(kind.key())?;
        match rest.chars().next() {
            None => Some((kind, "")),
            Some(c) if c.is_whitespace() => Some((kind, rest.trim())),
            Some(_) => None,
        }
    })
}

/// `query` 说的是 `choices` 里哪个（见模块开头）：编号（从 1 起）；整个一样的编号、名字（不分大小写，名字要只有一个对得上）；
/// 模型（`kind`）再看最后一个 `/` 后面那一段，只有一个对得上才算。
pub(super) fn exact<'a>(kind: Kind, query: &str, choices: &'a [Choice]) -> Option<&'a Choice> {
    if let Ok(number) = query.parse::<usize>() {
        return number.checked_sub(1).and_then(|at| choices.get(at));
    }
    if let Some(choice) = choices
        .iter()
        .find(|choice| choice.id.eq_ignore_ascii_case(query))
    {
        return Some(choice);
    }
    let only = |mut found: std::iter::Filter<std::slice::Iter<'a, Choice>, _>| match (
        found.next(),
        found.next(),
    ) {
        (Some(choice), None) => Some(choice),
        _ => None,
    };
    let lowered = query.to_lowercase();
    if let Some(choice) = only(
        choices
            .iter()
            .filter(|choice| choice.name.to_lowercase() == lowered),
    ) {
        return Some(choice);
    }
    if kind != Kind::Model {
        return None;
    }
    let tails: Vec<&Choice> = choices
        .iter()
        .filter(|choice| {
            choice
                .id
                .rsplit_once('/')
                .is_some_and(|(_, tail)| tail.eq_ignore_ascii_case(query))
        })
        .collect();
    match tails[..] {
        [choice] => Some(choice),
        _ => None,
    }
}

/// 照关键词 `query` 在 `choices` 里搜（见模块开头）：交回对得上的全部，带它在 `choices` 里的编号（从 1 起），照像不像排；
/// 关键词是空的全列、照原来的先后。
pub(super) fn search<'a>(query: &str, choices: &'a [Choice]) -> Vec<(usize, &'a Choice)> {
    let mut scored: Vec<(u32, usize, &Choice)> = choices
        .iter()
        .enumerate()
        .filter_map(|(at, choice)| {
            let text = format!("{} {}", choice.id, choice.name);
            fuzzy(query, &text).map(|score| (score, at + 1, choice))
        })
        .collect();
    if !query.trim().is_empty() {
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    }
    scored
        .into_iter()
        .map(|(_, number, choice)| (number, choice))
        .collect()
}

/// 比的时候只看的字：小写的字母数字（`claude-opus-5-5` 是 `claudeopus55`）。
fn squashed(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// 模糊匹配的分（见模块开头）：整个出现的词 2 分，字照先后零散出现的 1 分；有一个词找不到的是空的。只有标点的词不算。
fn fuzzy(query: &str, text: &str) -> Option<u32> {
    let hay = squashed(text);
    let mut score = 0;
    for word in query.split_whitespace().map(squashed) {
        if word.is_empty() {
            continue;
        }
        if hay.contains(&word) {
            score += 2;
            continue;
        }
        let mut rest = hay.as_str();
        for c in word.chars() {
            let at = rest.find(c)?;
            rest = &rest[at + c.len_utf8()..];
        }
        score += 1;
    }
    Some(score)
}

/// 核心列的回答 `listed`（`kind` 的那个方法的 `result`）里能选的（见模块开头）。
pub(super) fn choices(kind: Kind, listed: &Value) -> Vec<Choice> {
    let rows = |key: &str| listed[key].as_array().into_iter().flatten();
    let named = |key: &'static str| {
        rows(match kind {
            Kind::Preset => "presets",
            _ => "personas",
        })
        .filter(|row| row.get("problem").is_none())
        .filter_map(move |row| {
            let id = row[key].as_str()?.to_string();
            let name = row["name"].as_str().unwrap_or_default().to_string();
            Some(Choice { id, name })
        })
        .collect()
    };
    match kind {
        Kind::Persona => named("persona"),
        Kind::Preset => named("preset"),
        Kind::Model => {
            let same = |id: String| Choice {
                name: id.clone(),
                id,
            };
            let pools = rows("pools")
                .filter_map(|pool| pool["name"].as_str().map(|name| format!("@{name}")));
            let models = rows("providers")
                .flat_map(|provider| provider["models"].as_array().into_iter().flatten())
                .filter_map(|model| model["ref"].as_str().map(str::to_string));
            pools.chain(models).map(same).collect()
        }
    }
}

#[cfg(test)]
mod tests;
