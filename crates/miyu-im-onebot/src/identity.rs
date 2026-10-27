//! 识人术（施工 I-10 上，设计 01 第一节第 22 条；2026-10-19 项目主人定：OneBot 驱动里内置的 im 插件 `miyu-im-identity`，暂时只支持
//! QQ）：群里一个人的名字借了别人的，提醒 AI 这不是那个人（O-53 写在桥里的冒充提醒挪进来）。记那一句、同一个人借同一个名字只记一次
//! 由宿主做（`core/route/impostor.rs`），这里只认借没借、借的是谁。
//!
//! 设置是驱动的配置项（`im.qq.identity_*`，藏起来，在插件页「设置」里改）：
//!
//! - `identity_on`：总开关，默认开。
//! - `identity_admin`：保护终端管理员的名字（他在这个群用过的），默认开。
//! - `identity_card`：保护 AI 自己的群名片，默认开。
//! - `identity_people`：识人映射，一项写 `名字=QQ号`；名字被号码对不上的人用了就算借（同一个名字写了几个号的，哪一个都算本人）。
//!   写错的那一项不算。
//!
//! 比法照旧版：都转小写、去掉空白，含着就算（「老板本人」「米尤bot」也算）；不到两个字的名字不比（一个字到处撞）；借了几个的取头一个
//! （终端管理员的、她自己的、映射里的，照这个先后）。

use serde_json::{Map, Value};

use miyu_im::plugins::declared::{Links, Ui};
use miyu_im::plugins::{Builtin, Declared, Seen};

/// 编号。
const ID: &str = "miyu-im-identity";

/// 设置项名字的开头。
const PREFIX: &str = "identity_";

/// 保护终端管理员的名字。
pub(crate) const ADMIN: &str = "identity_admin";

/// 保护 AI 自己的群名片。
pub(crate) const CARD: &str = "identity_card";

/// 识人映射。
pub(crate) const PEOPLE: &str = "identity_people";

/// 总开关。
pub(crate) const ON: &str = "identity_on";

/// 比的名字最少几个字（去掉空白以后）。
const SHORTEST: usize = 2;

/// 识人术。
pub struct Identity;

impl Builtin for Identity {
    fn id(&self) -> &'static str {
        ID
    }

    fn declared(&self) -> Declared {
        let words = |zh: &str, en: &str, ja: &str| {
            [("zh", zh), ("en", en), ("ja", ja)]
                .into_iter()
                .map(|(lang, text)| (lang.to_string(), text.to_string()))
                .collect()
        };
        Declared {
            name: words("识人术", "Identity", "なりすまし防止"),
            summary: words(
                "认清谁是谁：有人借了别人的名字，提醒 AI 这不是那个人。",
                "Tells who is who: when someone borrows another person's name, reminds the AI it is not them.",
                "誰が誰かを見分けます。他人の名前を名乗る人がいたら、本人ではないと AI に伝えます。",
            ),
            links: Links {
                icon: Some("users".to_string()),
                ..Links::default()
            },
            platforms: vec!["qq".to_string()],
            // 识人术认的是群里借名字的（施工 I-51）。
            venues: vec!["group".to_string()],
            needs: Vec::new(),
            default: true,
            ui: Ui::default(),
        }
    }

    fn settings(&self) -> &'static str {
        PREFIX
    }

    fn impostor(&self, config: &Map<String, Value>, seen: &Seen<'_>) -> Option<String> {
        let switch = |name: &str| config.get(name).and_then(Value::as_bool).unwrap_or(true);
        let mut protected: Vec<String> = Vec::new();
        if switch(ADMIN) {
            protected.extend(seen.admin_names.iter().cloned());
        }
        if switch(CARD)
            && let Some(card) = seen.own_card
        {
            protected.push(card.to_string());
        }
        let people = people(config.get(PEOPLE));
        for (name, _) in &people {
            let theirs = people
                .iter()
                .any(|(other, number)| other == name && number == seen.number);
            if !theirs && !protected.contains(name) {
                protected.push(name.clone());
            }
        }
        borrowed(seen.name, &protected).cloned()
    }
}

/// 识人映射的每一项：`名字=号码`，名字去掉前后空白、不空，号码是数字；写错的不要。
pub(crate) fn people(value: Option<&Value>) -> Vec<(String, String)> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let (name, number) = item.as_str()?.split_once('=')?;
            let (name, number) = (name.trim(), number.trim());
            let digits = !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
            (!name.is_empty() && digits).then(|| (name.to_string(), number.to_string()))
        })
        .collect()
}

/// 照这一种写法比：转小写、去掉空白。
fn folded(name: &str) -> String {
    name.chars()
        .filter(|char| !char.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// `name` 借了 `protected` 里哪一个：照 [`folded`] 比，含着就算；不到 [`SHORTEST`] 个字的不比；借了几个的取头一个。
pub(crate) fn borrowed<'a>(name: &str, protected: &'a [String]) -> Option<&'a String> {
    let name = folded(name);
    protected.iter().find(|one| {
        let one = folded(one);
        one.chars().count() >= SHORTEST && name.contains(&one)
    })
}

#[cfg(test)]
mod tests;
