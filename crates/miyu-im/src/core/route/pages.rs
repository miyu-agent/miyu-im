//! 场所卡片的两页（施工 O-58 下，`onebot.md`「场所卡片的两页」）：清单的两块整页 `groups`、`friends`，内容照
//! `rules::card_page` 拼，经核心的 `widget.set` 推；三个方法 `cards.save_one`、`cards.add`、`cards.remove` 改卡片（私聊页的还改
//! 白名单），读的一头照推送交到这里。
//!
//! - 谁在看：核心发 `widget.watch {widget}` 的记下、当场推一份，`widget.unwatch` 的拿掉。卡片、白名单变了以后在看的都重推。
//! - 推：卡片、默认值当场算好，她在的群、好友、人格另起任务问齐了（同 `cards.get`）再拼、再推；核心拒了的记一行。
//! - 改白名单：经核心的 `config.set`（核心 T-9 补：扩展能逐项改自己的项），带 `expect`，有人同时改过的不覆盖。推来新的白名单时
//!   （`pushed`）照样重推。

use std::collections::BTreeSet;
use std::sync::Arc;

use serde_json::{Map, Value, json};

use super::Route;
use super::cards::lists;
use crate::TARGET;
use crate::chat::{Bots, Id};
use crate::core::caller::Caller;
use crate::core::{Gone, reason};
use crate::plugins::Toggles;
use crate::rules::card_page::{self, FRIEND_PREFIX, GROUPS};
use crate::rules::{Card, Cards, Edit, Known, Words, card_edit, save_cards};
use crate::settings::whitelist_key;

/// 在看的那几块。
#[derive(Debug, Default)]
pub(super) struct Pages {
    watching: BTreeSet<String>,
}

impl Route {
    /// 核心推来的通知是不是两页的 `widget.watch`、`widget.unwatch`：是的照办，交回真。
    pub(super) fn page_pushed(&mut self, pushed: &Value) -> bool {
        let widget = pushed["params"]["widget"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        match pushed["method"].as_str() {
            Some("widget.watch") => {
                self.pages.watching.insert(widget.clone());
                self.push_page(&widget);
                true
            }
            Some("widget.unwatch") => {
                self.pages.watching.remove(&widget);
                true
            }
            _ => false,
        }
    }

    /// 卡片、白名单变了：在看的都照新的推，叫一声 im 的会话页（施工 I-7 上）。
    pub(super) fn repush_pages(&mut self) {
        self.cards_changed.notify();
        let watching: Vec<String> = self.pages.watching.iter().cloned().collect();
        for widget in watching {
            self.push_page(&widget);
        }
    }

    /// 推一块：手头的当场算好，群、好友、人格另起任务问齐了再拼。认不得的块不推。
    fn push_page(&mut self, widget: &str) {
        if widget != GROUPS && widget != card_page::FRIENDS {
            return;
        }
        // 两页只画后台页那个平台的（施工 I-80：页面分平台是 I-85 的事）。
        let page = self.page_platform();
        let mut cards = self.rules.cards().clone();
        cards.groups.retain(|card| card.platform == page);
        cards.privates.retain(|card| card.platform == page);
        let whitelist = self.whitelist_numbers(page);
        let mut groups = Map::new();
        for card in &cards.groups {
            groups.insert(card.id.to_string(), self.group_defaults(page, &card.id));
        }
        let mut privates = Map::new();
        for id in &whitelist {
            privates.insert(id.to_string(), self.private_defaults(page, id));
        }
        let known = Known {
            whitelist,
            defaults: json!({
                "other_groups": self.group_defaults(page, &Id::from(0)),
                "other_privates": self.other_private_defaults(page),
                "groups": groups,
                "privates": privates,
            }),
            ..Known::default()
        };
        let job = Job {
            widget: widget.to_string(),
            cards,
            known,
            words: self.rules.page_words().clone(),
            lang: self.core.language.clone(),
        };
        let (bots, caller) = (self.card_bots(), self.core.caller());
        self.chores.spawn(push(job, bots, caller));
    }

    /// 白名单成员里平台 `platform` 上的号（`im.whitelist` 里 `<平台>:` 开头的）。
    pub(super) fn whitelist_numbers(&self, platform: &str) -> Vec<Id> {
        self.whitelist
            .iter()
            .filter_map(|identity| {
                let (on, number) = identity.split_once(':')?;
                (on == platform).then(|| Id::new(number)).flatten()
            })
            .collect()
    }

    /// 方法参数 `args` 说的是哪个平台（施工 I-80，同 I-70 上的规矩）：写了的照它（没挂的不认）；没写的只挂一个平台时是它，挂了
    /// 几个的不认（拒 `bad_params`）。
    fn method_platform(&self, args: &Value) -> Option<&'static str> {
        self.platforms
            .ours(args)
            .ok()
            .map(|platform| platform.name())
    }

