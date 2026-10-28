//! 后台页调的方法（施工 O-28 上，`onebot.md` 第一条「后台页」第 2 条，`package-pages.md`「`package.methods`、`package.call`、
//! `method.call`」）：桥经 `package.methods` 登记，头在接入通讯平台 的后台页上经核心的 `package.call` 调，核心反向发 `method.call`。
//!
//! - 登记（[`register`]）：开好监听、桥手里的状态造好以后发，写出去就交回等回应、记运行日志的那一段（同 `provide`，「施工时定的」
//!   第 139、154 条）。被拒了只记一行 `ERROR`，桥照跑。
//! - 答（[`Methods::answer`]）：和 `tool.call` 一样由读的一头当场答（第 140 条）。读的一头在握手时就起来了，那时桥手里的状态
//!   还没有：造好以后（`crate::running::Running`，施工 O-28 下）经 [`Methods::ready`] 交进来，登记跟在后面（第 153 条）。
//! - `status` 照 `Running::status`（和状态文件照的同一份）；`connection.token` 交令牌的值，运行日志不记值。
//! - 给别的包调的两个（施工 O-42，[`ACTION`]、[`VENUE`]，清单 `[export]` 里只有它们）一起登记，读的一头不在这里答，照推送交给
//!   跟核心的那一头（`route/exports.rs`）：要用的会话表、连接在那里。

use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};

use super::caller::Caller;
use crate::TARGET;
use crate::core::{Gone, reason};
use crate::running::Running;

/// 后台页的「连接」页要的：NapCat 的状态、实际听的端口、地址的路径、令牌设没设、平台的名字。
const STATUS: &str = "status";

/// 令牌的值（「显示」「复制」）。
const TOKEN: &str = "connection.token";

/// 给别的包调的：替它调一个 OneBot 动作（施工 O-42）。
pub(crate) const ACTION: &str = "chat.action";

/// 给别的包调的：一个会话是哪个场所（施工 O-42）。
pub(crate) const VENUE: &str = "chat.venue";

/// 给别的包调的：以她的名义发一条（施工 O-56，同工具 `chat_send`）。
pub(crate) const SEND: &str = "chat.send";

/// 给别的包调的：一条消息里第几样图和它的来历（施工 I-39，表情包包偷图）。
pub(crate) const MEDIA: &str = "chat.media";

/// 给 im 的插件调的：往场所会话里记一句给 AI 的事实（施工 I-8 上，设计 01 第四节第 6 条）。
pub(crate) const NOTE: &str = "chat.note";

/// 后台页的场所卡片：读（施工 O-58 上）。
pub(crate) const CARDS_GET: &str = "cards.get";

/// 后台页的场所卡片：存（施工 O-58 上）。
pub(crate) const CARDS_SAVE: &str = "cards.save";

/// 会话浮窗的「概况」：一张卡今天的数、最近判断（施工 I-7 上补）。
pub(crate) const CARDS_OVERVIEW: &str = "cards.overview";

/// 后台「消息记录」页（施工 I-34 上，`route/browse.rs`；C-1 中下起记录改读 im 自己的库）：列场所会话、删一个会话。
pub(crate) const SESSIONS_LIST: &str = "sessions.list";
pub(crate) const SESSIONS_DELETE: &str = "sessions.delete";

/// 场所卡片两页的三个方法（施工 O-58 下，`rules::card_page`）：页面上存一张表单、加一项、删一项。
use crate::rules::card_page::{
    ADD as CARDS_ADD, REMOVE as CARDS_REMOVE, SAVE_ONE as CARDS_SAVE_ONE,
};

/// 方法 `method` 是不是交给跟核心的那一头答的：给别的包调的那几个（施工 O-42；O-56 多 `chat.send`），场所卡片的两个（施工 O-58
/// 上：卡片、场所规则、连接都在那一头），消息记录页的两个（施工 I-34 上：桥认得的会话在那一头）。
pub(super) fn exported(method: &str) -> bool {
    matches!(
        method,
        ACTION
            | VENUE
            | MEDIA
            | SEND
            | NOTE
            | CARDS_GET
            | CARDS_SAVE
            | CARDS_SAVE_ONE
            | CARDS_ADD
            | CARDS_REMOVE
            | CARDS_OVERVIEW
            | SESSIONS_LIST
            | SESSIONS_DELETE
    )
}

