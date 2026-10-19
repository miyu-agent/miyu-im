//! 登录了的（施工 I-5 上「怎么走」第 5、6 条）：一份登录一个会话号（随机 32 字节写成十六进制），带着账号和照登录令牌连核心的
//! 那条人的连接。只在内存里：桥重启要重新进。最多留 [`HELD`] 份，多了拿掉最早的（几次跳过来各开一份，旧的没人登出）。
//! 会话号按常数时间比（同驱动比令牌）。

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// 最多留几份登录。
pub(super) const HELD: usize = 16;

/// 一份登录：账号、人的连接。
pub(crate) struct Login<P> {
    /// 核心认的账号（`login.redeem` 交回的）。
    pub(crate) account: String,
    /// 照这个人的登录令牌连着核心的连接（测试里换成别的）。
    pub(crate) person: P,
}

/// 登录了的，照先后。
pub(crate) struct Logins<P> {
    held: Mutex<Vec<(String, Arc<Login<P>>)>>,
}

impl<P> Default for Logins<P> {
    fn default() -> Logins<P> {
        Logins {
            held: Mutex::new(Vec::new()),
        }
    }
}

impl<P> Logins<P> {
    /// 记一份登录：账号 `account`、人的连接 `person`，交回会话号。取不到随机数的是空的（不开）。
    pub(crate) fn add(&self, account: String, person: P) -> Option<String> {
        let session = session()?;
        let mut held = self.lock();
        held.push((session.clone(), Arc::new(Login { account, person })));
        if held.len() > HELD {
            held.remove(0);
        }
        Some(session)
    }

    /// 会话号 `session` 的那份登录；没有的是空的。
    pub(crate) fn get(&self, session: &str) -> Option<Arc<Login<P>>> {
        let held = self.lock();
        let mut found = None;
        // 每一份都比完，不在比中的那一份停下。
        for (id, login) in held.iter() {
            if same(id.as_bytes(), session.as_bytes()) {
                found = Some(Arc::clone(login));
            }
        }
        found
    }

    /// 拿掉会话号 `session` 的那份登录（人的连接随最后一个拿着的放下一起断）：交回拿掉的那一份，没有的是空的。
    pub(crate) fn remove(&self, session: &str) -> Option<Arc<Login<P>>> {
        let mut held = self.lock();
        let mut removed = None;
        held.retain(|(id, login)| {
            let matched = same(id.as_bytes(), session.as_bytes());
            if matched {
                removed = Some(Arc::clone(login));
            }
            !matched
        });
        removed
    }

    /// 锁里不 `await`、不会崩；真崩了，里面的照样能用。
    fn lock(&self) -> MutexGuard<'_, Vec<(String, Arc<Login<P>>)>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// 新的会话号：32 个随机字节写成 64 位十六进制。取不到随机数的是空的。
fn session() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// 两串字节一样不一样，按常数时间比：长短不一样直接不一样；一样长的每个字节都比。
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .fold(0u8, |differs, (x, y)| differs | (x ^ y))
            == 0
}
