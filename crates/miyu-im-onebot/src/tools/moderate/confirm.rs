//! 群友叫她踢人要她再确认一次（施工 I-17，照旧版的 `require_ai_confirmation`）：叫她的人不是群主、管理员的，头一次调用不做，答一句
//! 「要确认：同样的调用带上 `confirm` 再发一次」；同一轮里同一个人叫的、参数（去掉 `confirm`）一样、带 `confirm: true` 的再一次才做。
//! 一条线（会话编号）记一次在等的，新的顶掉旧的；纯逻辑，进程里一份（[`check`]）。
//!
//! 「同一轮」照 im 交来的 `Call::used`（这条线这一轮这件工具以前调过几次）：一轮的头一次调用不会是确认（上一轮留下的照新的记下）。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde_json::Value;

/// 等确认最多多久：过了的当没有。
const KEPT: Duration = Duration::from_secs(600);

/// 一次在等确认的。
#[derive(Debug)]
struct Waiting {
    /// 谁叫的。
    caller: Option<i64>,
    /// 参数，去掉 `confirm`。
    args: Value,
    /// 什么时候记下的。
    at: Instant,
}

/// 每条线在等确认的那一次。
#[derive(Debug, Default)]
pub(crate) struct Confirms(HashMap<String, Waiting>);

impl Confirms {
    /// 线 `line` 上这一轮这件工具以前调过 `used` 次，这一次是 `caller` 叫的、参数 `args`，此刻 `now`：交回做不做。带
    /// `confirm: true`、这一轮前面要过确认、谁叫的和参数都一样的做（用掉那一次）；别的记下这一次，不做。
    pub(crate) fn check(
        &mut self,
        (line, used): (&str, usize),
        caller: Option<i64>,
        args: &Value,
        now: Instant,
    ) -> bool {
        self.0
            .retain(|_, waiting| now.duration_since(waiting.at) < KEPT);
        let bare = without_confirm(args);
        let confirmed = args["confirm"].as_bool() == Some(true);
        let matches = self
            .0
            .get(line)
            .is_some_and(|waiting| waiting.caller == caller && waiting.args == bare);
        if used > 0 && confirmed && matches {
            self.0.remove(line);
            return true;
        }
        self.0.insert(
            line.to_string(),
            Waiting {
                caller,
                args: bare,
                at: now,
            },
        );
        false
    }
}

/// 参数去掉 `confirm`：比的是要做的事一样不一样。
fn without_confirm(args: &Value) -> Value {
    let mut bare = args.clone();
    if let Some(object) = bare.as_object_mut() {
        object.remove("confirm");
    }
    bare
}

/// 这个进程里在等确认的。
static CONFIRMS: LazyLock<Mutex<Confirms>> = LazyLock::new(Mutex::default);

/// 照进程里那一份查（[`Confirms::check`]），此刻照本机的钟。
pub(crate) fn check(line: &str, used: usize, caller: Option<i64>, args: &Value) -> bool {
    CONFIRMS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .check((line, used), caller, args, Instant::now())
}

#[cfg(test)]
mod tests;
