//! 开端口、接连接、换端口（施工 I-3 从宿主的 `serve.rs`、`running/rebind.rs` 挪来；`onebot.md` 第一条「怎么走」第 1、2 条，
//! 「施工时定的」第 45、167 条；施工 I-81 下起一个连接一个端口，设计 07 第 7.7 节）：
//!
//! 1. 起来：连接表（[`Settings::all`]）里每个连接听它的地址、端口。一个都听不上的：头一个的原因（`Failure::PortInUse` 这类），
//!    桥起不来；听上了几个的照它们起来，听不上的记一行运行日志。听上一个说一行（`Notice::Listening`）；有连接没设令牌的也照开，
//!    它的端口 NapCat 连进来一律 401，接着说 `Notice::NoToken`、记一行运行日志。
//! 2. 每个端口一个任务接 TCP 连接，接到的交回这里，每个连接一个任务（`super::accept`），带着它是从哪个端口来的；接不了的
//!    （打开的文件太多这类）歇一下再接，不空转。
//! 3. 核心推来的配置变了（宿主经 `Gate::config` 通知）：令牌换了记一行（不记值，已经连着的不断，只有连进来的那一下比令牌）；
//!    新的端口开上，不要了的端口关掉；同一个端口换了地址的先开新的，开上了换掉旧的，开不了的不换（旧的照旧，不另试：人换一个
//!    再存，配置变了核心再推）。已经接进来的连接都不断。
//! 4. 报给宿主（`Gate::report`）：头一个连接的 `host`、`listen`、`token`（后台页的 `status`、状态文件照旧读这几格），加上
//!    `connections`：每个连接的地址、实际听的端口、有没有令牌。

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio::task::{AbortHandle, JoinSet};

use miyu_client::settings::Secret;
use miyu_im::TARGET;
use miyu_im::driver::{Gate, Running};
use miyu_im::serve::{Failure, Notice};

use crate::settings::Settings;

/// 每个听着的端口（实际的）现在的令牌：连进来的那一下照它比（`super::admitted`）。
pub(super) type Tokens = Arc<Mutex<BTreeMap<u16, Option<Secret>>>>;

/// 接到的连接和它是从哪个端口来的：每个端口的任务交过来，这里起任务接。
type Accepted = (TcpStream, u16);

/// 一个听着的端口：连接表里的那一项、实际听的端口、接连接的任务。
struct Listening {
    settings: Settings,
    port: u16,
    task: AbortHandle,
}

/// 照 `gate` 交来的配置起来：开好端口，交回跑着的那一段（模块的说明第 1 条）。
pub(super) async fn start(gate: Arc<Gate>) -> Result<Running, Failure> {
    let mut config = gate.config();
    let wanted = read(&config.borrow_and_update(), &gate);
    if wanted.is_empty() {
        return Err(Failure::Start("no port in listen".to_string()));
    }
    let (sender, accepted) = mpsc::channel(64);
    let tokens = Tokens::default();
    // 接连接的任务都在这一组里：桥停下时跑着的那一段放下，这一组跟着放下，端口都关掉。
    let mut ports = JoinSet::new();
    let mut listening = Vec::new();
    let mut first_failure = None;
    for settings in wanted {
        match open(settings, (&sender, &mut ports), &gate).await {
            Ok(one) => listening.push(one),
            Err(failure) => {
                tracing::warn!(target: TARGET, failure = ?failure, "listen failed");
                first_failure.get_or_insert(failure);
            }
        }
    }
    if listening.is_empty() {
        return Err(first_failure.unwrap_or_else(|| Failure::Start("no port".to_string())));
    }
    for one in &listening {
        gate.tell(Notice::Listening {
            host: one.settings.host.to_string(),
            port: one.port,
        });
    }
    if listening.iter().any(|one| one.settings.token.is_none()) {
        tracing::warn!(target: TARGET, "no token yet");
        gate.tell(Notice::NoToken);
    }
    publish(&listening, &tokens, &gate);
    Ok(Box::pin(accepting(
        (listening, ports),
        (sender, accepted),
        tokens,
        config,
        gate,
    )))
}

/// 读宿主交来的连接表，地址、端口读不出的照清单的默认值。
fn read(config: &Map<String, Value>, gate: &Gate) -> Vec<Settings> {
    Settings::all(config, |name| gate.default(name))
}

/// 开 `settings` 的端口，在 `ports` 里起一个任务接连接，接到的交给 `sender`。
async fn open(
    settings: Settings,
    (sender, ports): (&mpsc::Sender<Accepted>, &mut JoinSet<()>),
    gate: &Arc<Gate>,
) -> Result<Listening, Failure> {
    let (listener, port) = bind(settings.host, settings.port).await?;
    tracing::info!(target: TARGET, host = %settings.host, port, "listening");
    let task = ports.spawn(listen(listener, port, sender.clone(), Arc::clone(gate)));
    Ok(Listening {
        settings,
        port,
        task,
    })
}

