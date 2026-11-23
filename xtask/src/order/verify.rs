//! 核对排上了（施工 LW-IM-8，同核心 `xtask/src/order/verify.rs` 的 Linux 那一半）：只吃 `nm` 的输出和清单的文本。
//!
//! 清单里在符号表里找得到的函数合计 L 字节；这些函数里，起点落在「代码段最低的函数地址起、往后 1.25 L」以内的，按字节算要
//! 占九成以上。留两成五的余量是给对齐的空隙和链接器挪不动、插在中间的几段；换了链接器、参数没生效时这些函数散在整个代码段
//! 里，落在开头的只有一小部分。同名的局部函数可以有好几个，链接器照名字排的时候一起挪，这里也一起算。

use std::collections::{HashMap, HashSet};

use super::listing::{Function, functions};

/// 落在开头的字节要占的比例：分子、分母。
const SHARE: (u64, u64) = (9, 10);

/// 核对的结果。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Placed {
    /// 清单里不重复的名字数。
    pub(crate) listed: usize,
    /// 其中在符号表里找得到的。
    pub(crate) found: usize,
    /// 找得到的函数合计多少字节（L）。
    pub(crate) bytes: u64,
    /// 其中起点落在开头 1.25 L 以内的字节数。
    pub(crate) near: u64,
}

/// 照 `nm --defined-only -S` 的输出核对清单 `list` 排在了代码段开头。
///
/// # Errors
///
/// 清单是空的、清单里的名字一个都找不到、落在开头的不到九成：交回比例和可能的原因。
pub(crate) fn check(nm: &str, list: &str) -> Result<Placed, String> {
    let mut seen = HashSet::new();
    let listed: Vec<&str> = list
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty() && seen.insert(*name))
        .collect();
    if listed.is_empty() {
        return Err("清单是空的".to_string());
    }
    let all = functions(nm);
    let Some(start) = all.first().map(|function| function.address) else {
        return Err("nm 没列出代码段里的函数".to_string());
    };
    let mut by_name: HashMap<&str, Vec<&Function>> = HashMap::new();
    for function in &all {
        by_name
            .entry(function.name.as_str())
            .or_default()
            .push(function);
    }
    let picked: Vec<&Function> = listed
        .iter()
        .filter_map(|name| by_name.get(name))
        .flatten()
        .copied()
        .collect();
    let found = listed
        .iter()
        .filter(|name| by_name.contains_key(*name))
        .count();
    let bytes: u64 = picked.iter().map(|function| function.size).sum();
    if bytes == 0 {
        return Err(format!(
            "清单 {} 个名字，符号表里一个都找不到：清单不是照这一份程序录的",
            listed.len()
        ));
    }
    // 清单里的名字九成以上要在这一份程序里找得到：找不到的多，是清单照别的编法编的程序录的（符号名不一样），排上的只是零头。
    let count = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
    if count(found) * SHARE.1 < count(listed.len()) * SHARE.0 {
        return Err(format!(
            "清单 {} 个名字，这一份程序里只找到 {found} 个（要九成以上）：清单不是照这样编的程序录的",
            listed.len()
        ));
    }
    let limit = start + bytes + bytes / 4;
    let near: u64 = picked
        .iter()
        .filter(|function| function.address < limit)
        .map(|function| function.size)
        .sum();
    let placed = Placed {
        listed: listed.len(),
        found,
        bytes,
        near,
    };
    if near * SHARE.1 < bytes * SHARE.0 {
        return Err(format!(
            "清单 {} 个函数、合计 {bytes} 字节，排在代码段开头的只有 {near} 字节（要九成以上）：链接器、参数没生效？",
            placed.listed
        ));
    }
    Ok(placed)
}

#[cfg(test)]
mod tests;
