//! 开端口、接连接、换端口（施工 I-3 从宿主的 `serve.rs`、`running/rebind.rs` 挪来；`onebot.md` 第一条「怎么走」第 1、2 条，
//! 「施工时定的」第 45、167 条）：
//!
//! 1. 起来：听宿主交来的地址 `host`（出厂 `0.0.0.0`，施工 I-27）、端口 `listen`。被占了：`Failure::PortInUse`，桥起不来。听上了说一行
//!    （`Notice::Listening`）；令牌没设的也照开，NapCat 连进来一律 401，接着说 `Notice::NoToken`、记一行运行日志。
//! 2. 每个 TCP 连接一个任务（`super::accept`）；接不了的（打开的文件太多这类）歇一下再接，不空转。
//! 3. 核心推来的配置变了（宿主经 `Gate::config` 通知）：令牌换了记一行（不记值，已经连着的不断，只有连进来的那一下比令牌）；
//!    地址、端口变了开新的，开上了换掉旧的，开不了的不换（旧的照旧，不另试：人换一个再存，配置变了核心再推）。已经接进来的连接不断。
//! 4. 地址、端口、有没有令牌报给宿主（`Gate::report`）：后台页的 `status`、状态文件照它说 `host`、`listen`、`token`。

use std::net::IpAddr;
use std::sync::Arc;

use serde_json::{Map, Value, json};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinSet;

use miyu_im::TARGET;
use miyu_im::driver::{Gate, Running};
use miyu_im::serve::{Failure, Notice};

use crate::settings::Settings;

/// 照 `gate` 交来的配置起来：开好端口，交回跑着的那一段（模块的说明第 1 条）。
pub(super) async fn start(gate: Arc<Gate>) -> Result<Running, Failure> {
    let mut config = gate.config();
    let settings = read(&config.borrow_and_update(), &gate)
        .ok_or_else(|| Failure::Start("no port in listen".to_string()))?;
    let (listener, port) = bind(settings.host, settings.port).await?;
    tracing::info!(target: TARGET, host = %settings.host, port, "listening");
    gate.tell(Notice::Listening {
        host: settings.host.to_string(),
        port,
    });
    if settings.token.is_none() {
        tracing::warn!(target: TARGET, "no token yet");
        gate.tell(Notice::NoToken);
    }
    gate.report(status(port, &settings));
    Ok(Box::pin(accepting(
        listener,
        (settings, port),
        config,
        gate,
    )))
}

/// 读宿主交来的配置，地址、端口读不出的照清单的默认值。
fn read(config: &Map<String, Value>, gate: &Gate) -> Option<Settings> {
    Settings::read(config, |name| gate.default(name))
}

/// 接连接、配置变了换（模块的说明第 2、3 条）：一直跑，桥停下时随宿主的任务组一起放下，接进来的连接也一起放下。
async fn accepting(
    mut listener: TcpListener,
    (mut settings, mut port): (Settings, u16),
    mut config: watch::Receiver<Map<String, Value>>,
    gate: Arc<Gate>,
) {
    let mut connections = JoinSet::new();
    let mut watching = true;
    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => {
                    connections.spawn(super::accept(stream, Arc::clone(&gate)));
                }
                Err(error) => {
                    tracing::warn!(target: TARGET, error = %error, "accept failed");
                    tokio::time::sleep(gate.tuning().accept_retry()).await;
                }
            },
            changed = config.changed(), if watching => {
                if changed.is_err() {
                    // 宿主不发了（桥在停）：照现在的接着接。
                    watching = false;
                    continue;
                }
                let Some(wanted) = read(&config.borrow_and_update(), &gate) else {
                    continue;
                };
                if wanted.token != settings.token {
                    tracing::info!(target: TARGET, "token changed");
                }
                if (wanted.host, wanted.port) != (settings.host, settings.port) {
                    match bind(wanted.host, wanted.port).await {
                        Ok((bound, now)) => {
                            listener = bound;
                            port = now;
                            tracing::info!(target: TARGET, host = %wanted.host, listen = port, "applied");
                        }
                        Err(Failure::PortInUse(port)) => {
                            tracing::warn!(target: TARGET, port, "apply port in use");
                            continue;
                        }
                        Err(other) => {
                            tracing::warn!(target: TARGET, failure = ?other, "apply failed");
                            continue;
                        }
                    }
                }
                settings = wanted;
                gate.report(status(port, &settings));
            },
            Some(ended) = connections.join_next() => {
                if let Err(error) = ended {
                    tracing::error!(target: TARGET, error = %error, "connection crashed");
                }
            },
        }
    }
}

/// 报给宿主的：听的地址、实际听的端口，有没有令牌（`set`、`none`）。
fn status(port: u16, settings: &Settings) -> Map<String, Value> {
    let token = match settings.token {
        Some(_) => "set",
        None => "none",
    };
    let mut status = Map::new();
    status.insert("host".into(), json!(settings.host.to_string()));
    status.insert("listen".into(), json!(port));
    status.insert("token".into(), json!(token));
    status
}

/// 听地址 `host` 的 `wanted`：交回监听和实际的端口（`0` 是系统挑的那一个）。被占了的说是哪个端口。
async fn bind(host: IpAddr, wanted: u16) -> Result<(TcpListener, u16), Failure> {
    let listener = match TcpListener::bind((host, wanted)).await {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            return Err(Failure::PortInUse(wanted));
        }
        Err(error) => return Err(Failure::Start(error.to_string())),
    };
    let port = listener
        .local_addr()
        .map_err(|error| Failure::Start(error.to_string()))?
        .port();
    Ok((listener, port))
}
