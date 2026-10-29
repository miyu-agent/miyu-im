//! `/model`（施工 I-53，项目主人 10-27：QQ 里打 `/model` 没反应。核心没有这条命令，终端里那个是终端自己做的，交给核心回
//! `unknown_command`，照普通的话交给她，她多半不理）：桥自己办，不交核心。只给终端管理员和场所规则 `managers` 里的人（听得到被拒的
//! 那一句的那几个，`hears_refusals`），别人打了照原来交核心。
//!
//! 项目主人 10-27 又定（施工 I-57）：模型太多，不一口气全列；搜和换分开，换只认确切的名字，免得模糊找错了就换掉。
//!
//! - `/model`：说这个会话现在用哪个模型（卡片上选了的写它，没选的写「跟随默认（…）」），再说一行怎么用。
//! - `/model search <关键词>`：照核心 `model.list` 列的池（`@<池>`）和模型（`<供应商>/<模型>`）模糊找，列出前 [`SHOWN`] 个，多的说
//!   还有几个；正在用的那个标出来。模糊：照空白拆词，每个词都要有，先后不论，字照先后零散出现也算（同后台选模型，I-50）。没写
//!   关键词的照先后列（池在前）。
//! - `/model <名字>`：确切的名字（不分大小写）写进这个会话的卡片 `model`（同后台卡片上选，`cards.save_one`；项目主人 10-27 定：
//!   改卡片，不只管这一段对话）。只写了模型那一段（最后一个 `/` 后面）、又只有一个对得上的也算。对不上的说没有，提示去搜。
//! - `/model 默认`（`default` 同）：卡片上的模型去掉，跟随默认。
//!
//! 这个会话没有自己的卡片（陌生群、陌生私聊）的不改兜底卡（会改到所有陌生的会话），说先在后台给它加一张卡片。回的那一句照回执
//! 发（不撤回）。字照连接的语言，在出厂的 `cards/human/page.json`（`model-*`）。

use miyu_chat::Venue;
use serde_json::{Value, json};

use super::super::{Message, Route};
use crate::TARGET;
use crate::chat::To;
use crate::core::Gone;

/// 找到几个时最多列几个。
const SHOWN: usize = 10;

/// 去掉卡片上的模型的说法。
const DEFAULTS: [&str; 3] = ["默认", "default", "既定"];

/// 搜的子命令。
const SEARCH: &str = "search";