/// 答方法的那一头：桥手里的状态交进来以前什么都答不了。
pub(crate) struct Methods {
    /// 跑着的桥：连接表、实际听的端口、最新的配置、桥自己的数都在它身上。
    running: OnceLock<Arc<Running>>,
}

impl Methods {
    /// 还没拿到桥手里的状态的一份。
    pub(crate) fn new() -> Methods {
        Methods {
            running: OnceLock::new(),
        }
    }

    /// 桥手里的状态造好了：交进来，之后的 `method.call` 照它答。只交一次，再交的不算。
    pub(crate) fn ready(&self, running: Arc<Running>) {
        if self.running.set(running).is_err() {
            // 一个桥只起来一次，交第二次是没有的事；交了也照第一次的答。
        }
    }

    /// 答编号是 `id` 的 `method.call`，参数 `params` 是 `{"method", "params"}`：交回整条回应。不认识的方法、状态还没交进来的
    /// 回 -32601（核心交回 `method_failed`）。
    pub(super) fn answer(&self, id: Value, params: &Value) -> Value {
        let method = params["method"].as_str().unwrap_or_default();
        let result = match (self.running.get(), method) {
            (Some(running), STATUS) => {
                let mut status = running.status();
                status.insert("path".into(), json!(short_path(&running.tuning.paths)));
                Value::Object(status)
            }
            (Some(running), TOKEN) => {
                // 驱动配置里的令牌（施工 I-3；后台页随 I-5 重做，这一处跟着走）：照原样交，去掉前后空白，空的当没有。
                let token = running
                    .current
                    .driver_value(running.first().platform, "token");
                let token = token
                    .as_ref()
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|token| !token.is_empty());
                tracing::info!(target: TARGET, "page token read");
                json!({"token": token})
            }
            _ => {
                tracing::warn!(target: TARGET, method, "page method not understood");
                let error = json!({"code": -32601, "message": "unknown_method", "data": {"reason": "unknown_method"}});
                return json!({"jsonrpc": "2.0", "id": id, "error": error});
            }
        };
        json!({"jsonrpc": "2.0", "id": id, "result": result})
    }
}

/// NapCat 那边地址的路径：桥认的几个里最短的，一样短的取先写的（「施工时定的」第 152 条）。一个都没有的是空的。
pub(crate) fn short_path(paths: &[String]) -> &str {
    paths
        .iter()
        .min_by_key(|path| path.len())
        .map_or("", String::as_str)
}

/// `package.methods` 的参数：后台页的两个、给别的包调的两个（施工 O-42），不写时限（照核心的 30 秒：`chat.action` 最多等
/// `call_timeout_seconds`，比它短）。
fn registered() -> Value {
    json!({"methods": [
        {"name": STATUS},
        {"name": TOKEN},
        {"name": ACTION},
        {"name": VENUE},
        {"name": MEDIA},
        {"name": SEND},
        {"name": NOTE},
        {"name": CARDS_GET},
        {"name": CARDS_SAVE},
        {"name": CARDS_SAVE_ONE},
        {"name": CARDS_ADD},
        {"name": CARDS_REMOVE},
        {"name": CARDS_OVERVIEW},
        {"name": SESSIONS_LIST},
        {"name": SESSIONS_DELETE},
    ]})
}

/// 登记后台页调的方法：经 `caller` 发 `package.methods`，写出去就交回等回应、记运行日志的那一段（交给 `serve` 的任务组）。
/// 写不出去的（核心关了管道）交回空的：跟核心的那一头接着读到头，照「怎么走」第 11 条停下。
pub(crate) async fn register(caller: &Caller) -> Option<impl Future<Output = ()> + use<>> {
    let answer = caller.send("package.methods", registered()).await.ok()?;
    Some(async move {
        match answer.wait().await {
            Ok(reply) => match reason(&reply) {
                None => {
                    let count = reply["result"]["methods"].as_u64().unwrap_or_default();
                    tracing::info!(target: TARGET, count, "methods registered");
                }
                Some(reason) => {
                    let detail = &reply["error"]["data"];
                    tracing::error!(target: TARGET, reason, detail = %detail, "methods not registered");
                }
            },
            Err(Gone) => {
                // 核心关了管道：跟核心的那一头接着读到头，桥停下。
            }
        }
    })
}

#[cfg(test)]
mod tests;
