//! 一张卡拼成规则文件的几行（从 `cards.rs` 挪出来，施工 I-7 中一）：`[[rule]]`、匹配条件、设了的几格。
//!
//! - 卡上的格各写成一个规则键（`allow`、`persona`……，出厂参数的写成 `表.项`），同一张表的并成一行内联表（`outbound = { … }`）。
//! - 卡上的 `rules`（施工 I-7 中一：会话浮窗上那些没有单独一格的规则键）照写在最后：和上面的格是同一个键的，照 `rules` 的。键的
//!   写法不对的（[`key_ok`]）不写：键是照原样拼进 TOML 的，只收小写字母、数字、下划线和一个点（`card_edit` 存之前就拒）。
//! - 值照 JSON 的写法写进去：字照 JSON 的转义（TOML 基本字符串都认），数、真假照原样，列表、对象照 TOML 的数组、内联表。

use serde_json::Value;

use super::{Card, Chatty, GroupSet, LongText, OtherPrivateSet, PROBABILITY, PrivateSet};
use miyu_chat::{File, Source};

/// 只收在卡上 `rules` 里的顶层键（别的顶层键各有一格，或者不归卡管：`allow`、`match`）。私聊卡没有限流、睡眠两格，写在这里
/// （施工 I-7 中三下一，和忽略限流、忽略睡眠一起）。
pub(crate) const TOP: [&str; 7] = [
    "discipline",
    "parallel",
    "rate",
    "sleep",
    "ignore_rate",
    "ignore_sleep",
    "reaction",
];

/// 一张卡匹配哪些场所。
pub(super) enum Matcher {
    /// 会话默认值（施工 I-7 下二）：每一个场所。
    All,
    /// 「其他群」：群，但不是名单里的这几个（群号照 `, ` 连好）。名单里的群不继承它设的格（施工 O-58 上）。
    Others(String),
    /// 群卡、私聊卡：只配它自己。
    Own,
    /// 「其他私聊」：私聊，但不是这几个号（白名单成员、认出来的终端管理员，照 `, ` 连好，施工 O-60）。
    Strangers(String),
}

