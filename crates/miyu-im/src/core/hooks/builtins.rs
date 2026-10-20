//! 驱动带的插件的挂接点（施工 I-10 上，`crate::plugins::Builtin`）：在进程里直接问，不经核心。只问总开关（驱动配置里的 `<开头>on`）
//! 开着、在这个场所开着（同软件包插件的场所开关，`Hooks::allowed`）的。

use crate::chat::To;
use crate::plugins::Seen;

use super::Hooks;

impl Hooks {
    /// 群 `group` 里的一条 `seen` 借了谁的名字（识人术）：照先后问开着的驱动带的插件，头一个说借了的为准；都没说的是空的。
    pub(crate) fn impostor(&self, group: i64, seen: &Seen<'_>) -> Option<String> {
        let config = self.driver.borrow().clone();
        self.builtins.iter().find_map(|builtin| {
            let switch = format!("{}on", builtin.settings());
            let on = config
                .get(&switch)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            if !on || !self.allowed(builtin.id(), Some(To::Group(group))) {
                return None;
            }
            builtin.impostor(&config, seen)
        })
    }
}
