//! im 的插件（施工 I-8 上，设计 01 第四节）：别的包在清单里写 `[content.im] dir = "…"` 给 im 带一个目录，目录的根上有
//! `plugin.toml`（[`declared`]）的，就是 im 的插件。im 起来时、核心推 `content.changed` 时经 `content.list`、`content.read` 读
//! （`load`）。插件在一个平台上能不能用照它声明的 `platforms`、`needs` 和驱动的能力表算（[`Plugins::usable`]）；在一个场所
//! 开没开照场所卡片（`crate::rules` 的卡片，[`Toggles`]）。挂接点只问能用、开着的（`crate::core::hooks`）。

pub mod declared;
mod load;
mod toggles;

use std::collections::BTreeMap;

pub use declared::Declared;
pub(crate) use load::load;
pub use toggles::Toggles;

use crate::chat::{Cap, Platform};

/// 认得的插件：包的编号到它的声明，读不成的是为什么。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plugins {
    known: BTreeMap<String, Result<Declared, String>>,
}

/// 插件在一个平台上用不了的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unusable {
    /// 不是 im 的插件（没带 `plugin.toml`）。
    NotAPlugin,
    /// `plugin.toml` 读不成：为什么。
    Broken(String),
    /// 声明了只在别的平台上用。
    OtherPlatforms,
    /// 平台缺这几样能力。
    Missing(Vec<Cap>),
}

impl Plugins {
    /// 照读到的 `known`（包的编号到声明或为什么读不成）。
    pub fn new(known: BTreeMap<String, Result<Declared, String>>) -> Plugins {
        Plugins { known }
    }

    /// 包 `package` 的声明；不是插件、读不成的是空的。
    pub fn declared(&self, package: &str) -> Option<&Declared> {
        self.known.get(package)?.as_ref().ok()
    }

    /// 每一个认得的包和它的声明（或为什么读不成），照编号排。
    pub fn all(&self) -> impl Iterator<Item = (&str, &Result<Declared, String>)> {
        self.known
            .iter()
            .map(|(package, declared)| (package.as_str(), declared))
    }

    /// 包 `package` 在平台 `platform`（能力表 `caps`）上能不能用：能用的 `Ok`，不能用的交回为什么。
    ///
    /// # Errors
    ///
    /// 不是插件、声明读不成、只在别的平台上用、平台缺能力（[`Unusable`]）。
    pub fn usable(&self, package: &str, platform: Platform, caps: &[Cap]) -> Result<(), Unusable> {
        let declared = match self.known.get(package) {
            None => return Err(Unusable::NotAPlugin),
            Some(Err(why)) => return Err(Unusable::Broken(why.clone())),
            Some(Ok(declared)) => declared,
        };
        if !declared.platforms.is_empty()
            && !declared
                .platforms
                .iter()
                .any(|name| name == platform.name())
        {
            return Err(Unusable::OtherPlatforms);
        }
        let missing: Vec<Cap> = declared
            .needs
            .iter()
            .copied()
            .filter(|need| !caps.contains(need))
            .collect();
        match missing.is_empty() {
            true => Ok(()),
            false => Err(Unusable::Missing(missing)),
        }
    }
}

#[cfg(test)]
mod tests;