    /// 核心转来的一次 `method.call`（`pushed`）是不是两页的三个方法：是的照办、答，交回真。
    ///
    /// # Errors
    ///
    /// 改白名单时写不出去、核心断开。
    pub(super) async fn page_called(&mut self, pushed: &Value) -> Result<bool, Gone> {
        let id = pushed["id"].clone();
        let args = &pushed["params"]["params"];
        let answer = match pushed["params"]["method"].as_str() {
            Some(card_page::SAVE_ONE) => self.save_one(args),
            Some(card_page::ADD) => self.add_card(args).await?,
            Some(card_page::REMOVE) => self.remove_card(args).await?,
            _ => return Ok(false),
        };
        match answer {
            Some(answer) => self.core.answerer().answer(&id, answer),
            None => self.core.answerer().refuse(&id, "bad_params"),
        }
        Ok(true)
    }

    /// `cards.save_one {card, values}`：部分更新那一张卡。白名单成员还没有卡的先建一张。参数写错的交回空的（拒 `bad_params`）。
    pub(in crate::core::route) fn save_one(&mut self, args: &Value) -> Option<Value> {
        let card = card_edit::card_of(&args["card"], self.method_platform(args))?;
        let platform = card.platform().unwrap_or(self.page_platform());
        let values = card_page::from_page(args["values"].as_object()?, platform);
        let mut cards = self.rules.cards().clone();
        if let Card::Private(platform, id) = &card
            && self.whitelist_numbers(platform).contains(id)
        {
            cards = card_edit::add(&cards, card.clone());
        }
        let words = self.rules.page_words().clone();
        let lang = self.core.language.clone();
        Some(match card_edit::update(&cards, card, &values) {
            Edit::Done(next) => self.store(*next, &words, &lang),
            Edit::Missing => json!({"saved": false, "message": words.get("missing", &lang)}),
            Edit::Problems(problems) => {
                let problems: Vec<Value> = problems
                    .iter()
                    .map(|(key, code)| json!({"key": key, "message": problem_text(&words, &lang, key, code)}))
                    .collect();
                tracing::info!(target: TARGET, problems = problems.len(), "venue card refused");
                json!({"saved": false, "problems": problems})
            }
        })
    }

    /// `cards.add {value, platform?}`：群号原样是群卡；`p:<号>` 是私聊：先把 `<平台>:<号>` 加进白名单。答 `{id}`。平台照
    /// [`Route::method_platform`]。
    async fn add_card(&mut self, args: &Value) -> Result<Option<Value>, Gone> {
        let (Some(value), Some(platform)) = (args["value"].as_str(), self.method_platform(args))
        else {
            return Ok(None);
        };
        let platform = platform.to_string();
        let words = self.rules.page_words().clone();
        let lang = self.core.language.clone();
        let (card, number) = match value.strip_prefix(FRIEND_PREFIX) {
            Some(rest) => match Id::new(rest) {
                Some(id) => (Card::Private(platform, id.clone()), id),
                None => return Ok(None),
            },
            None => match Id::new(value) {
                Some(id) => (Card::Group(platform, id.clone()), id),
                None => return Ok(None),
            },
        };
        if let Card::Private(platform, id) = &card
            && let Some(failed) = self
                .change_whitelist((platform, id), true, &words, &lang)
                .await?
        {
            return Ok(Some(failed));
        }
        let next = card_edit::add(self.rules.cards(), card);
        let stored = self.store(next, &words, &lang);
        if stored["saved"] == false {
            return Ok(Some(stored));
        }
        Ok(Some(json!({"id": number.to_string()})))
    }

    /// `cards.remove {card}`：删掉那一张；私聊的还把那个号移出白名单。「其他群」不收（拒 `bad_params`）。
    async fn remove_card(&mut self, args: &Value) -> Result<Option<Value>, Gone> {
        let Some(card) = card_edit::card_of(&args["card"], self.method_platform(args)) else {
            return Ok(None);
        };
        let Some(next) = card_edit::remove(self.rules.cards(), card.clone()) else {
            return Ok(None);
        };
        let words = self.rules.page_words().clone();
        let lang = self.core.language.clone();
        if let Card::Private(platform, id) = &card
            && let Some(failed) = self
                .change_whitelist((platform, id), false, &words, &lang)
                .await?
        {
            return Ok(Some(failed));
        }
        Ok(Some(self.store(next, &words, &lang)))
    }

    /// 存盘、换上、重推；写不进盘的答 `{saved: false, message}`。
    pub(super) fn store(&mut self, cards: Cards, words: &Words, lang: &str) -> Value {
        if let Err(error) = save_cards(self.rules.root(), &cards) {
            tracing::warn!(target: TARGET, error = %error, "venue cards not saved");
            return json!({"saved": false, "message": words.get("not_saved", lang)});
        }
        tracing::info!(target: TARGET, groups = cards.groups.len(), privates = cards.privates.len(), "venue cards saved");
        let before = self.wanted();
        self.rules.set_cards(cards);
        self.repin(&before);
        self.hooks
            .set_toggles(Toggles::from_cards(self.rules.cards()));
        self.forget_privates();
        self.repush_pages();
        json!({"saved": true})
    }

