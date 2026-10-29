//! 页面调驱动带的插件（施工 Q-6，`plugins::Builtin::call`）：界面块的内容、按钮和表单的动作不经核心的 `package.call`，在进程里直接
//! 问驱动带的那一个，交给它这一刻的配置、状态目录、连着的号。

use serde_json::Value;

use super::Site;
use crate::chat::Platform;
use crate::plugins::{Builtin, Context};
use crate::status_file;

impl Site {
    /// 编号是 `package` 的驱动带的插件和带它的驱动的平台（施工 I-85 上：几个驱动里找）；不是的是空的。
    fn builtin(&self, package: &str) -> Option<(std::sync::Arc<dyn Builtin>, Platform)> {
        self.drivers.iter().find_map(|driver| {
            driver
                .builtins()
                .into_iter()
                .find(|builtin| builtin.id() == package)
                .map(|builtin| (builtin, driver.platform()))
        })
    }

    /// 调驱动带的插件 `package` 的方法 `method`，参数 `params`：错的照 `package.call` 的写法交回（原因，空的说明）。不是驱动带的
    /// 插件的回 `unknown_package`。
    pub(super) async fn call_builtin(
        &self,
        package: &str,
        method: &str,
        params: &Value,
    ) -> Result<Value, (String, String)> {
        let Some((builtin, platform)) = self.builtin(package) else {
            return Err(("unknown_package".to_string(), String::new()));
        };
        let Some(station) = self.running.station(Some(platform.name())) else {
            return Err(("unknown_package".to_string(), String::new()));
        };
        let context = Context {
            platform,
            config: self.running.current.driver(platform).borrow().clone(),
            store: status_file::store(&self.root, platform),
            bots: std::sync::Arc::clone(&station.bots),
        };
        builtin
            .call(&context, method, params)
            .await
            .map_err(|reason| (reason, String::new()))
    }
}
