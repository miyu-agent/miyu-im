//! 读插件（施工 I-8 上）：经核心的 `content.list` 列出给 im 带了内容的包，`content.read` 读每个包目录根上的 `plugin.toml`
//! （一次最多 512 KiB，接着读到头；`data` 是 base64）。读不成的（没有这个文件、不是 UTF-8、写错了）记下为什么、记一行运行日志，
//! 别的照读。核心拒了 `content.list` 的当一个插件都没有。

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::{Plugins, declared};
use crate::TARGET;
use crate::core::caller::Caller;
use crate::core::{Gone, reason};

/// 插件的声明在它目录里的路径。
const DECLARED: &str = "plugin.toml";

/// 读一遍。
///
/// # Errors
///
/// 等的时候核心断开。
pub(crate) async fn load(caller: &Caller) -> Result<Plugins, Gone> {
    let listed = caller.call("content.list", json!({})).await?;
    if let Some(why) = reason(&listed) {
        tracing::warn!(target: TARGET, why, "plugins not listed");
        return Ok(Plugins::default());
    }
    let packages: Vec<String> = listed["result"]["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|one| one["package"].as_str().map(str::to_string))
        .collect();
    let mut known = BTreeMap::new();
    for package in packages {
        let declared = match read_all(caller, &package).await? {
            Ok(bytes) => String::from_utf8(bytes)
                .map_err(|_| format!("{DECLARED} is not UTF-8"))
                .and_then(|text| declared::read(&text)),
            Err(why) => Err(why),
        };
        match &declared {
            Ok(_) => tracing::info!(target: TARGET, package, "plugin known"),
            Err(why) => tracing::warn!(target: TARGET, package, why, "plugin not read"),
        }
        known.insert(package, declared);
    }
    let count = known.values().filter(|declared| declared.is_ok()).count();
    tracing::info!(target: TARGET, count, "plugins loaded");
    Ok(Plugins::new(known))
}

/// 读完 `package` 的 [`DECLARED`]：读不成的交回为什么（核心说的原因码，或者回的读不懂）。
async fn read_all(caller: &Caller, package: &str) -> Result<Result<Vec<u8>, String>, Gone> {
    let mut bytes = Vec::new();
    loop {
        let params = json!({"package": package, "path": DECLARED, "offset": bytes.len()});
        let reply = caller.call("content.read", params).await?;
        if let Some(why) = reason(&reply) {
            return Ok(Err(format!("{DECLARED}: {why}")));
        }
        let Some(chunk) = chunk(&reply["result"]) else {
            return Ok(Err(format!("{DECLARED}: unreadable answer")));
        };
        let eof = reply["result"]["eof"].as_bool().unwrap_or(true);
        if chunk.is_empty() && !eof {
            return Ok(Err(format!("{DECLARED}: empty chunk before the end")));
        }
        bytes.extend_from_slice(&chunk);
        if eof {
            return Ok(Ok(bytes));
        }
    }
}

/// 回应里的一块：`data` 是 base64。
fn chunk(result: &Value) -> Option<Vec<u8>> {
    STANDARD.decode(result["data"].as_str()?).ok()
}
