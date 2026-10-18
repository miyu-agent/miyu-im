//! 卡片存在哪（`onebot.md` 第一条「场所卡片」）：`<数据根>/state/packages/im/cards.json`，和状态文件同一个目录，由桥读写。
//!
//! - 读（[`load`]）：没有的是没有卡片；读不成、超过 1 MiB、不是这个样子的交回原因，桥照没有卡片用、记一行运行日志，不挡桥起来
//!   （施工单「风险」第 2 条）。
//! - 写（[`save`]）：先写临时文件再改名盖上（`miyu_client::places::write`），读的人看不到写了一半的；和盘上一样的不写。

use std::io;
use std::path::PathBuf;

use miyu_client::places::DataRoot;

use super::Cards;
use crate::PACKAGE;

/// 文件名：在这个包放状态的目录里。
const FILE: &str = "cards.json";

/// 最多读多少个字节：同规则文件。
const MAX_BYTES: u64 = 1024 * 1024;

/// 数据根 `root` 里卡片存在哪。
pub fn path(root: &DataRoot) -> PathBuf {
    root.state().join("packages").join(PACKAGE).join(FILE)
}

/// 读数据根 `root` 里的卡片。没有那个文件的是没有卡片。
///
/// # Errors
///
/// 读不成、太大、不是卡片的样子：原因（给运行日志、`cards.get`）。
pub fn load(root: &DataRoot) -> Result<Cards, String> {
    let path = path(root);
    let size = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Cards::default()),
        Err(error) => return Err(error.to_string()),
    };
    if size > MAX_BYTES {
        return Err(format!("larger than {MAX_BYTES} bytes"));
    }
    let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

/// 把 `cards` 写进数据根 `root`：交回写没写（和盘上一样的不写）。
///
/// # Errors
///
/// 建不了目录、写不进、改不了名。
pub fn save(root: &DataRoot, cards: &Cards) -> io::Result<bool> {
    let mut bytes = serde_json::to_vec_pretty(cards).map_err(io::Error::other)?;
    bytes.push(b'\n');
    miyu_client::places::write(&path(root), &bytes)
}