    /// 把平台 `platform` 上的 `id`（`<平台>:<号>`）加进（`add`）、移出白名单：经核心的 `config.set`，带 `expect`。已经是那样
    /// 的不改。成了交回空的、换上手里的白名单；不成的交回要答的 `{saved: false, message}`。
    async fn change_whitelist(
        &mut self,
        (platform, id): (&str, &Id),
        add: bool,
        words: &Words,
        lang: &str,
    ) -> Result<Option<Value>, Gone> {
        let identity = format!("{platform}:{id}");
        let mut next = self.whitelist.clone();
        let present = next.contains(&identity);
        match (add, present) {
            (true, true) | (false, false) => return Ok(None),
            (true, false) => next.push(identity),
            (false, true) => next.retain(|one| one != &identity),
        }
        let mut expect = json!({"value": self.whitelist});
        let mut reply = self.set_whitelist(&next, &expect).await?;
        // `expect` 查的是系统那一层里写着的原样：没写的是 `{}`。核心交回的那一层和桥看到的白名单是一回事的（没写而名单是空的、
        // 写的就是这一份），照它再试一次；对不上的才是别处改过。
        if reason(&reply) == Some("config_conflict") {
            let current = reply["error"]["data"]["current"].clone();
            let written = current.get("value").cloned().unwrap_or_else(|| json!([]));
            if current.is_object() && written == json!(self.whitelist) && current != expect {
                expect = current;
                reply = self.set_whitelist(&next, &expect).await?;
            }
        }
        if let Some(why) = reason(&reply) {
            tracing::warn!(target: TARGET, why, "whitelist not changed");
            let message = if why == "config_conflict" {
                words.get("whitelist_changed", lang)
            } else {
                words.get("whitelist_failed", lang).replace("{}", why)
            };
            return Ok(Some(json!({"saved": false, "message": message})));
        }
        tracing::info!(target: TARGET, count = next.len(), "whitelist changed by page");
        self.whitelist = next;
        Ok(None)
    }
}

impl Route {
    /// 经核心的 `config.set` 把系统那一层的白名单写成 `next`，那一层现在应当是 `expect`：交回核心的回应。
    async fn set_whitelist(&mut self, next: &[String], expect: &Value) -> Result<Value, Gone> {
        let change = json!({"key": whitelist_key(), "value": next, "expect": expect});
        self.core
            .call(
                "config.set",
                json!({"layer": "system", "changes": [change]}),
            )
            .await
    }
}

/// 一条问题写成给人看的一句：读不出、没有这一格的照原因说；规则写不对的照那一格该怎么写说（同它的提示），没有现成说法的
/// 只说写法不对，不把原因码给人看。
fn problem_text(words: &Words, lang: &str, key: &str, code: &str) -> String {
    let field = format!("problem.field.{key}");
    match code {
        "value" | "unknown_key" => words.get(&format!("problem.{code}"), lang),
        _ if words.has(&field) => words.get(&field, lang),
        _ => words.get("problem.other", lang),
    }
}

/// 推一块要的：哪一块、卡片、手头算好的、字、桥的语言。
struct Job {
    widget: String,
    cards: Cards,
    known: Known,
    words: Words,
    lang: String,
}

/// 问齐群、好友、人格，拼好推：核心拒了的记一行。
async fn push(job: Job, bots: Arc<Bots>, caller: Caller) {
    let lists = lists(&bots, &caller).await;
    let pairs = |rows: &[Value], id: &str| -> Vec<(Id, String)> {
        rows.iter()
            .filter_map(|row| {
                Some((
                    serde_json::from_value::<Id>(row[id].clone()).ok()?,
                    row["name"].as_str().unwrap_or_default().to_string(),
                ))
            })
            .collect()
    };
    let mut known = job.known;
    known.groups = pairs(&lists.groups, "id");
    known.friends = pairs(&lists.friends, "id");
    known.personas = lists
        .personas
        .iter()
        .filter_map(|row| {
            let id = row["persona"].as_str()?.to_string();
            let name = row["name"]
                .as_str()
                .map_or_else(|| id.clone(), str::to_string);
            Some((id, name))
        })
        .collect();
    let content = if job.widget == GROUPS {
        card_page::groups_page(&job.cards, &known, &job.words, &job.lang)
    } else {
        card_page::friends_page(&job.cards, &known, &job.words, &job.lang)
    };
    // 核心断开了（`Gone`）：跟核心的那一头会读到头、桥停下，这里没有别的要做。
    let params = json!({"widget": job.widget, "content": content});
    if let Ok(reply) = caller.call("widget.set", params).await
        && let Some(why) = reason(&reply)
    {
        let detail = &reply["error"]["data"];
        tracing::warn!(target: TARGET, widget = job.widget, why, detail = %detail, "page not pushed");
    }
}
