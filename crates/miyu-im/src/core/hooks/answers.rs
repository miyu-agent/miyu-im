//! 挂接点的回答怎么合（施工 O-43 中、下、末；施工 I-70 中上从 `hooks.rs` 挪来：那个文件到了行数上限）：进站的几个意见取最重的，
//! 出站一个包的回答读成换、不发、照原样，打分的几个加分夹住再加。纯逻辑。

use serde_json::Value;

use super::{Nudge, Steer, Steered};

/// 一个包、几个包的加分最多加减多少：门槛是 0 到 1 左右的数，夹住免得一个包把判断整个改掉（施工 O-43 末）。
const NUDGE_MAX: f64 = 1.0;

/// 几个包的加分（照编号排）各夹到 -1 到 1、加起来再夹一次；给了不是 0 的记下是谁。
pub(super) fn summed(answers: Vec<(String, f64)>) -> Nudge {
    let bonus = answers
        .iter()
        .map(|(_, bonus)| bonus.clamp(-NUDGE_MAX, NUDGE_MAX))
        .sum::<f64>();
    Nudge {
        bonus: bonus.clamp(-NUDGE_MAX, NUDGE_MAX),
        by: answers
            .into_iter()
            .filter(|(_, bonus)| *bonus != 0.0)
            .map(|(package, _)| package)
            .collect(),
    }
}

/// 出站挂接点一个包的回答。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Edit {
    /// 照原样：`{}`、核心断开的。
    Keep,
    /// 换成这一段：`{"text": <不空的字>}`。
    Change(String),
    /// 不发：`{"drop": true}`。
    Drop,
}

impl Edit {
    /// 读一个包的回答 `result`：`drop` 是 `true` 的不发；`text` 是不空的字的换；两格都没有的照原样；别的写法是答错了
    /// （`bad_answer`）。
    pub(super) fn read(result: &Value) -> Result<Edit, String> {
        let bad = || "bad_answer".to_string();
        match (result.get("drop"), result.get("text")) {
            (Some(Value::Bool(true)), None) => Ok(Edit::Drop),
            (None, Some(Value::String(text))) if !text.trim().is_empty() => {
                Ok(Edit::Change(text.clone()))
            }
            (None, None) if result.is_object() || result.is_null() => Ok(Edit::Keep),
            _ => Err(bad()),
        }
    }
}

/// 几个包的意见（照编号排）取最重的：一样重的照编号取头一个；都照常的没有包。
pub(super) fn heaviest(answers: Vec<(String, Steer)>) -> Steered {
    let mut steered = Steered::pass();
    for (package, steer) in answers {
        if steer > steered.steer {
            steered = Steered {
                steer,
                by: Some(package),
            };
        }
    }
    steered
}
