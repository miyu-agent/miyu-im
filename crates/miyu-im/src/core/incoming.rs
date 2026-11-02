//! 读进来的一条（施工 I-98）：事件推送（`event`，没有编号）只看一眼方法、会话，原文留着，用的时候才解成 `Value`；别的（回应、
//! 别的推送、核心发来的请求）照旧当场解。
//!
//! 为什么（4c 11-02 用 heaptrack 量的）：起来订阅群会话从头补日志时，补来的推送在 [`super::Core::call_as`] 等回应的那一会儿全
//! 堆进留着的里面，整份日志同时以 `Value` 活着，一条约是原文的 7 倍；最大的群 3.7 万条峰值 187 MB，glibc 一次冲高不还页。原文
//! 留着只占原文那么大，取出来一条解一条（[`Taken`]）。

use std::borrow::Cow;
use std::collections::VecDeque;

use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::Value;

/// 读进来的一条。
#[derive(Debug)]
pub(crate) enum Incoming {
    /// 事件推送：哪个会话（没写的是空的）、原文。
    Event {
        /// 会话编号。
        session: Option<Box<str>>,
        /// 那一行原文。
        raw: Box<str>,
    },
    /// 别的：解好了的。
    Parsed(Value),
}

impl Incoming {
    /// 读一行：事件推送只看一眼（[`Peek`]），别的解成 `Value`。
    ///
    /// # Errors
    ///
    /// 不是 JSON。
    pub(crate) fn read(line: &str) -> Result<Incoming, serde_json::Error> {
        let peek: Peek<'_> = serde_json::from_str(line)?;
        if peek.method.as_deref() == Some("event") && peek.id.is_none() {
            let session = peek
                .params
                .and_then(|params| params.session)
                .map(|session| session.into_owned().into_boxed_str());
            return Ok(Incoming::Event {
                session,
                raw: line.trim_end().into(),
            });
        }
        serde_json::from_str(line).map(Incoming::Parsed)
    }

    /// 解成 `Value`：事件推送这时才解（读进来时看过一眼是 JSON，照说解得出；万一解不出的是 `null`，用的一方照读不出的推送丢）。
    pub(crate) fn value(self) -> Value {
        match self {
            Incoming::Event { raw, .. } => serde_json::from_str(&raw).unwrap_or(Value::Null),
            Incoming::Parsed(value) => value,
        }
    }

    /// 是会话编号合 `wanted` 的事件推送。
    pub(crate) fn event_of(&self, wanted: impl Fn(&str) -> bool) -> bool {
        matches!(self, Incoming::Event { session: Some(session), .. } if wanted(session))
    }
}

/// 只看一眼的那几格：别的格照 JSON 走过去，不分配。
#[derive(Deserialize)]
struct Peek<'a> {
    #[serde(borrow, default)]
    method: Option<Cow<'a, str>>,
    #[serde(default)]
    id: Option<IgnoredAny>,
    #[serde(borrow, default)]
    params: Option<PeekParams<'a>>,
}

/// 推送参数里的会话编号。
#[derive(Deserialize)]
struct PeekParams<'a> {
    #[serde(borrow, default)]
    session: Option<Cow<'a, str>>,
}

/// 取出来的几条：照先后一条一条解，同时活着的 `Value` 只有手上那一条。
pub(crate) struct Taken(pub(super) VecDeque<Incoming>);

impl Iterator for Taken {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        self.0.pop_front().map(Incoming::value)
    }
}

#[cfg(test)]
mod tests;
