//! 驱动自己带的 im 插件（施工 I-10 上，设计 01 第一节第 22 条、第四节第 6 条；2026-10-19 项目主人定：识人术是 OneBot 的插件，代码在驱动里）：
//! 照样是一个 im 插件（声明、场所开关、插件页），只是不是软件包：跟驱动在一个进程里，出厂就在、卸不掉，挂接点在进程里直接问，不经核心的
//! `package.call`。
//!
//! - 设置是驱动的配置项（清单 `[settings.<平台>.<开头>…]`，藏起来，只在插件页「设置」里改），名字都以 [`Builtin::settings`] 开头；
//!   总开关是 `<开头>on`（插件页的总开关改它）。
//! - 现在只有一处挂接点：群里一个人借了别人的名字（[`Builtin::impostor`]，识人术）。

use serde_json::{Map, Value};

use super::Declared;

/// 驱动带的一个插件。
pub trait Builtin: Send + Sync {
    /// 编号（同软件包的写法，`miyu-im-identity`）：场所卡片的插件开关照它记。
    fn id(&self) -> &'static str;

    /// 声明（同 `plugin.toml` 读出来的）。
    fn declared(&self) -> Declared;

    /// 设置项的名字的开头（`identity_`）：驱动的配置里以它开头的几项是这个插件的，总开关是 `<开头>on`。
    fn settings(&self) -> &'static str;

    /// 群里一个人借了谁的名字（识人术）：`config` 是驱动这一刻的配置（没写的照清单的默认补上了），`seen` 是这一条。交回借的那个
    /// 名字；不管、没借的是空的。
    fn impostor(&self, config: &Map<String, Value>, seen: &Seen<'_>) -> Option<String> {
        let _ = (config, seen);
        None
    }
}

/// 群里的一条（[`Builtin::impostor`]）。
#[derive(Debug, Clone, Copy)]
pub struct Seen<'a> {
    /// 发的人这一条时叫什么（群名片或昵称）。
    pub name: &'a str,
    /// 发的人在平台上的号。
    pub number: &'a str,
    /// 终端管理员在这个群用过的名字。
    pub admin_names: &'a [String],
    /// 她自己在这个群的群名片；没问到的是空的。
    pub own_card: Option<&'a str>,
}
