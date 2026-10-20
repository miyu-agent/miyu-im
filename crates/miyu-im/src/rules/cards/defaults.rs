//! 卡片每一格的默认值（`onebot.md`「场所卡片」，`cards.get`）：卡片下面几层（出厂、配置项、系统的规则）套到这个场所上的样子，
//! 照卡片的格写。后台页照它画淡字带「默认」的值。
//!
//! - 没有规则设到的格是 `null`（页面写「不设」）；`on` 没设的是接话（出厂的群、私聊都不写 `allow`）。
//! - 插嘴照线路规程和抽样的千分比认档：`when-called` 是 `never`，千分比不到 35 是 `few`、不到 85 是 `some`、别的是 `many`。
//! - 来处只说是哪一层：`factory`、`setting`、`system`；照出厂参数来的（插嘴、去句号）是 `factory`。
//! - 卡上 `rules` 能写的每一个键（施工 I-7 中一，[`rules`]）：出厂参数的每一项，规则改了的照规则、没改的照出厂；
//!   顶层的几个（`lines::TOP`）和 `judge.model` 规则里有的照规则，没有的是 `null`。

use serde_json::{Map, Value, json};

use miyu_chat::Source;

use super::lines::TOP;
use crate::rules::{Applied, Values};

/// 出厂文件里可以不写的那一项：没写时照规则，规则也没有的是 `null`。
const OPTIONAL: &str = "judge.model";

/// 卡上 `rules` 能写的每一个键的默认值（模块的说明）：`{键: {value, from}}`。
pub fn rules(applied: &Applied, factory: &Values) -> Value {
    let mut keys = Map::new();
    for (key, value) in factory {
        let found =
            entry(applied, key).unwrap_or_else(|| json!({"value": value, "from": "factory"}));
        keys.insert((*key).to_string(), found);
    }
    for key in TOP.into_iter().chain([OPTIONAL]) {
        keys.insert(key.to_string(), plain(applied, key));
    }
    Value::Object(keys)
}

/// 群卡的默认值：`{格: {value, from}}`。
pub fn group(applied: &Applied) -> Value {
    let mut set = Map::new();
    let on = entry(applied, "allow").unwrap_or_else(|| json!({"value": true, "from": null}));
    set.insert("on".into(), on);
    for key in [
        "persona",
        "preset",
        "extra_prompt",
        "keywords",
        "managers",
        "sleep",
        "rate",
        "show_ids",
        "recent",
    ] {
        set.insert(key.into(), plain(applied, key));
    }
    set.insert("chatty".into(), chatty(applied));
    long_text(applied, &mut set);
    Value::Object(set)
}

/// 私聊卡的默认值。
pub fn private(applied: &Applied) -> Value {
    let mut set = Map::new();
    for key in ["persona", "preset", "extra_prompt", "show_ids"] {
        set.insert(key.into(), plain(applied, key));
    }
    long_text(applied, &mut set);
    Value::Object(set)
}

/// 「其他私聊」卡的默认值（施工 O-60）：私聊卡的几格，另有回复（没有规则设到的不回复）、限流。
pub fn other_private(applied: &Applied) -> Value {
    let mut set = private(applied);
    if let Value::Object(set) = &mut set {
        let on = entry(applied, "allow").unwrap_or_else(|| json!({"value": false, "from": null}));
        set.insert("on".into(), on);
        set.insert("rate".into(), plain(applied, "rate"));
        set.insert("sleep".into(), plain(applied, "sleep"));
    }
    set
}

/// 规则设到的一格；没设的 `{value: null, from: null}`。
fn plain(applied: &Applied, key: &str) -> Value {
    entry(applied, key).unwrap_or_else(|| json!({"value": null, "from": null}))
}

/// 规则设到的一格：值和来处；没设的是空的。
fn entry(applied: &Applied, key: &str) -> Option<Value> {
    applied
        .resolved
        .entries
        .get(key)
        .map(|entry| json!({"value": entry.value.json(), "from": layer(entry.origin.source)}))
}

/// 插嘴是哪一档。
fn chatty(applied: &Applied) -> Value {
    let discipline = applied.resolved.entries.get("discipline");
    if discipline.is_some_and(|entry| entry.value.json() == "when-called") {
        return json!({"value": "never", "from": discipline.map(|entry| layer(entry.origin.source))});
    }
    let level = match applied.params.chatty.probability() {
        0..35 => "few",
        35..85 => "some",
        _ => "many",
    };
    let from = applied
        .resolved
        .entries
        .get("chatty.probability")
        .map_or("factory", |entry| layer(entry.origin.source));
    json!({"value": level, "from": from})
}

/// 长文的两格（施工 O-59 上）、仅发送最终消息（施工 O-57）：出厂参数套上规则改的。
fn long_text(applied: &Applied, set: &mut Map<String, Value>) {
    let from = |key: &str| {
        applied
            .resolved
            .entries
            .get(key)
            .map_or("factory", |entry| layer(entry.origin.source))
    };
    set.insert(
        "long_text".into(),
        json!({"value": applied.params.long_text.name(), "from": from("outbound.long_text")}),
    );
    set.insert(
        "long_chars".into(),
        json!({"value": applied.params.long_chars, "from": from("outbound.long_chars")}),
    );
    set.insert(
        "final_only".into(),
        json!({"value": applied.params.final_only, "from": from("outbound.final_only")}),
    );
}

/// 哪一层。
fn layer(source: Source) -> &'static str {
    match source {
        Source::Factory => "factory",
        Source::Settings => "setting",
        Source::System => "system",
    }
}
