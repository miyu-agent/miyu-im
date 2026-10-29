//! 驱动带的插件的挂接点（施工 I-10 上，`crate::plugins::Builtin`）：在进程里直接问，不经核心。只问总开关（驱动配置里的 `<开头>on`）
//! 开着、在这个场所开着（同软件包插件的场所开关，`Hooks::allowed`）的。施工 Q-1 起驱动的工具也照这个问（[`Hooks::active`]）。
//!
//! 挂几个驱动的（施工 I-70 中上）：插件记着是哪个驱动带的，总开关、设置照那个驱动的配置取；只在那个驱动的平台上的场所里算开着
//! （它跟着驱动走，别的平台上没有它要的连接）。

use serde_json::{Map, Value};

use crate::chat::{Id, Platform, To};
use crate::plugins::{Builtin, Seen};

use super::{Hooks, Mounted};

impl Hooks {
    /// 平台 `platform` 上群 `group` 里的一条 `seen` 借了谁的名字（识人术）：照先后问那个平台的驱动带的、开着的插件，头一个说借了的
    /// 为准；都没说的是空的。
    pub(crate) fn impostor(
        &self,
        platform: Platform,
        group: Id,
        seen: &Seen<'_>,
    ) -> Option<String> {
        let mounted = self.mounted(platform)?;
        let config = mounted.config.borrow().clone();
        let to = Some(To::Group(group));
        mounted
            .builtins
            .iter()
            .filter(|builtin| self.on(builtin.as_ref(), (mounted, platform, to.clone()), &config))
            .find_map(|builtin| builtin.impostor(&config, seen))
    }

    /// 平台 `platform` 上场所 `to` 开着的那个平台的驱动带的插件的编号，照先后（施工 Q-1：交给驱动的工具，`toolset::Call::plugins`）。
    pub(crate) fn active(&self, platform: Platform, to: To) -> Vec<&'static str> {
        let Some(mounted) = self.mounted(platform) else {
            return Vec::new();
        };
        let config = mounted.config.borrow().clone();
        mounted
            .builtins
            .iter()
            .filter(|builtin| {
                self.on(
                    builtin.as_ref(),
                    (mounted, platform, Some(to.clone())),
                    &config,
                )
            })
            .map(|builtin| builtin.id())
            .collect()
    }

    /// 平台 `platform` 上场所 `to` 开着的插件的编号，软件包的和驱动带的都算，照编号排（施工 S-1 上c：经 `venue.plugins` 告诉核心，
    /// 核心 O-60 照它给带 `plugin` 的工具）。驱动带的另看带它的驱动的总开关。
    pub(crate) fn plugins_on(&self, platform: Platform, to: To) -> Vec<String> {
        let known: Vec<String> = self
            .plugins
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .all()
            .map(|(package, _)| package.to_string())
            .collect();
        known
            .into_iter()
            .filter(|package| match self.owner(package) {
                Some((mounted, builtin)) => {
                    let config = mounted.config.borrow().clone();
                    self.on(builtin, (mounted, platform, Some(to.clone())), &config)
                }
                None => self.allowed(package, platform, Some(to.clone())),
            })
            .collect()
    }

    /// 平台 `platform` 的驱动这一刻的配置（没写的照清单的默认补上了）；没挂的平台是空的。
    pub(crate) fn driver_config(&self, platform: Platform) -> Map<String, Value> {
        self.mounted(platform)
            .map(|mounted| mounted.config.borrow().clone())
            .unwrap_or_default()
    }

    /// 编号是 `package` 的驱动带的插件和带它的驱动；不是驱动带的是空的。
    fn owner(&self, package: &str) -> Option<(&Mounted, &dyn Builtin)> {
        self.drivers.iter().find_map(|mounted| {
            mounted
                .builtins
                .iter()
                .find(|builtin| builtin.id() == package)
                .map(|builtin| (mounted, builtin.as_ref()))
        })
    }

    /// 驱动 `owner` 带的插件 `builtin` 在平台 `platform` 上的场所 `to` 开着没有：是那个驱动的平台、驱动配置 `config` 里的总开关
    /// （没写的当开着）、场所开关。
    fn on(
        &self,
        builtin: &dyn Builtin,
        (owner, platform, to): (&Mounted, Platform, Option<To>),
        config: &Map<String, Value>,
    ) -> bool {
        let switch = format!("{}on", builtin.settings());
        let on = config.get(&switch).and_then(Value::as_bool).unwrap_or(true);
        owner.platform == platform && on && self.allowed(builtin.id(), platform, to)
    }
}
