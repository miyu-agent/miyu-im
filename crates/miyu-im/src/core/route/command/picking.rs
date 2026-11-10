//! 办 `/model`、`/persona`、`/preset`（[`super::pick`] 的说明）：问核心能选的、读写这个会话的卡片、回一句。
//!
//! 卡片照这个连接的号找（施工 I-194：后台在会话页加的卡只管那一个号，原来只照「整个平台」的找，找不到就说没有卡片）：这个号
//! 自己的那张，没有的改整个平台的那张，都没有的先建一张这个号的、只写这一格（施工 I-191，同后台在会话页「加卡片」），不改
//! 兜底卡（会改到所有陌生的会话）。回的那一句照回执发（不撤回）。字照连接的语言，在出厂的
//! `cards/human/page.json`（`model-*`、`persona-*`、`preset-*`）。

use miyu_chat::Venue;
use serde_json::{Value, json};

use super::super::{Message, Route};
use super::pick::{Asked, Choice, Kind, asked, choices, exact, search};
use crate::TARGET;
use crate::chat::{Id, To};
use crate::core::Gone;
use crate::rules::Words;

impl Route {
    /// 办 `kind` 那条命令，后面的词是 `query`。不归它管的人打的交回假，照原来交核心。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn pick_command(
        &mut self,
        message: &Message,
        kind: Kind,
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
        let word = |name: &str| words.get(&format!("{}-{name}", kind.key()), &lang);
        let listed = self.core.call(kind.list(), json!({})).await?;
        let listed = &listed["result"];
        let all = choices(kind, listed);
        let said = match asked(query) {
            Asked::Now => {
                let mut said = match self.card_value(message, kind) {
                    Some(id) => word("now").replace("{name}", &shown(&all, &id)),
                    None => match self.fallback(message, kind, listed) {
                        Some(id) => word("follows").replace("{name}", &shown(&all, &id)),
                        None => word("follows-plain"),
                    },
                };
                said.push('\n');
                said.push_str(&word("usage"));
                said
            }
            Asked::Clear => self.set_picked(message, kind, None, (&words, &lang)),
            Asked::Set(name) => match exact(kind, name, &all) {
                Some(choice) => self.set_picked(message, kind, Some(choice), (&words, &lang)),
                None => word("none").replace("{query}", name),
            },
            Asked::Search(keywords) => {
                let found = search(keywords, &all);
                if found.is_empty() {
                    word("search-none").replace("{query}", keywords)
                } else {
                    let using = self
                        .card_value(message, kind)
                        .or_else(|| self.fallback(message, kind, listed));
                    let mut said = word("found");
                    for (number, choice) in found {
                        said.push_str(&format!("\n{number}. {}", choice.shown()));
                        if using.as_deref() == Some(choice.id.as_str()) {
                            said.push_str(&word("using-mark"));
                        }
                    }
                    said
                }
            }
        };
        tracing::info!(target: TARGET, venue = %message.place.peer.venue, message = message.number, command = kind.key(), "command ran");
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

    /// 卡片上没写时跟随的：照规则套出来的；模型再没有的照核心的对话模型。
    fn fallback(&mut self, message: &Message, kind: Kind, listed: &Value) -> Option<String> {
        Venue::parse(&message.place.peer.venue)
            .and_then(|venue| {
                let applied = self.applied(&venue);
                super::super::applied::text_of(&applied, kind.key()).map(str::to_string)
            })
            .or_else(|| match kind {
                Kind::Model => listed["uses"]["chat"].as_str().map(str::to_string),
                _ => None,
            })
    }

    /// 这个场所自己的卡片：是哪个号的（空的是整个平台的那张）、上面的人格、模型、预设三格。这个连接的号的那张，没有的看整个
    /// 平台的那张；都没有的是空的。
    fn own_card(&self, message: &Message) -> Option<(Option<Id>, [Option<String>; 3])> {
        let peer = &message.place.peer;
        let platform = peer.platform.name();
        let cards = self.rules.cards();
        // 这个号的排在整个平台的前面，别的号的不算。
        let rank = |bot: &Option<Id>| match bot {
            Some(bot) if *bot == peer.bot => Some(0),
            None => Some(1),
            Some(_) => None,
        };
        let mut found: Vec<(u8, Option<Id>, [Option<String>; 3])> = match &peer.to {
            To::Group(group) => cards
                .groups
                .iter()
                .filter(|card| card.platform == platform && card.id == *group)
                .filter_map(|card| {
                    let set = &card.set;
                    let cells = [set.persona.clone(), set.model.clone(), set.preset.clone()];
                    rank(&card.bot).map(|rank| (rank, card.bot.clone(), cells))
                })
                .collect(),
            To::Private(user) => cards
                .privates
                .iter()
                .filter(|card| card.platform == platform && card.id == *user)
                .filter_map(|card| {
                    let set = &card.set;
                    let cells = [set.persona.clone(), set.model.clone(), set.preset.clone()];
                    rank(&card.bot).map(|rank| (rank, card.bot.clone(), cells))
                })
                .collect(),
        };
        found.sort_by_key(|(rank, ..)| *rank);
        found.into_iter().next().map(|(_, bot, cells)| (bot, cells))
    }

    /// 这个场所自己的卡片上 `kind` 那一格写的。
    fn card_value(&self, message: &Message, kind: Kind) -> Option<String> {
        let (_, [persona, model, preset]) = self.own_card(message)?;
        match kind {
            Kind::Model => model,
            Kind::Persona => persona,
            Kind::Preset => preset,
        }
    }

    /// 把这个场所的卡片 `kind` 那一格改成 `choice`（空的是去掉），交回回给他的那一句。
    fn set_picked(
        &mut self,
        message: &Message,
        kind: Kind,
        choice: Option<&Choice>,
        (words, lang): (&Words, &str),
    ) -> String {
        let word = |name: &str| words.get(&format!("{}-{name}", kind.key()), lang);
        let peer = &message.place.peer;
        let platform = peer.platform.name();
        // 有卡片的改那一张（这个号的，或者整个平台的）；没有的建一张这个号的。
        let bot = match self.own_card(message) {
            Some((bot, _)) => bot,
            None => Some(peer.bot.clone()),
        };
        let card = match &peer.to {
            To::Group(group) => {
                json!({"kind": "group", "platform": platform, "bot": bot, "id": group})
            }
            To::Private(user) => {
                json!({"kind": "private", "platform": platform, "bot": bot, "id": user})
            }
        };
        let mut values = serde_json::Map::new();
        values.insert(
            kind.key().to_string(),
            json!(choice.map(|choice| &choice.id)),
        );
        let saved = self.save_one(&json!({"card": card, "values": values}), true);
        match saved {
            Some(answer) if answer["saved"] == true => match choice {
                Some(choice) => word("set").replace("{name}", &choice.shown()),
                None => word("cleared"),
            },
            Some(answer) if answer.get("problems").is_some() => {
                let detail = answer["problems"][0]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                tracing::info!(target: TARGET, venue = %peer.venue, command = kind.key(), reason = "card_refused", "not picked");
                word("refused").replace("{detail}", &detail)
            }
            other => {
                tracing::warn!(target: TARGET, venue = %peer.venue, command = kind.key(), answer = ?other, "card not saved");
                word("no-card")
            }
        }
    }
}

/// `id` 给人看的写法：能选的里有的照它（「名字 (编号)」），没有的照原样。
fn shown(all: &[Choice], id: &str) -> String {
    all.iter()
        .find(|choice| choice.id == id)
        .map_or_else(|| id.to_string(), Choice::shown)
}
