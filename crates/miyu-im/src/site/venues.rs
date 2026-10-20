//! 会话页要的方法（施工 I-7 上，设计 01 第八节第 3 条）：卡片墙、新建、浮窗读写的都是场所卡片，和核心的后台页同一套（`cards.get`、
//! `cards.save_one`、`cards.add`、`cards.remove`，在 `crate::core::route`；施工 I-7 上补多「概况」的 `cards.overview`）。页面服务不另写一份：在这份登录的人的连接上经核心的
//! `package.call` 调 im 自己（设计第九节第 2 条：人的权限，核心照人查），读的一头照核心转来的 `method.call` 答，答的原样交回页面。
//!
//! 卡片、白名单变了，读的一头叫一声 `Site::cards`，开着的通道推 `{"event": "cards"}`（`channel`）。
//!
//! 会话浮窗选模型（施工 I-7 中一）：`models` 在人的连接上问核心的 `model.list`，只交页面要的：`models`（配好的每个模型的引用
//! `<供应商>/<模型>`，用不了的另带 `state`）、`pools`（池的名字）、`uses`（核心的对话、看图模型配的引用）。

use serde_json::{Value, json};

use super::Person;
use crate::PACKAGE;
use crate::core::methods::{CARDS_GET, CARDS_OVERVIEW};
use crate::rules::card_page::{ADD, REMOVE, SAVE_ONE};

/// 转给读的一头的几个方法。
pub(super) const FORWARDED: [&str; 5] = [CARDS_GET, SAVE_ONE, ADD, REMOVE, CARDS_OVERVIEW];

/// 在 `person` 的连接上调 im 自己的 `method`：交回答的，拒了的交回（原因、原话）。
pub(super) async fn forward(
    person: &Person,
    method: &str,
    params: &Value,
) -> Result<Value, (String, String)> {
    let params = json!({"package": PACKAGE, "method": method, "params": params});
    person
        .lock()
        .await
        .call("venues", "package.call", params)
        .await
        .map_err(|why| (why.reason, why.message))
}

/// `models`（模块的说明）：核心拒了的交回（原因、原话）。
pub(super) async fn models(person: &Person) -> Result<Value, (String, String)> {
    let listed = person
        .lock()
        .await
        .call("models", "model.list", json!({}))
        .await
        .map_err(|why| (why.reason, why.message))?;
    Ok(picked(&listed))
}

/// `presets`（施工 I-16，会话浮窗「预设」那一格）：核心 `preset.list` 里写对了的那几个，`{presets: [{id, name}]}`；名字没写的照编号。
pub(super) async fn presets(person: &Person) -> Result<Value, (String, String)> {
    let listed = person
        .lock()
        .await
        .call("presets", "preset.list", json!({}))
        .await
        .map_err(|why| (why.reason, why.message))?;
    Ok(preset_choices(&listed))
}

/// 核心 `preset.list` 的回答 `listed` 里写对了的预设：编号、名字（没写的照编号）。写错了的（带 `problem`）不列。
pub(super) fn preset_choices(listed: &Value) -> Value {
    let presets: Vec<Value> = listed["presets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|one| one.get("problem").is_none())
        .filter_map(|one| {
            let id = one["preset"].as_str()?;
            let name = one["name"]
                .as_str()
                .filter(|name| !name.is_empty())
                .unwrap_or(id);
            Some(json!({"id": id, "name": name}))
        })
        .collect();
    json!({"presets": presets})
}

/// 核心 `model.list` 的回答 `listed` 里页面要的那几样（模块的说明）。
pub(super) fn picked(listed: &Value) -> Value {
    let models: Vec<Value> = listed["providers"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|provider| provider["models"].as_array().into_iter().flatten())
        .filter_map(|model| {
            let reference = model["ref"].as_str()?;
            let mut one = json!({"ref": reference});
            if let Some(state) = model["state"].as_str().filter(|state| *state != "ok") {
                one["state"] = json!(state);
            }
            Some(one)
        })
        .collect();
    let pools: Vec<Value> = listed["pools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|pool| pool["name"].as_str().map(|name| json!(name)))
        .collect();
    json!({"models": models, "pools": pools, "uses": listed["uses"]})
}
