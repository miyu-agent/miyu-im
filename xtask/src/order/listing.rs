//! 把模拟器的日志换成函数清单（施工 LW-IM-8）：只吃 `nm` 的输出和日志的文本，不碰文件系统（同核心 `miyu-perf` 的
//! `order/listing.rs`）。
//!
//! 日志（QEMU 带 `-d page,in_asm`）开头是一张内存布局：`start end size prot` 那一行下面第一行是程序整段的范围，起点就是程序
//! 装在哪（位置无关的程序每次可能不同）。之后每一块代码头一回翻译时一行一条指令，开头是地址：
//!
//! ```text
//! 0x00005555555ab3c0:  f3 0f 1e fa              endbr64
//! ```
//!
//! 地址减去装载起点落在代码段里的，照符号表找到它所在的函数；每个函数只记第一次，照先后。动态库、动态链接器的地址不在
//! 代码段里，不算。

/// 符号表里的一个函数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function {
    /// 地址（程序里的，位置无关的从 0 起）。
    pub(crate) address: u64,
    /// 大小，字节。
    pub(crate) size: u64,
    /// 符号的原名（没有还原过的）。
    pub(crate) name: String,
}

/// 照 `nm --defined-only -S` 的输出读出代码段里的函数（类型 `t`、`T`、`w`、`W`、大小不是 0），照地址排。
pub(crate) fn functions(nm: &str) -> Vec<Function> {
    let mut found: Vec<Function> = nm
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, ' ');
            let address = u64::from_str_radix(parts.next()?, 16).ok()?;
            let size = u64::from_str_radix(parts.next()?, 16).ok()?;
            let kind = parts.next()?;
            let name = parts.next()?;
            (matches!(kind, "t" | "T" | "w" | "W") && size > 0).then(|| Function {
                address,
                size,
                name: name.to_string(),
            })
        })
        .collect();
    found.sort_by_key(|function| function.address);
    found
}

/// 日志里程序的装载起点：内存布局那张表头下面的第一行的起点。没有的（不是带 `page` 录的）是空的。
pub(crate) fn base(log: &str) -> Option<u64> {
    let mut lines = log.lines();
    lines.find(|line| line.starts_with("start "))?;
    let first = lines.next()?;
    u64::from_str_radix(first.split('-').next()?, 16).ok()
}

/// 一条指令行的地址；不是指令行的是空的。
pub(crate) fn address(line: &str) -> Option<u64> {
    let rest = line.strip_prefix("0x")?;
    let (hex, _) = rest.split_once(':')?;
    u64::from_str_radix(hex, 16).ok()
}

/// 照先后收函数：几份日志接着收，收过的不再记。
pub(crate) struct Listing<'a> {
    /// 符号表里的函数，照地址排。
    functions: &'a [Function],
    /// 每个函数收过没有。
    seen: Vec<bool>,
    /// 收到的，照第一次的先后：在 `functions` 里的位置。
    order: Vec<usize>,
}

impl<'a> Listing<'a> {
    /// 空的，函数照 `functions`（[`functions`] 读出来的）认。
    pub(crate) fn new(functions: &'a [Function]) -> Listing<'a> {
        Listing {
            functions,
            seen: vec![false; functions.len()],
            order: Vec::new(),
        }
    }

    /// 收一份日志的一行：`base` 是这一份的装载起点。
    pub(crate) fn line(&mut self, base: u64, line: &str) {
        let Some(at) = address(line).and_then(|at| at.checked_sub(base)) else {
            return;
        };
        let index = self
            .functions
            .partition_point(|function| function.address <= at);
        let Some(index) = index.checked_sub(1) else {
            return;
        };
        let function = &self.functions[index];
        if at < function.address + function.size && !self.seen[index] {
            self.seen[index] = true;
            self.order.push(index);
        }
    }

    /// 收到第几个为止（下一段从这里接着数）。
    pub(crate) fn count(&self) -> usize {
        self.order.len()
    }

    /// 收到的函数名，照先后，一行一个。
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.order
            .iter()
            .map(|&index| self.functions[index].name.as_str())
    }

    /// 收到的函数合计多少字节。
    pub(crate) fn bytes(&self) -> u64 {
        self.order
            .iter()
            .map(|&index| self.functions[index].size)
            .sum()
    }
}

#[cfg(test)]
mod tests;
