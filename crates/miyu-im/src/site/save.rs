//! 存连接设置（施工 I-6 中，设计 01 第八节第 2 条：新建连接的「保存并连接」、连接卡齿轮里的「保存」）：页面交驱动的配置项
//! `{"values": {名字: 字}}`（OneBot 是 `listen`、`token`），im 在这份登录的人的连接上替人存（人的权限，设计第九节第 2 条）：
//!
//! - 密钥（`Driver::secrets`）：非空的先 `secret.set` 存成 `im-<平台>-<名字>`，配置里写引用 `{"secret": …}`；空的恢复默认（没有）。
//! - 别的：照人敲的字交（`input`），核心照清单的类型读、查。
//!
//! 一起写进系统配置（`config.set`，有一项不对整条不收）；核心收下推 `extension.config`，驱动当场换（端口先开新的再换上）。
//! 不是驱动的配置项、不是字的：`bad_params`；核心拒了的照它的原因和原话交回。

use serde_json::{Value, json};

use super::{Person, Site};
use crate::settings::key;

/// 存下的样子：成了的 `{"saved": true}`；不成的（原因、原话）。
pub(super) async fn save(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let bad = || ("bad_params".to_string(), String::new());
    let values = params["values"].as_object().ok_or_else(bad)?;
    let (station, driver) = site.pick(params)?;
    let platform = station.platform.name();
    let mut changes = Vec::new();
    let mut secrets = Vec::new();
    for (name, value) in values {
        if !driver.settings().contains(&name.as_str()) {
            return Err(bad());
        }
        let text = value.as_str().ok_or_else(bad)?;
        let key = key(platform, name);
        if !driver.secrets().contains(&name.as_str()) {
            changes.push(json!({"key": key, "input": text}));
        } else if text.trim().is_empty() {
            changes.push(json!({"key": key, "unset": true}));
        } else {
            let secret = format!("{}-{platform}-{name}", crate::PACKAGE);
            changes.push(json!({"key": key, "value": {"secret": secret}}));
            secrets.push((secret, text.trim().to_string()));
        }
    }
    if changes.is_empty() {
        return Err(bad());
    }
    let mut person = person.lock().await;
    for (name, value) in secrets {
        person
            .call(
                "secret",
                "secret.set",
                json!({"name": name, "value": value}),
            )
            .await
            .map_err(|why| (why.reason, why.message))?;
    }
    person
        .call(
            "config",
            "config.set",
            json!({"layer": "system", "changes": changes}),
        )
        .await
        .map_err(|why| (why.reason, why.message))?;
    tracing::info!(target: crate::TARGET, keys = changes.len(), "connection settings saved");
    Ok(json!({"saved": true}))
}
