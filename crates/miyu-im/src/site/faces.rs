//! 机器人的名字、头像（施工 I-6 中，设计 01 第八节第 2 条：连接卡上「头像是机器人账号在平台上的头像，卡上写名字」）。
//!
//! - 名字：页面问连接（`connections`）时，号还没问过名字的另起任务问平台（`Link::nickname`，OneBot 是 `get_login_info`），这次
//!   先不带；问到了记下、叫一声 `changed`，页面再问时带上。连上时不问：NapCat 那边只在有人看页面时多一个动作。没问成的下次
//!   再问。
//! - 头像：页面只能连同源（内容安全策略），由页面服务转：`GET /avatar/<号>`，号是连着的或者断开了的，照 `bridge.json` 的
//!   `avatar_user_url` 换号去取（同查头像的工具，`crate::media::bytes`），取到了记在内存里，以后照它给。
//! - 会话卡上的头像（施工 I-7 上）：`GET /avatar/user/<号>`、`GET /avatar/group/<群号>`，照 `avatar_user_url`、
//!   `avatar_group_url` 取，不查号在不在名单里（地址只是平台的头像模板换个号）。记下的超过 [`MOST`] 张先全清了再记。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::Site;
use crate::TARGET;
use crate::chat::{Face, Id};
use crate::media::{bytes, extension};

/// 一张头像：字节和 `Content-Type`。
pub(super) type Avatar = Arc<(Vec<u8>, &'static str)>;

/// 头像最多记多少张。
const MOST: usize = 512;

/// 谁的头像。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Whose {
    /// 机器人的号。
    Bot(Id),
    /// 会话里一个人的号。
    User(Id),
    /// 群号。
    Group(Id),
}

impl Whose {
    /// 照 `/avatar/` 后面的路径认：`<号>` 是机器人，`user/<号>`、`group/<号>`。号照 [`Id`] 的规矩查（平台认不认得由驱动定，施工 I-82）。
    pub(super) fn parse(rest: &str) -> Option<Whose> {
        let number = Id::new;
        match rest.split_once('/') {
            None => number(rest).map(Whose::Bot),
            Some(("user", id)) => number(id).map(Whose::User),
            Some(("group", id)) => number(id).map(Whose::Group),
            Some(_) => None,
        }
    }
}

/// 记下的名字、头像。
#[derive(Default)]
pub(super) struct Faces {
    /// 号 → 问到的名字（平台回了没有的是空的）。
    names: Mutex<HashMap<Id, Option<String>>>,
    /// 正在问名字的号。
    asking: Mutex<HashSet<Id>>,
    /// 谁 → 头像。
    avatars: Mutex<HashMap<Whose, Avatar>>,
}

impl Faces {
    /// 号 `bot` 的名字：问到了的交回；没问过、连着的另起任务问（问到了叫一声 `changed`），这次是空的。
    pub(super) fn name(&self, site: &Arc<Site>, bot: Id) -> Option<String> {
        if let Some(name) = lock(&self.names).get(&bot) {
            return name.clone();
        }
        let link = site.running.first().bots.get(&bot)?;
        if !lock(&self.asking).insert(bot.clone()) {
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
                    tracing::debug!(target: TARGET, bot = %bot, error = ?error, "bot name unknown")
                }
            }
        });
        None
    }

    /// `whose` 的头像：记下的照它；没有的问驱动在哪（[`crate::chat::Link::avatar`]，施工 I-82：QQ 照地址模板，别的平台各有取法），经后台页那个平台连着
    /// 的号问（机器人自己的照它自己的连接，断开了的照别的号）；机器人的号要是连着的或者断开了的。取不到的是空的。
    pub(super) async fn avatar(&self, site: &Site, whose: Whose) -> Option<Avatar> {
        if let Some(avatar) = lock(&self.avatars).get(&whose) {
            return Some(Arc::clone(avatar));
        }
        let tuning = &site.running.tuning;
        let bots = &site.running.first().bots;
        let any = || bots.first().map(|(_, link)| link);
        let (face, link) = match &whose {
            Whose::Bot(bot) => {
                if bots.get(bot).is_none() && !bots.gone().iter().any(|(gone, _)| gone == bot) {
                    return None;
                }
                (Face::User(bot.clone()), bots.get(bot).or_else(any)?)
            }
            Whose::User(user) => (Face::User(user.clone()), any()?),
            Whose::Group(group) => (Face::Group(group.clone()), any()?),
        };
        let source = match link.avatar(&face).await {
            Ok(Some(source)) => source,
            Ok(None) => return None,
            Err(error) => {
                tracing::debug!(target: TARGET, whose = ?whose, error = ?error, "avatar not located");
                return None;
            }
        };
        let got = match bytes(&[source], tuning.fetch(), tuning.image_limit()).await {
            Ok(got) => got,
            Err(why) => {
                tracing::debug!(target: TARGET, whose = ?whose, why, "avatar not fetched");
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
        let mut avatars = lock(&self.avatars);
        if avatars.len() >= MOST {
            avatars.clear();
        }
        avatars.insert(whose, Arc::clone(&avatar));
        Some(avatar)
    }
}

/// 锁里不 `await`、不会崩；真崩了，表照样能用。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