/// 一个端口接连接：接到的交回去（带上端口），接不了的歇一下再接；那一头不收了就停。
async fn listen(listener: TcpListener, port: u16, sender: mpsc::Sender<Accepted>, gate: Arc<Gate>) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                if sender.send((stream, port)).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "accept failed");
                tokio::time::sleep(gate.tuning().accept_retry()).await;
            }
        }
    }
}

/// 接连接、配置变了换（模块的说明第 2、3 条）：一直跑，桥停下时随宿主的任务组一起放下，接进来的连接也一起放下。
async fn accepting(
    (mut listening, mut ports): (Vec<Listening>, JoinSet<()>),
    (sender, mut accepted): (mpsc::Sender<Accepted>, mpsc::Receiver<Accepted>),
    tokens: Tokens,
    mut config: watch::Receiver<Map<String, Value>>,
    gate: Arc<Gate>,
) {
    let mut connections = JoinSet::new();
    let mut watching = true;
    loop {
        tokio::select! {
            Some((stream, port)) = accepted.recv() => {
                connections.spawn(super::accept(stream, Arc::clone(&gate), (port, Arc::clone(&tokens))));
            },
            changed = config.changed(), if watching => {
                if changed.is_err() {
                    // 宿主不发了（桥在停）：照现在的接着接。
                    watching = false;
                    continue;
                }
                let wanted = read(&config.borrow_and_update(), &gate);
                if wanted.is_empty() {
                    continue;
                }
                listening = apply(listening, wanted, (&sender, &mut ports), &gate).await;
                publish(&listening, &tokens, &gate);
            },
            Some(_) = ports.join_next() => {
                // 关掉了的端口的任务：收掉就是。
            },
            Some(ended) = connections.join_next() => {
                if let Err(error) = ended {
                    tracing::error!(target: TARGET, error = %error, "connection crashed");
                }
            },
        }
    }
}

/// 换成连接表 `wanted`（模块的说明第 3 条）：交回换好以后听着的那几个。先照端口认出没变的；剩下的新连接和剩下的旧连接照先后
/// 一一配对，当成换了端口、地址：新的开上了关掉配对的旧的，开不上的留着旧的。多出来的旧连接关掉。
async fn apply(
    mut listening: Vec<Listening>,
    wanted: Vec<Settings>,
    (sender, ports): (&mpsc::Sender<Accepted>, &mut JoinSet<()>),
    gate: &Arc<Gate>,
) -> Vec<Listening> {
    let mut kept = Vec::new();
    let mut fresh = Vec::new();
    for settings in wanted {
        // 同一个连接：连接表写的端口一样（写 0 让系统挑的照先后认头一个也写 0 的），或者就是它实际在听的那一个（原来写 0，
        // 推来的是挑好的那一个）；地址也没变。
        let same = listening.iter().position(|one| {
            one.settings.host == settings.host
                && (one.settings.port == settings.port
                    || (settings.port != 0 && one.port == settings.port))
        });
        match same.map(|at| listening.remove(at)) {
            Some(mut one) => {
                if one.settings.token != settings.token {
                    tracing::info!(target: TARGET, port = one.port, "token changed");
                }
                one.settings = settings;
                kept.push(one);
            }
            None => fresh.push(settings),
        }
    }
    let mut olds = listening.into_iter();
    for settings in fresh {
        let old = olds.next();
        match open(settings.clone(), (sender, &mut *ports), gate).await {
            Ok(opened) => {
                tracing::info!(target: TARGET, host = %settings.host, listen = opened.port, "applied");
                if let Some(old) = old {
                    old.task.abort();
                }
                kept.push(opened);
            }
            Err(failure) => {
                tracing::warn!(target: TARGET, port = settings.port, failure = ?failure, "apply failed");
                kept.extend(old);
            }
        }
    }
    // 连接表里没有了的：关掉它的端口，已经接进来的连接不断。
    for gone in olds {
        tracing::info!(target: TARGET, port = gone.port, "closed");
        gone.task.abort();
    }
    kept
}

/// 换好的连接表交出去：每个端口的令牌、报给宿主的状态（模块的说明第 4 条）。
fn publish(listening: &[Listening], tokens: &Tokens, gate: &Gate) {
    *tokens.lock().unwrap_or_else(PoisonError::into_inner) = listening
        .iter()
        .map(|one| (one.port, one.settings.token.clone()))
        .collect();
    gate.report(status(listening));
}

/// 有没有令牌：`set`、`none`。
fn token_word(settings: &Settings) -> &'static str {
    match settings.token {
        Some(_) => "set",
        None => "none",
    }
}

/// 报给宿主的：头一个连接的地址、实际听的端口、有没有令牌，加上每个连接的这三样。
fn status(listening: &[Listening]) -> Map<String, Value> {
    let mut status = Map::new();
    if let Some(first) = listening.first() {
        status.insert("host".into(), json!(first.settings.host.to_string()));
        status.insert("listen".into(), json!(first.port));
        status.insert("token".into(), json!(token_word(&first.settings)));
    }
    let connections: Vec<Value> = listening
        .iter()
        .map(|one| {
            json!({
                "host": one.settings.host.to_string(),
                "listen": one.port,
                "token": token_word(&one.settings),
            })
        })
        .collect();
    status.insert("connections".into(), json!(connections));
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
