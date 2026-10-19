//! 会话页要的方法（施工 I-7 上，设计 01 第八节第 3 条）：卡片墙、新建、浮窗读写的都是场所卡片，和核心的后台页同一套（`cards.get`、
//! `cards.save_one`、`cards.add`、`cards.remove`，在 `crate::core::route`；施工 I-7 上补多「概况」的 `cards.overview`）。页面服务不另写一份：在这份登录的人的连接上经核心的
//! `package.call` 调 im 自己（设计第九节第 2 条：人的权限，核心照人查），读的一头照核心转来的 `method.call` 答，答的原样交回页面。
//!
//! 卡片、白名单变了，读的一头叫一声 `Site::cards`，开着的通道推 `{"event": "cards"}`（`channel`）。

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
