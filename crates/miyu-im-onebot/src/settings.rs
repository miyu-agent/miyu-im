//! OneBot 驱动自己的配置项（施工 I-3，设计 01 第六节；原来在宿主的 `settings.rs`，施工 O-20）：宿主照 [`NAMES`] 把握手交来的、
//! 推来的这几键挑出来，没有的、`null` 的照清单的默认值补上，经 `Gate::config` 交过来，这里读成端口、令牌。
//!
//! - 端口 `listen`：0 到 65535 的整数照它（核心只收 1024 到 65535；0 是测试让系统挑）；不是的照清单的默认值（「施工时定的」
//!   第 38 条）。
//! - 令牌 `token`：是字的照它，去掉前后空白（[`Secret::new`]）；没有的、空的、不是字的就是没有：照样起来，NapCat 连进来一律
//!   401（第 1、2 条）。核心不交取不到的引用，分不出「没写引用」和「取不到」（「施工时定的」第 41 条）。

use serde_json::{Map, Value};

use miyu_client::settings::Secret;

/// 端口：清单里的名字。
const LISTEN: &str = "listen";

/// 令牌：清单里的名字。
const TOKEN: &str = "token";

/// 驱动的配置项：清单里的名字（`Driver::settings`）。识人术的四项（施工 I-10 上，`crate::identity`）、群管理的几项（施工 Q-1，`crate::admin`）也是驱动的配置项。
pub const NAMES: &[&str] = &[
    LISTEN,
    TOKEN,
    crate::identity::ON,
    crate::identity::ADMIN,
    crate::identity::CARD,
    crate::identity::PEOPLE,
    crate::admin::ON,
    crate::admin::MUTE,
    crate::admin::MUTE_MIN,
    crate::admin::MUTE_MAX,
    crate::admin::KICK,
    crate::admin::RECORDS,
    crate::admin::KEEP,
    crate::admin::SYNC_UNMUTE,
    crate::admin::LOG,
    crate::admin::SUMMARY,
];

/// 其中是密钥的（`Driver::secrets`，施工 I-6 中）。
pub const SECRETS: &[&str] = &[TOKEN];

/// OneBot 驱动用的两项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// NapCat 反连进来的端口，只听 `127.0.0.1`；`0` 是让系统挑一个（测试用）。
    pub port: u16,
    /// NapCat 连进来时要出示的访问令牌；没有的是空的。`Debug` 只印 `Secret(…)`。
    pub token: Option<Secret>,
}

impl Settings {
    /// 读宿主交来的 `config`（照名字放）：端口读不出的照 `default`（清单的默认值）。都读不出的是空的。
    pub fn read(config: &Map<String, Value>, default: Option<&Value>) -> Option<Settings> {
        Some(Settings {
            port: config
                .get(LISTEN)
                .and_then(port)
                .or_else(|| default.and_then(port))?,
            token: token(config),
        })
    }
}

/// `config` 里的令牌：是字的照它，去掉前后空白；别的、空的是没有。
pub fn token(config: &Map<String, Value>) -> Option<Secret> {
    config
        .get(TOKEN)
        .and_then(Value::as_str)
        .and_then(|token| Secret::new(token).ok())
}

/// 端口的默认值那一项的名字：`Gate::default` 照它取。
pub fn listen() -> &'static str {
    LISTEN
}

/// 一个端口：0 到 65535 的整数；别的是空的。
fn port(value: &Value) -> Option<u16> {
    value.as_u64().and_then(|port| u16::try_from(port).ok())
}
