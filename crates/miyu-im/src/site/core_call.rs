//! 转发口 `core.call {method, params?}`（设计 09 第二节第 1 条，施工 I-145）：人格、预设、模型三页在这份登录的人自己的连接上原样调
//! 核心，协议的形状照核心的，im 只管放不放行。
//!
//! - 放行的只有 [`ALLOWED`] 里的方法，别的拒 `not_allowed`；`method` 没写、不是字拒 `bad_params`。`params` 不写的照 `{}`。
//! - 核心接了的交回 `{"result": <核心回的>}`；核心拒了的（连不上、断开这几种也算）交回 `{"refused": {"reason", "message", "data"}}`，
//!   不当 im 自己的拒绝：页面照核心的原因写给人看的字（`persona_conflict` 这些），通道的拒绝只留给 im 自己的。`data` 是核心拒绝里
//!   带的整份（冲突时现在的值、写错的地方、续传从哪接着，施工 I-148 换钉以后才有），核心以外的几种是 null。
//! - 命令编号每次不同（`page-<序号>`）：这几样核心都不照编号去重，不同只是省得以后有人加了去重的方法进白名单撞上。

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use super::Person;

/// 页面能经转发口调的核心方法。
pub(super) const ALLOWED: [&str; 26] = [
    "persona.list",
    "persona.get",
    "persona.read",
    "persona.set",
    "persona.delete",
    "persona.avatar",
    "persona.background",
    "preset.list",
    "preset.get",
    "preset.set",
    "preset.delete",
    "provider.catalog",
    "provider.detect",
    "provider.test",
    "model.list",
    "config.schema",
    "config.get",
    "config.set",
    "secret.set",
    "check",
    "blob.open",
    "blob.write",
    "blob.close",
    "package.list",
    "memory.list",
    // 看人格的记忆（施工 I-207）：认识的人的卡。只有读的，改、删不放。
    "memory.cards",
];

/// 下一个命令编号的序号。
static NEXT: AtomicU64 = AtomicU64::new(0);

/// `core.call`（模块的说明）。
pub(super) async fn call(person: &Person, params: &Value) -> Result<Value, (String, String)> {
    let method = allowed(params)?;
    let inner = params.get("params").cloned().unwrap_or_else(|| json!({}));
    let id = format!("page-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    Ok(match person.lock().await.call(&id, method, inner).await {
        Ok(result) => json!({"result": result}),
        Err(why) => {
            json!({"refused": {"reason": why.reason, "message": why.message, "data": why.data}})
        }
    })
}

/// 照 `params` 认要调的方法：没写、不在白名单里的交回拒绝（纯逻辑）。
fn allowed(params: &Value) -> Result<&str, (String, String)> {
    let method = params["method"]
        .as_str()
        .ok_or_else(|| ("bad_params".to_string(), String::new()))?;
    if ALLOWED.contains(&method) {
        Ok(method)
    } else {
        Err(("not_allowed".to_string(), String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::allowed;
    use serde_json::json;

    #[test]
    fn only_listed_methods_pass() {
        assert_eq!(
            allowed(&json!({"method": "persona.set"})),
            Ok("persona.set")
        );
        assert_eq!(
            allowed(&json!({"method": "session.send"})).unwrap_err().0,
            "not_allowed"
        );
        assert_eq!(
            allowed(&json!({"method": "secret.list"})).unwrap_err().0,
            "not_allowed"
        );
        assert_eq!(allowed(&json!({})).unwrap_err().0, "bad_params");
        assert_eq!(allowed(&json!({"method": 3})).unwrap_err().0, "bad_params");
    }
}
