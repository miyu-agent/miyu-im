//! 冒充提醒（施工 O-53，`onebot.md` 第一条「冒充提醒」，2026-10-17 项目主人定：识人术，免得她在群里被别人借名字带着走）：群里一个人
//! 的名字里含着终端管理员在这个群用过的名字、或者她自己的名字（群成员缓存里她的群名片），他又不是那个人，就在他这一条记下以后
//! 经核心的 `session.note` 记一块 `impostor` 事实（模板 `facts/impostor.txt`，给模型看的字，登记在 `docs/ledger.md`），写着
//! 哪一条、借的是谁的名字。
//!
//! - 比法照旧版：都转小写、去掉空白，含着就算（不止一模一样：「老板本人」「米尤bot」也算）；不到两个字的名字不比（一个字到处撞）。
//! - 同一个人借同一个名字，这个群这次桥起来只记一块；换了名字、换了人再记。
//! - 终端管理员自己（小号也是：问到的对应表、在这个群说过的话带 `account` 的）不算；白名单成员照算。

use std::collections::BTreeMap;
use std::collections::HashSet;

use miyu_client::ids::ExternalId;
use serde_json::json;

use super::Route;
use crate::TARGET;
use crate::core::{Gone, reason};

/// 冒充提醒那一块事实的 `kind`。
const IMPOSTOR: &str = "impostor";

/// 比的名字最少几个字（去掉空白以后）。
const SHORTEST: usize = 2;

/// 照这一种写法比：转小写、去掉空白。
fn folded(name: &str) -> String {
    name.chars()
        .filter(|char| !char.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// `name` 借了 `protected` 里哪一个（纯逻辑）：照 [`folded`] 比，含着就算；不到 [`SHORTEST`] 个字的不比；借了几个的取头一个。
pub(super) fn borrowed<'a>(name: &str, protected: &'a [String]) -> Option<&'a String> {
    let name = folded(name);
    protected.iter().find(|one| {
        let one = folded(one);
        one.chars().count() >= SHORTEST && name.contains(&one)
    })
}

/// 记过的：群会话、发的人、借的名字；问过她群名片的群（缓存里没有的每个群只问一次 NapCat，问不到的不再问）。
#[derive(Debug, Default)]
pub(super) struct Noted {
    /// 记过的（群会话、发的人、借的名字）。
    noted: HashSet<(String, String, String)>,
    /// 问过她群名片的群。
    asked: HashSet<i64>,
}

impl Route {
    /// 群号是 `group` 的群会话 `session` 里，`sender` 叫 `name` 的平台编号 `msg` 那一条（收进它的机器人号 `bot`）：借了名字的记
    /// 一块事实（见模块开头）。终端管理员用过的名字照投影；他自己是不是终端管理员问核心的 `venue.binding`（记一阵），问不到的照
    /// 投影兜底。不收留着的推送：收了，判这一条时它就像重发的（「群里怎么叫她」第 8 条）。
    ///
    /// # Errors
    ///
    /// 写不出去、等的时候核心断开。
    pub(super) async fn impostor(
        &mut self,
        session: &str,
        (group, bot): (i64, i64),
        (sender, name, msg): (&ExternalId, &str, i64),
    ) -> Result<(), Gone> {
        if self.terminal_admin(sender.as_str()).await? == Some(true) {
            return Ok(());
        }
        let Some(projection) = self.groups.get(session) else {
            return Ok(());
        };
        if projection.is_admin(sender) {
            return Ok(());
        }
        let mut protected = projection.admin_names();
        // 她自己的群名片：缓存里没有的每个群问一次 NapCat（问到的记进缓存，问不到的不再问）。
        let mut asking = self.impostors.asked.insert(group);
        if let Some(mine) = self.member(group, bot, bot, &mut asking).await {
            protected.push(mine);
        }
        let Some(claimed) = borrowed(name, &protected).cloned() else {
            return Ok(());
        };
        let key = (
            session.to_string(),
            sender.as_str().to_string(),
            claimed.clone(),
        );
        if !self.impostors.noted.insert(key) {
            return Ok(());
        }
        let msg = msg.to_string();
        let fields = BTreeMap::from([("msg", msg.as_str()), ("name", claimed.as_str())]);
        let text = match self.rules.impostor_fact().render(&fields) {
            Ok(text) => text,
            Err(error) => {
                // 照说不会：起来时查过模板只要这两个字段（`rules/facts.rs`）。
                tracing::warn!(target: TARGET, session, error = %error, "impostor not written");
                return Ok(());
            }
        };
        let facts = json!([{"kind": IMPOSTOR, "text": text}]);
        let reply = self
            .core
            .call("session.note", json!({"session": session, "facts": facts}))
            .await?;
        match reason(&reply) {
            None => tracing::info!(target: TARGET, session, message = msg, "impostor noted"),
            Some(reason) => tracing::warn!(target: TARGET, session, reason, "note refused"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
