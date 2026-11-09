//! 接入通讯平台的 Telegram 驱动（施工 I-183 中上，设计 07 第 3.5、4.5 节）：官方机器人接口（Bot API），长轮询收消息（im 往外连，
//! 不开端口），每个连接一个机器人令牌、能单独设代理。宿主 `miyu-im` 不依赖这里；程序 `miyu-im`（`miyu-im-app`）把 [`Telegram`]
//! 和 OneBot 一起交给宿主。
//!
//! 骨架（I-183 中上）：连上（`getMe`）、登记进宿主、长轮询、409 和 401 和 429、断线重连、代理、状态。I-183 中下收发字、引用、@、
//! 话题、长文拆段。工具和页面随 I-183 下，图、贴表情、撤回、文件随 I-185。

mod api;
mod bot;
mod known;
mod link;
mod menu;
mod probe;
mod receive;
mod running;
mod send;
pub mod settings;

use std::sync::Arc;

use miyu_im::chat::{Cap, Platform};
use miyu_im::driver::{Driver, Gate, Probing, Started, TokenTable};
use serde_json::{Map, Value};

pub use api::{Failure, Proxy};

/// 平台：场所、人、命令编号照它的名字拼（`tg:<号>`）。
pub const PLATFORM: Platform = Platform::new("tg");

/// Telegram 的驱动。
#[derive(Debug, Clone, Copy, Default)]
pub struct Telegram;

impl Driver for Telegram {
    fn platform(&self) -> Platform {
        PLATFORM
    }

    fn caps(&self) -> &'static [Cap] {
        // 命令菜单（I-183 下上）；别的随后面几步加（设计 07 第 3.5 节的表）。
        &[Cap::Menu]
    }

    fn settings(&self) -> &'static [&'static str] {
        settings::NAMES
    }

    fn secrets(&self) -> &'static [&'static str] {
        settings::SECRETS
    }

    fn token_table(&self) -> Option<TokenTable> {
        // 一个连接一个机器人令牌，另带代理（I-183 下中）。
        Some(TokenTable {
            tokens: "tokens",
            columns: &[("proxies", "proxy", "env")],
        })
    }

    fn probe(&self, params: Value, config: Map<String, Value>) -> Probing {
        Box::pin(probe::probe(params, config))
    }

    fn start(&self, gate: Arc<Gate>) -> Started {
        Box::pin(running::start(gate))
    }
}
