//! 驱动带的插件的挂接点（施工 I-10 上，`crate::plugins::Builtin`）：在进程里直接问，不经核心。只问总开关（驱动配置里的 `<开头>on`）
//! 开着、在这个场所开着（同软件包插件的场所开关，`Hooks::allowed`）的。施工 Q-1 起驱动的工具也照这个问（[`Hooks::active`]）。

use serde_json::{Map, Value};

use crate::chat::To;
use crate::plugins::{Builtin, Seen};

use super::Hooks;

impl Hooks {
    /// 群 `group` 里的一条 `seen` 借了谁的名字（识人术）：照先后问开着的驱动带的插件，头一个说借了的为准；都没说的是空的。
    pub(crate) fn impostor(&self, group: i64, seen: &Seen<'_>) -> Option<String> {
        let config = self.driver_config();
        let to = Some(To::Group(group));
        self.builtins
            .iter()
            .filter(|builtin| self.on(builtin.as_ref(), to, &config))
            .find_map(|builtin| builtin.impostor(&config, seen))
    }

    /// 在场所 `to` 开着的驱动带的插件的编号，照先后（施工 Q-1：交给驱动的工具，`toolset::Call::plugins`）。
    pub(crate) fn active(&self, to: To) -> Vec<&'static str> {
        let config = self.driver_config();
        self.builtins
            .iter()
            .filter(|builtin| self.on(builtin.as_ref(), Some(to), &config))
            .map(|builtin| builtin.id())
            .collect()
    }

    /// 驱动这一刻的配置（没写的照清单的默认补上了）。
    pub(crate) fn driver_config(&self) -> Map<String, Value> {
        self.driver.borrow().clone()
    }

    /// 驱动带的插件 `builtin` 在场所 `to` 开着没有：驱动配置 `config` 里的总开关（没写的当开着）、场所开关。
    fn on(&self, builtin: &dyn Builtin, to: Option<To>, config: &Map<String, Value>) -> bool {
        let switch = format!("{}on", builtin.settings());
        let on = config.get(&switch).and_then(Value::as_bool).unwrap_or(true);
        on && self.allowed(builtin.id(), to)
    }
}
