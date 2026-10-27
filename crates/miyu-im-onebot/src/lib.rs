//! 接入通讯平台的 OneBot v11 驱动（施工 I-2 下，设计 01 第三节）：NapCat 这类实现反连进来的 WebSocket（`listen`），OneBot 的
//! 事件和动作怎么认、怎么拼（[`onebot`]），宿主要做的事照 OneBot 办（[`onebot::Wire`]）。宿主 `miyu-im` 不依赖这里；程序
//! `miyu-im`（`main.rs`）把 [`OneBot`] 交给宿主。

mod admin;
mod backfill;
mod identity;
mod listen;
mod lookups;
pub mod onebot;
pub mod settings;
mod tools;

pub use listen::OneBot;
