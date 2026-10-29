//! 场所卡片的两页（施工 O-58 下，`onebot.md`「场所卡片的两页」）：「群」「私聊」两块整页的内容，照核心的界面块写法（`widgets.md`
//! 「整页里能改」：`sections`、`form`）。纯逻辑：卡片、她在的群、好友、人格、卡片下面几层的默认值都由调的一方交进来。
//!
//! - 一页是跳到另一页的按钮加 `sections`（按钮在前：网页画在列表上面一行，终端也好找）。一张卡是 `sections` 的一项：小标题加表单，一组一张（群卡四组、私聊卡两组），各自保存，底下一个删掉的按钮（「其他群」没有）。
//! - 每一格的 `value` 是卡片上写的，没写的 `null`；`default.text` 照默认值写成「默认 · …」。管理的人在页面上只写号，存的时候补上
//!   `qq:`（[`from_page`]）。
//! - 给人看的字照 `data/cards/human/page.json`（[`Words`]），三种语言并列交给核心照每个头的语言挑；编成 JSON 超过 [`MAX_BYTES`] 的
//!   换成只有桥这时的语言，还超的从后面去掉几项。

use serde_json::{Map, Value, json};

use super::Cards;
use crate::chat::Id;

mod parts;
mod words;

use parts::{
    choices, control_name, default_value, name_of, name_of_persona, page_value, to_object,
};
pub use words::Words;

/// 两页的编号（清单 `[[widget]]` 的 `id`）。
pub const GROUPS: &str = "groups";

/// 「私聊」那一页。
pub const FRIENDS: &str = "friends";

/// 三种语言。
const LANGS: [&str; 3] = ["zh", "en", "ja"];

/// 一块最多多大（核心 64 KiB，留出余量）。
pub const MAX_BYTES: usize = 60 * 1024;

/// 下拉、加一项的选项最多几个（核心的上限）。
const MAX_OPTIONS: usize = 100;

/// 一组最多几项（核心的上限）。
const MAX_ITEMS: usize = 200;

/// 存一张表单调的方法。
pub const SAVE_ONE: &str = "cards.save_one";

/// 加一项调的方法。
pub const ADD: &str = "cards.add";

/// 删一项调的方法。
pub const REMOVE: &str = "cards.remove";

/// 私聊页加一项的值的前缀：`p:<号>`；群页的值是群号原样。
pub const FRIEND_PREFIX: &str = "p:";

/// 画哪几种语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Langs<'a> {
    /// 三种并列，核心照头挑。
    All,
    /// 只有一种（太大的时候）。
    One(&'a str),
}

/// 拼的时候要的：她在的群、好友（号、名字）、人格（编号、名字）、白名单成员的号、卡片下面几层的默认值（`cards.get` 的
/// `defaults` 的样子：`{"other_groups": {格: {value, from}}, "other_privates": {…}, "groups": {号: …}, "privates": {号: …}}`）。
#[derive(Debug, Clone, Default)]
pub struct Known {
    /// 她在的群：号、群名。
    pub groups: Vec<(Id, String)>,
    /// 好友：号、昵称。
    pub friends: Vec<(Id, String)>,
    /// 人格：编号、名字。
    pub personas: Vec<(String, String)>,
    /// 白名单成员的号。
    pub whitelist: Vec<Id>,
    /// 卡片下面几层的默认值。
    pub defaults: Value,
}

/// 一格的控件。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Toggle,
    Persona,
    Textarea,
    List,
    Chatty,
    Text,
    LongText,
    Number,
}