/// 一张卡的那一份：`[[rule]]`、匹配条件、设的几格。
pub(super) fn file(card: Card, matcher: &Matcher, lines: &[String]) -> File {
    let matcher = match (card, matcher) {
        (_, Matcher::Others(listed)) => format!("{{ kind = \"group\", not_group = [{listed}] }}"),
        (_, Matcher::Strangers(known)) => format!("{{ kind = \"private\", not_user = [{known}] }}"),
        (Card::Group(id), Matcher::Own) => format!("{{ kind = \"group\", group = [{id}] }}"),
        (Card::Private(id), Matcher::Own) => format!("{{ kind = \"private\", user = [{id}] }}"),
        (Card::OtherGroups, Matcher::Own) => "{ kind = \"group\" }".to_string(),
        (Card::OtherPrivates, Matcher::Own) => "{ kind = \"private\" }".to_string(),
        (_, Matcher::All) | (Card::Defaults, Matcher::Own) => "{}".to_string(),
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

/// 拼着的几个键：键、写成 TOML 的值，照先后。
#[derive(Default)]
struct Entries(Vec<(String, String)>);

impl Entries {
    /// 设了才记。
    fn put(&mut self, key: &str, value: Option<String>) {
        if let Some(value) = value {
            self.set(key, value);
        }
    }

    /// 记一个键：已经有的换掉值（位置不变）。
    fn set(&mut self, key: &str, value: String) {
        match self.0.iter_mut().find(|(one, _)| one == key) {
            Some(entry) => entry.1 = value,
            None => self.0.push((key.to_string(), value)),
        }
    }

    /// 卡上的 `rules` 照写在最后：键写法不对的、值是空的不写。
    fn rules(&mut self, rules: Option<&std::collections::BTreeMap<String, Value>>) {
        for (key, value) in rules.into_iter().flatten() {
            if key_ok(key) && !value.is_null() {
                self.set(key, toml(value));
            }
        }
    }

    /// 写成几行：`表.项` 的照表并成一行内联表，排在那张表头一回出现的位置。
    fn lines(self) -> Vec<String> {
        let mut tables: Vec<(String, Vec<String>)> = Vec::new();
        let mut order: Vec<Result<String, usize>> = Vec::new();
        for (key, value) in self.0 {
            match key.split_once('.') {
                Some((table, item)) => {
                    let item = format!("{item} = {value}");
                    match tables.iter_mut().position(|(name, _)| name == table) {
                        Some(at) => tables[at].1.push(item),
                        None => {
                            order.push(Err(tables.len()));
                            tables.push((table.to_string(), vec![item]));
                        }
                    }
                }
                None => order.push(Ok(format!("{key} = {value}"))),
            }
        }
        order
            .into_iter()
            .map(|line| match line {
                Ok(line) => line,
                Err(at) => format!("{} = {{ {} }}", tables[at].0, tables[at].1.join(", ")),
            })
            .collect()
    }
}

/// 卡上 `rules` 的键写得对不对：`表.项`（小写字母、数字、下划线，一个点）或者 [`TOP`] 里的一个。
pub(crate) fn key_ok(key: &str) -> bool {
    let word = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    };
    match key.split_once('.') {
        Some((table, item)) => word(table) && word(item),
        None => TOP.contains(&key),
    }
}

/// 群卡的几行：`allow` 一定写（群卡没写的接，「其他群」没写的不接），别的设了才写。
pub(super) fn group_lines(set: &GroupSet, listed: bool) -> Vec<String> {
    let mut entries = Entries::default();
    entries.set("allow", set.on.unwrap_or(listed).to_string());
    group_entries(&mut entries, set);
    entries.lines()
}

/// 会话默认值的几行（施工 I-7 下二）：同群卡，只是不写 `allow`（是否回复照各自的卡）。
pub(super) fn defaults_lines(set: &GroupSet) -> Vec<String> {
    let mut entries = Entries::default();
    group_entries(&mut entries, set);
    entries.lines()
}

/// 群卡除了 `allow` 的几格。
fn group_entries(entries: &mut Entries, set: &GroupSet) {
    common(entries, (&set.persona, &set.model, &set.extra_prompt));
    entries.put("keywords", set.keywords.as_deref().map(texts));
    match set.chatty {
        Some(Chatty::Never) => entries.set("discipline", quoted("when-called")),
        Some(level) => {
            let per_mille = PROBABILITY
                .iter()
                .find_map(|(known, value)| (*known == level).then_some(*value))
                .unwrap_or_default();
            entries.set("discipline", quoted("chatty"));
            entries.set("chatty.probability", per_mille.to_string());
        }
        None => {}
    }
    entries.put("managers", set.managers.as_deref().map(texts));
    entries.put("sleep", set.sleep.as_deref().map(quoted));
    entries.put("rate", set.rate.as_deref().map(quoted));
    entries.put("show_ids", set.show_ids.map(|show| show.to_string()));
    entries.put("recent", set.recent.map(|recent| recent.to_string()));
    outbound(entries, set.final_only, (set.long_text, set.long_chars));
    entries.rules(set.rules.as_ref());
}

/// 私聊卡的几行：设了才写。
pub(super) fn private_lines(set: &PrivateSet) -> Vec<String> {
    let mut entries = Entries::default();
    common(&mut entries, (&set.persona, &set.model, &set.extra_prompt));
    outbound(
        &mut entries,
        set.final_only,
        (set.long_text, set.long_chars),
    );
    entries.rules(set.rules.as_ref());
    entries.lines()
}

/// 「其他私聊」的几行（施工 O-60）：`allow` 一定写（没写的不回复），别的设了才写。
pub(super) fn other_private_lines(set: &OtherPrivateSet) -> Vec<String> {
    let mut entries = Entries::default();
    entries.set("allow", set.on.unwrap_or(false).to_string());
    common(&mut entries, (&set.persona, &set.model, &set.extra_prompt));
    entries.put("rate", set.rate.as_deref().map(quoted));
    outbound(
        &mut entries,
        set.final_only,
        (set.long_text, set.long_chars),
    );
    entries.rules(set.rules.as_ref());
    entries.lines()
}

/// 几种卡都有的三格：人格、对话模型、附加提示词。
fn common(
    entries: &mut Entries,
    (persona, model, extra): (&Option<String>, &Option<String>, &Option<String>),
) {
    entries.put("persona", persona.as_deref().map(quoted));
    entries.put("model", model.as_deref().map(quoted));
    entries.put("extra_prompt", extra.as_deref().map(quoted));
}

/// 仅发送最终消息、长文（出站参数的几项）。去句号是通用的配置项（O-58 补二）。
fn outbound(
    entries: &mut Entries,
    final_only: Option<bool>,
    (long_text, long_chars): (Option<LongText>, Option<u32>),
) {
    entries.put("outbound.final_only", final_only.map(|on| on.to_string()));
    entries.put(
        "outbound.long_text",
        long_text.map(|kind| quoted(kind.name())),
    );
    entries.put(
        "outbound.long_chars",
        long_chars.map(|chars| chars.to_string()),
    );
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

/// 一个 JSON 的值写成 TOML 的：数组、对象照 TOML 的数组、内联表（对象的键照 JSON 的写法加引号）。空的写成空字（照说不会：
/// 写之前去掉了）。
fn toml(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(toml).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(key, value)| format!("{} = {}", quoted(key), toml(value)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
        Value::Null => quoted(""),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests;
