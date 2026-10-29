//! 挂着的几个平台和各自连着的号（施工 I-70 上，设计 07 第 4.3 节，`onebot.md` 第一条「在哪」）：一个 im 要同时挂几个驱动，
//! 宿主拼场所、认命令编号、找号都先认是哪个平台。有先后：第一个是第一个驱动的平台（[`Platforms::first`]，I-70 中下认旧数据用）。
//!
//! 找号不跨平台：指定的号不在时只退回同一个平台连着的号（[`Platforms::sender`]）。给别的包调的方法、工具只给一个号的，平台照
//! [`Platforms::ours`] 认（`extending.md`「调接入通讯平台的方法」）。

use std::sync::Arc;

use miyu_chat::Venue;
use serde_json::Value;

use super::{Bots, Id, Link, Platform};

/// 挂着的几个平台，照先后；每个带它连着的号。
#[derive(Debug, Clone, Default)]
pub struct Platforms(Vec<(Platform, Arc<Bots>)>);

/// [`Platforms::ours`] 不收的为什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unclaimed {
    /// 写了别的平台（不是挂着的）。
    Other,
    /// 没写 `platform`、挂着的又不止一个平台：调的一方答 `bad_params`，说缺 `platform`。
    Missing,
}

impl Unclaimed {
    /// 运行日志里写的为什么：写了别的平台的是 `another_platform`，挂了几个平台又没写的是 `platform_missing`。
    pub fn name(self) -> &'static str {
        match self {
            Unclaimed::Other => "another_platform",
            Unclaimed::Missing => "platform_missing",
        }
    }
}

impl Platforms {
    /// 照先后的几个平台和各自连着的号。
    pub fn new(platforms: Vec<(Platform, Arc<Bots>)>) -> Platforms {
        Platforms(platforms)
    }

    /// 挂着的平台，照先后。
    pub fn iter(&self) -> impl Iterator<Item = Platform> + '_ {
        self.0.iter().map(|(platform, _)| *platform)
    }

    /// 名字是 `name` 的平台；没挂的是空的。
    pub fn named(&self, name: &str) -> Option<Platform> {
        self.iter().find(|platform| platform.name() == name)
    }

    /// 场所 `venue` 是哪个平台的：照编号的第一段；没挂的是空的。
    pub fn of_venue(&self, venue: &Venue) -> Option<Platform> {
        self.named(venue.platform())
    }

    /// 第一个驱动的平台：一个都没挂的是空的。
    pub fn first(&self) -> Option<Platform> {
        self.iter().next()
    }

    /// 平台 `platform` 连着的号。没挂的平台交一份空的（一个号都没连着）：找号的照「没连着」办，不会拿到别的平台的号。
    pub fn bots(&self, platform: Platform) -> Arc<Bots> {
        self.0
            .iter()
            .find(|(one, _)| *one == platform)
            .map_or_else(Arc::default, |(_, bots)| Arc::clone(bots))
    }

    /// 平台 `platform` 上号 `bot` 现在的连接。
    pub fn link(&self, platform: Platform, bot: &Id) -> Option<Arc<dyn Link>> {
        self.bots(platform).get(bot)
    }

    /// 在平台 `platform` 上经哪个号发：`bot` 写了、连着的照它；不然退回这个平台连着的号里最小的那一个，不跨平台（施工 I-70 上）。
    /// 这个平台一个号都没连着的是空的。
    pub fn sender(&self, platform: Platform, bot: Option<Id>) -> Option<Id> {
        let bots = self.bots(platform);
        bot.filter(|bot| bots.get(bot).is_some())
            .or_else(|| bots.first().map(|(bot, _)| bot))
    }

    /// 给别的包调的方法、工具的参数 `args` 是冲哪个平台来的（施工 I-1、I-70 上「定了的」第 1 条）：写了 `platform` 的照写的，不是
    /// 挂着的不收；没写、写 `null` 的，只挂了一个平台时算它，挂了几个时不收。
    ///
    /// # Errors
    ///
    /// 写了别的平台的交 [`Unclaimed::Other`]；没写、挂着的又不止一个平台的交 [`Unclaimed::Missing`]。
    pub fn ours(&self, args: &Value) -> Result<Platform, Unclaimed> {
        match args.get("platform") {
            None | Some(Value::Null) => match self.0.as_slice() {
                [(only, _)] => Ok(*only),
                _ => Err(Unclaimed::Missing),
            },
            Some(name) => name
                .as_str()
                .and_then(|name| self.named(name))
                .ok_or(Unclaimed::Other),
        }
    }
}

#[cfg(test)]
mod tests;
