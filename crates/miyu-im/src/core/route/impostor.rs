//! 冒充提醒（施工 O-53，`onebot.md` 第一条「冒充提醒」，2026-10-17 项目主人定：识人术，免得她在群里被别人借名字带着走；施工 I-10 上：
//! 认借没借挪进驱动带的插件，`crate::plugins::Builtin::impostor`，OneBot 的识人术）：群里一个人这一条记下以后，这里交给开着的驱动带的
//! 插件（终端管理员在这个群用过的名字、她自己的群名片一起交过去），说借了谁的名字的，经核心的 `session.note` 记一块 `impostor` 事实
//! （模板 `facts/impostor.txt`，给模型看的字，登记在 `docs/ledger.md`），写着哪一条、借的是谁的名字。
//!
//! - 同一个人借同一个名字，这个群这次桥起来只记一块；换了名字、换了人再记。
//! - 终端管理员自己（小号也是：问到的对应表、在这个群说过的话带 `owner` 的）不算；白名单成员照算。

use std::collections::BTreeMap;
use std::collections::HashSet;

use miyu_client::ids::ExternalId;
use serde_json::json;

use super::Route;
use crate::TARGET;
use crate::chat::{Id, Platform};
use crate::core::{Gone, reason};
use crate::plugins::Seen;

/// 冒充提醒那一块事实的 `kind`。
const IMPOSTOR: &str = "impostor";

/// 记过的：群会话、发的人、借的名字。
#[derive(Debug, Default)]
pub(super) struct Noted {
    /// 记过的（群会话、发的人、借的名字）。
    noted: HashSet<(String, String, String)>,
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
        (platform, group, bot): (Platform, Id, Id),
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
        let admin_names = projection.admin_names();
        // 她自己的群名片：同场所消息的 `my_name`（施工 I-38 起两处共用，缓存里没有的问一次 NapCat，问不到的缓存时长以内不再问）。
        let own_card = self
            .my_name((platform, bot.clone()), Some(group.clone()))
            .await;
        let number = sender.as_str().rsplit(':').next().unwrap_or_default();
        let seen = Seen {
            name,
            number,
            admin_names: &admin_names,
            own_card: own_card.as_deref(),
        };
        let Some(claimed) = self.hooks.impostor((platform, &bot), group, &seen) else {
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
