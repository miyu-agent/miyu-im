//! 机器人的名字、头像（施工 I-6 中，设计 01 第八节第 2 条：连接卡上「头像是机器人账号在平台上的头像，卡上写名字」）。
//!
//! - 名字：页面问连接（`connections`）时，号还没问过名字的另起任务问平台（`Link::nickname`，OneBot 是 `get_login_info`），这次
//!   先不带；问到了记下、叫一声 `changed`，页面再问时带上。连上时不问：NapCat 那边只在有人看页面时多一个动作。没问成的下次
//!   再问。
//! - 头像：页面只能连同源（内容安全策略），由页面服务转：`GET /avatar/<号>`，号是连着的或者断开了的，照 `bridge.json` 的
//!   `avatar_user_url` 换号去取（同查头像的工具，`crate::media::bytes`），取到了记在内存里，以后照它给。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::Site;
use crate::TARGET;
use crate::chat::Source;
use crate::media::{bytes, extension};

/// 一张头像：字节和 `Content-Type`。
pub(super) type Avatar = Arc<(Vec<u8>, &'static str)>;

/// 记下的名字、头像。
#[derive(Default)]
pub(super) struct Faces {
    /// 号 → 问到的名字（平台回了没有的是空的）。
    names: Mutex<HashMap<i64, Option<String>>>,
    /// 正在问名字的号。
    asking: Mutex<HashSet<i64>>,
    /// 号 → 头像。
    avatars: Mutex<HashMap<i64, Avatar>>,
}

impl Faces {
    /// 号 `bot` 的名字：问到了的交回；没问过、连着的另起任务问（问到了叫一声 `changed`），这次是空的。
    pub(super) fn name(&self, site: &Arc<Site>, bot: i64) -> Option<String> {
        if let Some(name) = lock(&self.names).get(&bot) {
            return name.clone();
        }
        let link = site.running.bots.get(bot)?;
        if !lock(&self.asking).insert(bot) {
            return None;
        }
        let site = Arc::clone(site);
        tokio::spawn(async move {
            let asked = link.nickname().await;
            lock(&site.faces.asking).remove(&bot);
            match asked {
                Ok(name) => {
                    lock(&site.faces.names).insert(bot, name);
                    site.changed.notify();
                }
                Err(error) => {
                    tracing::debug!(target: TARGET, bot, error = ?error, "bot name unknown")
                }
            }
        });
        None
    }

    /// 号 `bot` 的头像：记下的照它；没有的、号是连着的或者断开了的，照模板去取。取不到的是空的。
    pub(super) async fn avatar(&self, site: &Site, bot: i64) -> Option<Avatar> {
        if let Some(avatar) = lock(&self.avatars).get(&bot) {
            return Some(Arc::clone(avatar));
        }
        let bots = &site.running.bots;
        let known = bots.get(bot).is_some() || bots.gone().iter().any(|(gone, _)| *gone == bot);
        let template = &site.running.tuning.avatar_user_url;
        if !known || template.is_empty() {
            return None;
        }
        let url = template.replace("{id}", &bot.to_string());
        let got = match bytes(&[Source::Url(url)], site.running.tuning.fetch()).await {
            Ok(got) => got,
            Err(why) => {
                tracing::debug!(target: TARGET, bot, why, "bot avatar not fetched");
                return None;
            }
        };
        let kind = match extension(&got) {
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };
        let avatar: Avatar = Arc::new((got, kind));
        lock(&self.avatars).insert(bot, Arc::clone(&avatar));
        Some(avatar)
    }
}

/// 锁里不 `await`、不会崩；真崩了，表照样能用。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
