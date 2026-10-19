//! 页面服务（设计 01 第九节第 1、3 条，施工 I-5 上）：im 自己的页面，只听本机。
//!
//! 1. 起来：听 `127.0.0.1` 的 `im.web_port`（`crate::current` 交来的页面服务那几项）。听上了说一句（`Notice::Page`）、端口报进
//!    状态（[`Port`]：后台页的 `status`、状态文件的 `web`）；被占了说一句（`Notice::PageInUse`）、记一行运行日志，桥照跑（页面是给人
//!    改设置的，NapCat 收发消息不靠它）。
//! 2. 每个 TCP 连接一个任务：当 HTTP 读（`request`），升级成 WebSocket 的接着当通道跑（`channel`）。接不了的歇一下再接。
//! 3. 端口变了开新的，开上了换掉旧的；开不了的不换（旧的照旧；原来没听上的照旧没有）。已经接进来的连接不断。
//!
//! 登录（`logins`）只在内存里：交接票换来的登录令牌照它另开一条人的连接（`ticket`），页面上要人的权限做的事以后走这条连接。

mod channel;
mod files;
mod guard;
mod logins;
mod request;
#[cfg(test)]
mod tests;
mod ticket;

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use hyper::upgrade::OnUpgrade;
use hyper_util::rt::TokioIo;
use serde_json::{Map, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, watch};
use tokio::task::JoinSet;

use miyu_client::open;
use miyu_client::places::DataRoot;

use crate::TARGET;
use crate::core::caller::Caller;
use crate::serve::Notice;
use crate::tuning::Tuning;
use logins::Logins;

/// 页面服务的配置项里端口的名字（`crate::settings::SITE`）。
const PORT: &str = "web_port";

/// 一份登录拿着的人的连接：一问一答，锁着用。
type Person = tokio::sync::Mutex<open::Core>;

/// 页面服务要的。
pub(crate) struct Site {
    /// 数据根：人的连接经它找本机套接字。
    pub(crate) root: DataRoot,
    /// 页面的文件：包文件夹的 `site/`。
    pub(crate) files: PathBuf,
    /// 扩展连接上的调用口：换交接票（`login.redeem`）。
    pub(crate) caller: Caller,
    /// 桥自己的数：接不了连接时歇多久。
    pub(crate) tuning: Tuning,
    /// 登录了的。
    logins: Logins<Person>,
}

impl Site {
    /// 数据根 `root`、页面的文件在 `files`、扩展连接上的调用口 `caller`、桥自己的数 `tuning`；还没有人登录。
    pub(crate) fn new(root: DataRoot, files: PathBuf, caller: Caller, tuning: Tuning) -> Site {
        Site {
            root,
            files,
            caller,
            tuning,
            logins: Logins::default(),
        }
    }
}

/// 页面服务实际听的端口：没听上的是空的。状态文件、后台页的 `status` 照它说 `web`。
pub(crate) struct Port {
    port: Mutex<Option<u16>>,
    /// 变了叫一声状态文件。
    changed: Arc<Notify>,
}

impl Port {
    /// 变了叫 `changed`（状态文件照这一刻再写）。
    pub(crate) fn new(changed: Arc<Notify>) -> Port {
        Port {
            port: Mutex::new(None),
            changed,
        }
    }

