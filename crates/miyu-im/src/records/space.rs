//! 聊天记录占多大、清理以后把空出来的还给硬盘（施工 I-230，todolist 第 53 条，项目主人 11-12 定：列表写总的、每个场所写约数）。
//!
//! - 总的照硬盘上的文件算（库文件加上还没并进去的 `-wal`）。
//! - 一个场所占多少库里分不出来（同一张表、同一份全文索引），照它的字、带的东西的占位在全部里占几成，把库里用着的那些分给它：
//!   约数。
//! - 删了的消息只是把页空出来，文件不变小；清理完了整理一遍（`VACUUM`），再把 `-wal` 并回去、截短，硬盘上才真的少了。

use std::ffi::OsString;
use std::path::Path;

use rusqlite::Connection;

/// 一条消息除了字、占位以外大约多占的字节（编号、时刻、两个索引里的那一项）：分约数时照它加，空消息也算一点。
pub(crate) const ROW: i64 = 64;

/// 库文件 `path` 连同 `-wal` 在硬盘上占多少字节；读不到的那一份算 0。
pub(crate) fn on_disk(path: &Path) -> u64 {
    let size = |path: &Path| std::fs::metadata(path).map_or(0, |meta| meta.len());
    let mut wal = OsString::from(path.as_os_str());
    wal.push("-wal");
    size(path) + size(Path::new(&wal))
}

/// 库里用着的字节：页数减空页，乘页的大小。
///
/// # Errors
///
/// 库读不出。
pub(crate) fn used(connection: &Connection) -> rusqlite::Result<i64> {
    let pragma = |name: &str| {
        connection.query_row(&format!("PRAGMA {name}"), [], |row| row.get::<_, i64>(0))
    };
    let (pages, free, size) = (
        pragma("page_count")?,
        pragma("freelist_count")?,
        pragma("page_size")?,
    );
    Ok((pages - free).max(0).saturating_mul(size))
}

/// 把 `used` 字节照各场所的分量 `weights` 分（约数，同样的先后）。分量都是 0 的都分 0。
pub(crate) fn shares(used: i64, weights: &[i64]) -> Vec<i64> {
    let total: i128 = weights
        .iter()
        .map(|weight| i128::from(*weight.max(&0)))
        .sum();
    if total == 0 {
        return vec![0; weights.len()];
    }
    weights
        .iter()
        .map(|weight| {
            let share = i128::from(used.max(0)) * i128::from(*weight.max(&0)) / total;
            i64::try_from(share).unwrap_or(i64::MAX)
        })
        .collect()
}

/// 整理（模块的说明）：`VACUUM` 把空页还给硬盘，再把 `-wal` 并回库文件、截短。并的时候有人正读着的，留到下一回并。
///
/// # Errors
///
/// 库写不进、等锁等过了（写的线程正写着一大批）。
pub(crate) fn compact(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch("VACUUM;")?;
    connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
}

#[cfg(test)]
mod tests;
