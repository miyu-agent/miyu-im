//! 正在传的文件（施工 I-231，todolist 第 58 条）：`qq_send_file`、`tg_send_file` 交出去以后在后台传，大文件要很久（主人那次 26.8 MB
//! 传了 18 分钟）。她以为没传成又叫一次，原来照样再传一份，群文件里就有两份。同一个场所、同一个名字、同样大小的那一份还在传的，
//! 不再传，答 `still-uploading`（项目主人 11-12 定）。
//!
//! 传的任务带着一个 [`Guard`]：传完、没传成、任务没了（崩了、im 停了）都会放掉，不会一直记着「在传」。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// 正在传的那几份：键是场所、名字、大小。
#[derive(Debug, Default, Clone)]
pub(super) struct Uploading(Arc<Mutex<HashSet<String>>>);

/// 记着的一份：放掉（drop）时从 [`Uploading`] 里拿掉。
#[derive(Debug)]
pub(super) struct Guard {
    set: Arc<Mutex<HashSet<String>>>,
    key: String,
}

impl Uploading {
    /// 场所 `venue` 的 `name`（大小 `size`）开始传：交回守着它的 [`Guard`]；同样的一份还在传的是空的。
    pub(super) fn start(&self, venue: &str, name: &str, size: &str) -> Option<Guard> {
        let key = format!("{venue}\n{name}\n{size}");
        let mut set = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set.insert(key.clone()).then(|| Guard {
            set: Arc::clone(&self.0),
            key,
        })
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let mut set = self
            .set
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        set.remove(&self.key);
    }
}

#[cfg(test)]
mod tests;