    /// 实际听的端口。
    pub(crate) fn get(&self) -> Option<u16> {
        *self.port.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 换成 `port`，叫一声状态文件。
    fn set(&self, port: u16) {
        *self.port.lock().unwrap_or_else(PoisonError::into_inner) = Some(port);
        self.changed.notify_one();
    }
}

/// 照 `config`（页面服务的那几项）起来（模块的说明第 1 条），交回接连接、换端口的那一段：一直跑，桥停下时随宿主的任务组一起
/// 放下。听上的、被占的都照 `tell` 说。
pub(crate) async fn start(
    site: Arc<Site>,
    mut config: watch::Receiver<Map<String, Value>>,
    tell: Arc<dyn Fn(Notice) + Send + Sync>,
    port: Arc<Port>,
) -> impl Future<Output = ()> {
    let wanted = wanted(&config.borrow_and_update());
    let listener = match wanted {
        Some(wanted) => bind(wanted, &tell, &port).await,
        None => {
            tracing::warn!(target: TARGET, "no page port");
            None
        }
    };
    accepting(listener, (wanted, config), site, (tell, port))
}

/// 接连接、端口变了换（模块的说明第 2、3 条）。
async fn accepting(
    mut listener: Option<(TcpListener, u16)>,
    (mut wanted, mut config): (Option<u16>, watch::Receiver<Map<String, Value>>),
    site: Arc<Site>,
    (tell, port): (Arc<dyn Fn(Notice) + Send + Sync>, Arc<Port>),
) {
    let mut connections = JoinSet::new();
    let mut watching = true;
    loop {
        let accepted = async {
            match &listener {
                Some((listener, port)) => (listener.accept().await, *port),
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            (accepted, port) = accepted => match accepted {
                Ok((stream, _)) => {
                    connections.spawn(accept(stream, port, Arc::clone(&site)));
                }
                Err(error) => {
                    tracing::warn!(target: TARGET, error = %error, "page accept failed");
                    tokio::time::sleep(site.tuning.accept_retry()).await;
                }
            },
            changed = config.changed(), if watching => {
                if changed.is_err() {
                    // 宿主不发了（桥在停）：照现在的接着接。
                    watching = false;
                    continue;
                }
                let now = self::wanted(&config.borrow_and_update());
                if now == wanted {
                    continue;
                }
                wanted = now;
                if let Some(now) = now
                    && let Some(bound) = bind(now, &tell, &port).await
                {
                    listener = Some(bound);
                }
            },
            Some(ended) = connections.join_next() => {
                if let Err(error) = ended {
                    tracing::error!(target: TARGET, error = %error, "page connection crashed");
                }
            },
        }
    }
}

/// 配置里要的端口；没有的、不是端口的是空的。
fn wanted(config: &Map<String, Value>) -> Option<u16> {
    config
        .get(PORT)
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
}

/// 只听 `127.0.0.1` 的 `wanted`（`0` 是系统挑一个）：听上了说、报端口，交回监听和实际的端口；被占了、听不了的说一句，是空的。
async fn bind(
    wanted: u16,
    tell: &Arc<dyn Fn(Notice) + Send + Sync>,
    port: &Port,
) -> Option<(TcpListener, u16)> {
    let listener = match TcpListener::bind(("127.0.0.1", wanted)).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::warn!(target: TARGET, port = wanted, error = %error, "page port not opened");
            tell(Notice::PageInUse { port: wanted });
            return None;
        }
    };
    let Ok(actual) = listener.local_addr().map(|address| address.port()) else {
        tracing::warn!(target: TARGET, port = wanted, "page port unknown");
        return None;
    };
    tracing::info!(target: TARGET, port = actual, "page listening");
    tell(Notice::Page { port: actual });
    port.set(actual);
    Some((listener, actual))
}

/// 接一个 TCP 连接：当 HTTP 读，升级成了的接着当通道跑，直到断开。`port` 是它进来的端口。
async fn accept(stream: TcpStream, port: u16, site: Arc<Site>) {
    let slot: Arc<Mutex<Option<OnUpgrade>>> = Arc::default();
    let service = {
        let (slot, site) = (Arc::clone(&slot), Arc::clone(&site));
        hyper::service::service_fn(move |incoming| {
            let (slot, site) = (Arc::clone(&slot), Arc::clone(&site));
            async move { Ok::<_, Infallible>(request::handle(incoming, port, &site, &slot).await) }
        })
    };
    let served = hyper::server::conn::http1::Builder::new()
        .serve_connection(TokioIo::new(stream), service)
        .with_upgrades()
        .await;
    if let Err(error) = served {
        tracing::debug!(target: TARGET, error = %error, "page connection ended");
    }
    let upgrade = slot.lock().unwrap_or_else(PoisonError::into_inner).take();
    let Some(upgrade) = upgrade else {
        return;
    };
    match upgrade.await {
        Ok(upgraded) => channel::run(TokioIo::new(upgraded), site).await,
        Err(error) => tracing::debug!(target: TARGET, error = %error, "channel not upgraded"),
    }
}