/// 一组：小标题的键、几格。
type Form = (&'static str, &'static [(&'static str, Control)]);

/// 群卡、「其他群」卡的四组。
const GROUP_FORMS: [Form; 4] = [
    (
        "chat",
        &[
            ("on", Control::Toggle),
            ("persona", Control::Persona),
            ("extra_prompt", Control::Textarea),
        ],
    ),
    (
        "call",
        &[
            ("keywords", Control::List),
            ("chatty", Control::Chatty),
            ("managers", Control::List),
            ("show_ids", Control::Toggle),
        ],
    ),
    ("pace", &[("sleep", Control::Text), ("rate", Control::Text)]),
    (
        "out",
        &[
            ("final_only", Control::Toggle),
            ("long_text", Control::LongText),
            ("long_chars", Control::Number),
        ],
    ),
];

/// 「其他私聊」卡的三组（施工 O-60）：对话多回复，另有节奏（限流）。
const OTHER_PRIVATE_FORMS: [Form; 3] = [
    (
        "chat",
        &[
            ("on", Control::Toggle),
            ("persona", Control::Persona),
            ("extra_prompt", Control::Textarea),
        ],
    ),
    ("pace", &[("rate", Control::Text)]),
    (
        "out",
        &[
            ("final_only", Control::Toggle),
            ("long_text", Control::LongText),
            ("long_chars", Control::Number),
        ],
    ),
];

/// 私聊卡的两组。
const PRIVATE_FORMS: [Form; 2] = [
    (
        "chat",
        &[
            ("persona", Control::Persona),
            ("extra_prompt", Control::Textarea),
        ],
    ),
    (
        "out",
        &[
            ("final_only", Control::Toggle),
            ("long_text", Control::LongText),
            ("long_chars", Control::Number),
        ],
    ),
];

/// 插嘴四档、长文两种：值和字的键。
const CHATTY: [&str; 4] = ["never", "few", "some", "many"];
const LONG_TEXT: [&str; 3] = ["split", "image", "forward"];

/// 「群」那一页：`lang` 是桥这时的语言（太大时只画它）。
pub fn groups_page(cards: &Cards, known: &Known, words: &Words, lang: &str) -> Value {
    fit(lang, |langs, count| {
        let ink = Ink {
            words,
            langs,
            known,
        };
        let mut items = vec![ink.item(
            Item {
                id: "other".into(),
                title: ink.word("other_groups"),
                subtitle: ink.word("other_groups_sub"),
                card: json!({"kind": "other_groups"}),
                removable: false,
            },
            &to_object(&cards.other_groups),
            &known.defaults["other_groups"],
            &GROUP_FORMS,
        )];
        // `count` 是一页最多几项：「其他群」占一项。
        for card in cards
            .groups
            .iter()
            .take(count.min(MAX_ITEMS).saturating_sub(1))
        {
            let number = card.id.to_string();
            let name = name_of(&known.groups, &card.id).unwrap_or_else(|| number.clone());
            items.push(ink.item(
                Item {
                    id: number.clone(),
                    title: json!(name),
                    subtitle: json!(number),
                    card: json!({"kind": "group", "id": card.id}),
                    removable: true,
                },
                &to_object(&card.set),
                &known.defaults["groups"][&number],
                &GROUP_FORMS,
            ));
        }
        let carded: Vec<Id> = cards.groups.iter().map(|card| card.id.clone()).collect();
        let options = choices(&known.groups, &carded, "");
        let sections = sections(items, ink.word("add_group"), options);
        json!([{"type": "buttons", "items": [{"label": ink.word("to_friends"), "action": {"open": FRIENDS}}]}, sections])
    })
}

/// 「私聊」那一页：白名单成员每人一项（没有卡的照空卡画）。
pub fn friends_page(cards: &Cards, known: &Known, words: &Words, lang: &str) -> Value {
    fit(lang, |langs, count| {
        let ink = Ink {
            words,
            langs,
            known,
        };
        // 「其他私聊」在最前面（施工 O-60），没有删掉。
        let other = ink.item(
            Item {
                id: "other".into(),
                title: ink.word("other_privates"),
                subtitle: ink.word("other_privates_sub"),
                card: json!({"kind": "other_privates"}),
                removable: false,
            },
            &to_object(&cards.other_privates),
            &known.defaults["other_privates"],
            &OTHER_PRIVATE_FORMS,
        );
        let members = known
            .whitelist
            .iter()
            .take(count.min(MAX_ITEMS).saturating_sub(1))
            .map(|id| {
                let number = id.to_string();
                let set = cards
                    .privates
                    .iter()
                    .find(|card| card.id == *id)
                    .map(|card| to_object(&card.set))
                    .unwrap_or_default();
                let name = name_of(&known.friends, id).unwrap_or_else(|| number.clone());
                ink.item(
                    Item {
                        id: number.clone(),
                        title: json!(name),
                        subtitle: json!(number),
                        card: json!({"kind": "private", "id": id}),
                        removable: true,
                    },
                    &set,
                    &known.defaults["privates"][&number],
                    &PRIVATE_FORMS,
                )
            });
        let items: Vec<Value> = std::iter::once(other).chain(members).collect();
        let options = choices(&known.friends, &known.whitelist, FRIEND_PREFIX);
        let sections = sections(items, ink.word("add_friend"), options);
        json!([{"type": "buttons", "items": [{"label": ink.word("to_groups"), "action": {"open": GROUPS}}]}, sections])
    })
}

/// 页面交来的一张表单的值换成卡片的写法：管理的人只写了号的补上 `qq:`。
pub fn from_page(values: &Map<String, Value>) -> Map<String, Value> {
    let mut values = values.clone();
    if let Some(Value::Array(list)) = values.get_mut("managers") {
        for one in list.iter_mut() {
            if let Some(text) = one.as_str()
                && !text.is_empty()
                && text.bytes().all(|byte| byte.is_ascii_digit())
            {
                *one = json!(format!("qq:{text}"));
            }
        }
    }
    values
}

/// 先三种语言并列；超了换成只有 `lang`；还超的从后面去掉几项。`build` 收画哪几种语言、一页最多几项。
fn fit(lang: &str, build: impl Fn(Langs<'_>, usize) -> Value) -> Value {
    let all = build(Langs::All, usize::MAX);
    if size(&all) <= MAX_BYTES {
        return all;
    }
    let mut count = usize::MAX;
    loop {
        let one = build(Langs::One(lang), count);
        let shown = one[1]["items"].as_array().map_or(0, Vec::len);
        let bytes = size(&one);
        if bytes <= MAX_BYTES || shown <= 1 {
            return one;
        }
        // 照超出的比例一次缩到位（每项差不多大），再少一项留余量。
        count = (shown * MAX_BYTES / bytes)
            .saturating_sub(1)
            .min(shown - 1)
            .max(1);
    }
}

/// 编成 JSON 多少字节。
fn size(content: &Value) -> usize {
    content.to_string().len()
}

/// 一项的身份。
struct Item {
    id: String,
    title: Value,
    subtitle: Value,
    card: Value,
    removable: bool,
}

/// 拼字的那一份：字、画哪几种语言、拼的时候要的。
struct Ink<'a> {
    words: &'a Words,
    langs: Langs<'a>,
    known: &'a Known,
}

impl Ink<'_> {
    /// 一处字：照每种语言算。
    fn text(&self, each: impl Fn(&str) -> String) -> Value {
        match self.langs {
            Langs::All => Value::Object(
                LANGS
                    .iter()
                    .map(|lang| ((*lang).to_string(), json!(each(lang))))
                    .collect(),
            ),
            Langs::One(lang) => json!(each(lang)),
        }
    }

    /// 一句现成的字。
    fn word(&self, key: &str) -> Value {
        self.text(|lang| self.words.get(key, lang))
    }

    /// 一张卡：几组小标题加表单，删得掉的底下一个按钮。`state` 照接不接话（群卡、「其他群」）。
    fn item(
        &self,
        item: Item,
        set: &Map<String, Value>,
        defaults: &Value,
        forms: &[Form],
    ) -> Value {
        let mut blocks = Vec::new();
        for (name, fields) in forms {
            blocks.push(json!({"type": "heading", "text": self.word(&format!("heading.{name}"))}));
            let fields: Vec<Value> = fields
                .iter()
                .map(|(key, control)| {
                    self.field(key, *control, set.get(*key), &defaults[*key]["value"])
                })
                .collect();
            blocks.push(json!({
                "type": "form",
                "id": format!("{name}-{}", item.id),
                "fields": fields,
                "submit": {"label": self.word("save"), "call": SAVE_ONE, "params": {"card": item.card}},
            }));
        }
        if item.removable {
            blocks.push(json!({"type": "buttons", "items": [{"label": self.word("remove"), "action": {"call": REMOVE, "params": {"card": item.card}}}]}));
        }
        let mut section = json!({"id": item.id, "title": item.title, "subtitle": item.subtitle, "blocks": blocks});
        // 有「回复」这一格的卡（群卡、两张「其他」）画状态点。
        if forms
            .iter()
            .any(|(_, fields)| fields.iter().any(|(key, _)| *key == "on"))
        {
            let on = set
                .get("on")
                .or(defaults["on"].get("value"))
                .and_then(Value::as_bool)
                .unwrap_or(true);
            section["state"] = json!(if on { "good" } else { "off" });
        }
        section
    }

    /// 一格。
    fn field(&self, key: &str, control: Control, value: Option<&Value>, default: &Value) -> Value {
        let mut field = json!({
            "key": key,
            "label": self.word(&format!("field.{key}")),
            "control": control_name(control),
            "value": page_value(key, value),
            "default": {"text": self.text(|lang| self.words.get("default", lang).replace("{}", &self.shown(key, control, default, lang)))},
        });
        let hint = format!("hint.{key}");
        if self.words.has(&hint) {
            field["hint"] = self.word(&hint);
        }
        match control {
            Control::Persona => field["options"] = self.personas(value),
            Control::Chatty => field["options"] = self.options("chatty", &CHATTY),
            Control::LongText => field["options"] = self.options("long_text", &LONG_TEXT),
            Control::Number => {
                field["min"] = json!(0);
                field["max"] = json!(100_000);
                field["integer"] = json!(true);
            }
            Control::Toggle | Control::Textarea | Control::List | Control::Text => {}
        }
        // 默认值本身（核心 X-15 三补）：头照它画 `null` 的样子、点亮默认那一档。类型对不上控件的、下拉里没有的、没有的不写。
        if let Some(shown) = default_value(key, control, default, &field["options"]) {
            field["default"]["value"] = shown;
        }
        field
    }

    /// 默认值写成一句：没有的「不设」，开关「开」「关」，列表用「、」连，选项照它的字，人格照名字。
    fn shown(&self, key: &str, control: Control, value: &Value, lang: &str) -> String {
        match (value, control) {
            (Value::Null, _) => self.words.get("unset", lang),
            (Value::Bool(on), _) => self.words.get(if *on { "on" } else { "off" }, lang),
            (Value::Array(list), _) => {
                let shown: Vec<String> = list
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|one| one.strip_prefix("qq:").unwrap_or(one).to_string())
                    .collect();
                if shown.is_empty() {
                    self.words.get("unset", lang)
                } else {
                    shown.join("、")
                }
            }
            (Value::String(text), Control::Chatty | Control::LongText) => {
                self.words.get(&format!("{key}.{text}"), lang)
            }
            (Value::String(text), Control::Persona) => {
                name_of_persona(&self.known.personas, text).unwrap_or_else(|| text.clone())
            }
            (Value::String(text), _) => text.clone(),
            (other, _) => other.to_string(),
        }
    }

    /// 人格的选项：核心列的，卡片上写着、列表里没有的（删了的）也放进去，免得一打开就被改掉。
    fn personas(&self, value: Option<&Value>) -> Value {
        let mut options: Vec<Value> = self
            .known
            .personas
            .iter()
            .take(MAX_OPTIONS)
            .map(|(id, name)| json!({"value": id, "label": name}))
            .collect();
        if let Some(current) = value.and_then(Value::as_str)
            && !self.known.personas.iter().any(|(id, _)| id == current)
        {
            options.truncate(MAX_OPTIONS - 1);
            options.push(json!({"value": current, "label": current}));
        }
        Value::Array(options)
    }

    /// 固定的几个选项：值和字。
    fn options(&self, prefix: &str, values: &[&str]) -> Value {
        values
            .iter()
            .map(|value| json!({"value": value, "label": self.word(&format!("{prefix}.{value}"))}))
            .collect()
    }
}

/// `sections` 一段：没有可加的不写 `add`。
fn sections(items: Vec<Value>, label: Value, options: Vec<Value>) -> Value {
    let mut sections = json!({"type": "sections", "items": items});
    if !options.is_empty() {
        sections["add"] = json!({"label": label, "call": ADD, "options": options});
    }
    sections
}

#[cfg(test)]
mod tests;
