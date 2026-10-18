//! 宿主和驱动之间（施工 I-2 下，设计 01 第三节）：驱动是一个平台的接法（OneBot 的反向 WebSocket……），宿主把监听到的连接交给
//! 它（[`Driver::accept`]），它从 [`Gate`] 拿宿主给的几样：连着的号登记在哪、认出来的事交给谁、令牌、桥自己的数、说给人听的、
//! 状态变了叫谁、平台事件交给挂接点。宿主不依赖驱动，驱动只经这里碰宿主。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use miyu_client::settings::Secret;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::{Notify, mpsc};

use crate::chat::{Bots, Event, Platform};
use crate::core::hooks::Hooks;
use crate::current::Current;
use crate::serve::Notice;
use crate::tuning::Tuning;

/// 一个平台的接法。
pub trait Driver: Send + Sync {
    /// 平台：场所、平台上的人、命令编号照它的名字拼。
    fn platform(&self) -> Platform;

    /// 接一个连进来的 TCP 连接，跑到它断开。
    fn accept(
        &self,
        stream: TcpStream,
        gate: Arc<Gate>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

/// 宿主给驱动的，各个连接共用。
pub struct Gate {
    /// 令牌在这里（和后台页的方法共用，核心推来新的就换上，施工 O-20）。
    current: Arc<Current>,
    /// 认哪几个路径、调用等多久、写队列多长（`bridge.json`）。
    tuning: Tuning,
    /// 连着的号。
    bots: Arc<Bots>,
    /// 认出来的事交给跟核心的那一头（`core/route.rs`）。
    inbound: mpsc::Sender<Event>,
    /// 连上、断开各说一行（「样子」）。
    tell: Arc<dyn Fn(Notice) + Send + Sync>,
    /// 连上、断开、认出号、问到是哪个实现时叫一声：状态文件照这一刻再写（施工 O-18）。
    changed: Arc<Notify>,
    /// 挂接点（施工 O-43 上）：平台的通知、请求原样交给挂在 `chat.event` 的包。
    hooks: Arc<Hooks>,
}

impl Gate {
    /// 拼一份：几样都是 `serve` 起来时备好的。
    pub(crate) fn new(
        (current, tuning): (Arc<Current>, Tuning),
        bots: Arc<Bots>,
        inbound: mpsc::Sender<Event>,
        tell: Arc<dyn Fn(Notice) + Send + Sync>,
        changed: Arc<Notify>,
        hooks: Arc<Hooks>,
    ) -> Gate {
        Gate {
            current,
            tuning,
            bots,
            inbound,
            tell,
            changed,
            hooks,
        }
    }

    /// 桥自己的数。
    pub fn tuning(&self) -> &Tuning {
        &self.tuning
    }

    /// 连着的号。
    pub fn bots(&self) -> &Bots {
        &self.bots
    }

    /// 连进来的要出示的令牌：桥手里最新的，没有的是空的。
    pub fn token(&self) -> Option<Secret> {
        self.current.token()
    }

    /// 认出来的一件事交给跟核心的那一头；那一头不收了（桥在停）交回假。
    pub async fn inbound(&self, event: Event) -> bool {
        self.inbound.send(event).await.is_ok()
    }

    /// 说给人听的一句。
    pub fn tell(&self, notice: Notice) {
        (self.tell)(notice);
    }

    /// 状态变了：状态文件照这一刻再写。
    pub fn changed(&self) {
        self.changed.notify_one();
    }

    /// 机器人号 `bot` 收到的一件平台事件 `event` 原样交给挂在 `chat.event` 的包，不等。
    pub fn event(&self, bot: i64, event: &Value) {
        self.hooks.event(bot, event);
    }
}
