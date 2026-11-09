//! 驱动跑起来（施工 I-183 中上）：照配置的连接表每个连接一个任务（[`crate::bot::run`]），核心推来新的配置照它换；各个连接的
//! 状态合成一份报给宿主（`Gate::report`：`connections`，页面、状态文件照它）。
//!
//! - 一个连接都没有也照样起来（没接 Telegram 的人不受影响），不联网，只等配置。
//! - 配置变了：令牌、代理、Bot API 地址都没变的连接照旧跑（不断、不重连）；变了的、不要了的放下（放下时拿掉登记），新的起上。连接照
//!   令牌认，不照第几个：删了前面的一个，后面的不重连。

use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value};
use tokio::task::{AbortHandle, JoinSet};

use miyu_im::driver::{Gate, Running};
use miyu_im::serve::Failure;

use crate::bot::{self, Shared, State};
use crate::settings::{self, Connection};

/// 跑着的一个连接：照什么起的、状态、任务。
struct Live {
    connection: Connection,
    api: String,
    shared: Arc<Shared>,
    task: AbortHandle,
}

/// 照 `gate` 交来的配置起来，交回跑着的那一段。
pub(crate) async fn start(gate: Arc<Gate>) -> Result<Running, Failure> {
    Ok(Box::pin(run(gate)))
}

/// 一直跑：换连接、等配置。
async fn run(gate: Arc<Gate>) {
    let mut config = gate.config();
    let mut tasks = JoinSet::new();
    let lives: Arc<Mutex<Vec<Live>>> = Arc::default();
    loop {
        let wanted = settings::read(&config.borrow_and_update(), |name| gate.default(name));
        apply(&gate, wanted, (&lives, &mut tasks));
        report(&gate, &lives);
        loop {
            tokio::select! {
                changed = config.changed() => {
                    if changed.is_err() {
                        // 宿主不推了（桥在停）：等着被放下。
                        std::future::pending::<()>().await;
                    }
                    break;
                }
                // 跑完了的连接（没令牌、令牌失效）收掉，状态留着。
                Some(_) = tasks.join_next() => {}
            }
        }
    }
}

/// 照新的连接表换：没变的留着，别的放下、起新的。
fn apply(
    gate: &Arc<Gate>,
    (wanted, api): (Vec<Connection>, String),
    (lives, tasks): (&Arc<Mutex<Vec<Live>>>, &mut JoinSet<()>),
) {
    let mut held = lives.lock().unwrap_or_else(PoisonError::into_inner);
    let mut kept: Vec<Live> = Vec::new();
    let mut old: Vec<Live> = held.drain(..).collect();
    for connection in wanted {
        let same = old.iter().position(|live| {
            live.api == api
                && live.connection.token == connection.token
                && live.connection.proxy == connection.proxy
                && connection.token.is_some()
        });
        if let Some(at) = same {
            let live = old.remove(at);
            live.shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .index = connection.index;
            kept.push(Live { connection, ..live });
            continue;
        }
        // 同一个令牌只换了代理的：认出过的机器人照旧报着，重连的这一阵后台页照号找得到它（改完马上删、再改）。
        let (bot, privacy) = old
            .iter()
            .find(|live| connection.token.is_some() && live.connection.token == connection.token)
            .map(|live| {
                let state = live
                    .shared
                    .state
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                (state.bot.clone(), state.privacy)
            })
            .unwrap_or_default();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                index: connection.index,
                token: connection.token.is_some(),
                proxy: connection.proxy.written().to_string(),
                bot,
                privacy,
                ..State::default()
            }),
            changed: {
                let (gate, lives) = (Arc::downgrade(gate), Arc::downgrade(lives));
                Box::new(move || {
                    if let (Some(gate), Some(lives)) = (gate.upgrade(), lives.upgrade()) {
                        report(&gate, &lives);
                    }
                })
            },
        });
        let task = tasks.spawn(bot::run(
            Arc::clone(gate),
            connection.clone(),
            api.clone(),
            Arc::clone(&shared),
        ));
        kept.push(Live {
            connection,
            api: api.clone(),
            shared,
            task,
        });
    }
    for live in old {
        live.task.abort();
    }
    kept.sort_by_key(|live| live.connection.index);
    *held = kept;
}

/// 各个连接的状态合成一份报给宿主：`connections` 照连接表的先后。
fn report(gate: &Gate, lives: &Mutex<Vec<Live>>) {
    // 连接改状态时先放开自己的锁再叫这里（`Shared::update`），这里先拿表的锁再拿各个连接的，不会互相等。
    let connections: Vec<Value> = lives
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .map(|live| {
            live.shared
                .state
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .fields()
        })
        .collect();
    let mut fields = Map::new();
    fields.insert("connections".to_string(), Value::Array(connections));
    gate.report(fields);
}