/// `/model` 后面的词说的是哪一种（见模块开头）。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Asked<'a> {
    /// 什么都没写：现在用哪个。
    Now,
    /// 搜这个关键词（可能是空的）。
    Search(&'a str),
    /// 去掉卡片上的模型。
    Clear,
    /// 换成这个名字。
    Set(&'a str),
}

/// `/model` 后面的词 `query`（去过首尾空白）说的是哪一种。
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

/// `text` 是不是 `/model` 命令：是的交回后面的词（去掉首尾空白，可能是空的）。
pub(super) fn model_query(text: &str) -> Option<&str> {
    let rest = text.trim_start().strip_prefix("/model")?;
    match rest.chars().next() {
        None => Some(""),
        Some(c) if c.is_whitespace() => Some(rest.trim()),
        Some(_) => None,
    }
}

/// 确切的名字 `query` 在 `names` 里是哪个（见模块开头）：整个一样的（不分大小写），没有的看模型那一段，只有一个对得上才算。
pub(super) fn exact(query: &str, names: &[String]) -> Option<String> {
    if let Some(name) = names.iter().find(|name| name.eq_ignore_ascii_case(query)) {
        return Some(name.clone());
    }
    let mut tails = names.iter().filter(|name| {
        name.rsplit_once('/')
            .is_some_and(|(_, tail)| tail.eq_ignore_ascii_case(query))
    });
    match (tails.next(), tails.next()) {
        (Some(name), None) => Some(name.clone()),
        _ => None,
    }
}

/// 照关键词 `query` 在 `names` 里搜（见模块开头）：交回照像不像排的前 [`SHOWN`] 个和还有几个没列；关键词是空的照原来的先后。
pub(super) fn search(query: &str, names: &[String]) -> (Vec<String>, usize) {
    let mut scored: Vec<(u32, &String)> = names
        .iter()
        .filter_map(|name| fuzzy(query, name).map(|score| (score, name)))
        .collect();
    if !query.trim().is_empty() {
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    }
    let more = scored.len().saturating_sub(SHOWN);
    let shown = scored
        .into_iter()
        .take(SHOWN)
        .map(|(_, name)| name.clone())
        .collect();
    (shown, more)
}

/// 模糊匹配的分（见模块开头）：整个出现的词 2 分，字照先后零散出现的 1 分；有一个词找不到的是空的。
fn fuzzy(query: &str, text: &str) -> Option<u32> {
    let hay = text.to_lowercase();
    let mut score = 0;
    for word in query.to_lowercase().split_whitespace() {
        if hay.contains(word) {
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

/// 核心 `model.list` 的回答 `listed` 里能选的：每个池 `@<池>`，每个模型的引用（池在前，没写关键词搜的时候先看到）。
fn choices(listed: &Value) -> Vec<String> {
    let models = listed["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|provider| provider["models"].as_array().into_iter().flatten())
        .filter_map(|model| model["ref"].as_str().map(str::to_string));
    let pools = listed["pools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|pool| pool["name"].as_str().map(|name| format!("@{name}")));
    pools.chain(models).collect()
}

impl Route {
    /// 办 `/model`（见模块开头），后面的词是 `query`。不归它管的人打的交回假，照原来交核心。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn model_command(
        &mut self,
        message: &Message,
        query: &str,
    ) -> Result<bool, Gone> {
        if !self.hears_refusals(message).await? {
            return Ok(false);
        }
        let Some(session) = self.session_of(message).await? else {
            return Ok(true);
        };
        let words = self.rules.page_words().clone();
        let lang = self.core.language.clone();
        let listed = self.core.call("model.list", json!({})).await?;
        let listed = &listed["result"];
        let said = match asked(query) {
            Asked::Now => {
                let mut said = self.model_now(message, listed, &words, &lang);
                said.push('\n');
                said.push_str(&words.get("model-usage", &lang));
                said
            }
            Asked::Clear => self.set_model(message, None, &words, &lang),
            Asked::Set(name) => match exact(name, &choices(listed)) {
                Some(model) => self.set_model(message, Some(&model), &words, &lang),
                None => words.get("model-none", &lang).replace("{query}", name),
            },
            Asked::Search(keywords) => {
                let (names, more) = search(keywords, &choices(listed));
                if names.is_empty() {
                    words
                        .get("model-search-none", &lang)
                        .replace("{query}", keywords)
                } else {
                    let using = self.using(message, listed);
                    let mut said = words.get("model-found", &lang);
                    for name in names {
                        said.push('\n');
                        said.push_str(&name);
                        if using.as_deref() == Some(name.as_str()) {
                            said.push_str(&words.get("model-using-mark", &lang));
                        }
                    }
                    if more > 0 {
                        said.push('\n');
                        said.push_str(
                            &words
                                .get("model-more", &lang)
                                .replace("{count}", &more.to_string()),
                        );
                    }
                    said
                }
            }
        };
        tracing::info!(target: TARGET, venue = %message.place.peer.venue, message = message.number, command = "model", "command ran");
        self.receipt(&session, &said, false).await?;
        Ok(true)
    }

    /// `message` 那个场所的会话：记着的，没有的找（同发消息）。
    async fn session_of(&mut self, message: &Message) -> Result<Option<String>, Gone> {
        let venue = message.place.peer.venue.as_str().to_string();
        let session = match self.venues.get(&venue).cloned() {
            Some(session) => Some(session),
            None => self.find(&message.place).await?,
        };
        if let Some(session) = &session {
            self.peers
                .insert(session.clone(), message.place.peer.clone());
        }
        Ok(session)
    }

    /// 「现在用的是哪个」：卡片上写的，没写的照规则套出来的，再没有的照核心的对话模型。
    fn model_now(
        &mut self,
        message: &Message,
        listed: &Value,
        words: &crate::rules::Words,
        lang: &str,
    ) -> String {
        if let Some(model) =
            self.card_model(message.place.peer.platform.name(), &message.place.peer.to)
        {
            return words.get("model-now", lang).replace("{model}", &model);
        }
        let fallback = self
            .fallback_model(message, listed)
            .unwrap_or_else(|| "-".to_string());
        words
            .get("model-follows", lang)
            .replace("{model}", &fallback)
    }

    /// 这个会话此刻真用的是哪个：卡片上写的，没写的照 [`Route::fallback_model`]。
    fn using(&mut self, message: &Message, listed: &Value) -> Option<String> {
        self.card_model(message.place.peer.platform.name(), &message.place.peer.to)
            .or_else(|| self.fallback_model(message, listed))
    }

    /// 卡片上没写时跟随的：照规则套出来的，再没有的照核心的对话模型。
    fn fallback_model(&mut self, message: &Message, listed: &Value) -> Option<String> {
        Venue::parse(&message.place.peer.venue)
            .and_then(|venue| {
                let applied = self.applied(&venue);
                super::super::applied::text_of(&applied, "model").map(str::to_string)
            })
            .or_else(|| listed["uses"]["chat"].as_str().map(str::to_string))
    }

    /// 平台 `platform` 上这个场所自己的卡片上写的模型。
    fn card_model(&self, platform: &str, to: &To) -> Option<String> {
        let cards = self.rules.cards();
        match to {
            To::Group(group) => cards
                .groups
                .iter()
                .find(|card| card.platform == platform && card.id == *group)?
                .set
                .model
                .clone(),
            To::Private(user) => cards
                .privates
                .iter()
                .find(|card| card.platform == platform && card.id == *user)?
                .set
                .model
                .clone(),
        }
    }

    /// 把这个场所的卡片的模型改成 `model`（空的是去掉），交回回给他的那一句。
    fn set_model(
        &mut self,
        message: &Message,
        model: Option<&str>,
        words: &crate::rules::Words,
        lang: &str,
    ) -> String {
        let platform = message.place.peer.platform.name();
        let card = match &message.place.peer.to {
            To::Group(group) => json!({"kind": "group", "platform": platform, "id": group}),
            To::Private(user) => json!({"kind": "private", "platform": platform, "id": user}),
        };
        let saved = self.save_one(&json!({"card": card, "values": {"model": model}}));
        match saved {
            Some(answer) if answer["saved"] == true => match model {
                Some(model) => words.get("model-set", lang).replace("{model}", model),
                None => words.get("model-cleared", lang),
            },
            Some(answer) if answer.get("problems").is_some() => {
                let detail = answer["problems"][0]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                tracing::info!(target: TARGET, venue = %message.place.peer.venue, reason = "card_refused", "model not set");
                words
                    .get("model-refused", lang)
                    .replace("{detail}", &detail)
            }
            _ => words.get("model-no-card", lang),
        }
    }
}

#[cfg(test)]
mod tests;
