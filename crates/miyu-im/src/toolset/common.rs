//! 几个驱动的工具同用的几件（施工 I-183 下下：Telegram 的工具接上时从 OneBot 驱动挪来，行为不变）：参数的读法、查聊天记录、发言排行、
//! 定下一次看一眼、@ 人的参数和认人。工具名各平台自己起（`qq_*`、`tg_*`），说明、答的话照各自的 [`super::ToolSet::dir`]；这里只认参数、交 im 办的
//! （[`super::Effect`]）或者照 im 的聊天记录答。

pub mod args;
pub mod glance;
pub mod history;
pub mod mention;
pub mod ranking;
