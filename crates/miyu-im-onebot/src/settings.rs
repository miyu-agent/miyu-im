//! OneBot 驱动自己的配置项（施工 I-3，设计 01 第六节；原来在宿主的 `settings.rs`，施工 O-20）：宿主照 [`NAMES`] 把握手交来的、
//! 推来的这几键挑出来，没有的、`null` 的照清单的默认值补上，经 `Gate::config` 交过来，这里读成地址、端口、令牌。
//!
//! - 地址 `host`（施工 I-27，2026-10-22 项目主人定：照旧 Miyu 谁都能连，出厂 `0.0.0.0`；只给本机的改成 `127.0.0.1`）：写得出一个
//!   IP 地址的照它（去掉前后空白）；不是的照清单的默认值，默认值也读不出的是 `0.0.0.0`。外人连进来靠令牌挡。
//!
//! - 端口 `listen`：0 到 65535 的整数照它（核心只收 1024 到 65535；0 是测试让系统挑）；不是的照清单的默认值（「施工时定的」
//!   第 38 条）。
//! - 令牌 `token`：是字的照它，去掉前后空白（[`Secret::new`]）；没有的、空的、不是字的就是没有：照样起来，NapCat 连进来一律
//!   401（第 1、2 条）。核心不交取不到的引用，分不出「没写引用」和「取不到」（「施工时定的」第 41 条）。
//!
//! 连接表（施工 I-81 下，设计 07 第 7.7 节）：一个连接一个端口、一个地址、一个令牌，存成三个藏起来的列表 `hosts`、`ports`、`tokens`，
//! 第几项是第几个连接，只有后台页写。照 `ports` 那一列算有几个连接：`hosts`、`tokens` 少了的那几个照默认地址、没有令牌，多的不认；
//! 端口重了的只认第一个。`ports` 没写过的，上面三项当第一个连接读（[`Settings::all`]）。

use std::net::{IpAddr, Ipv4Addr};

use serde_json::{Map, Value};

use miyu_client::settings::Secret;

/// 地址：清单里的名字（施工 I-27）。
const HOST: &str = "host";

/// 端口：清单里的名字。
const LISTEN: &str = "listen";

/// 令牌：清单里的名字。
const TOKEN: &str = "token";

/// 连接表的地址那一列（施工 I-81 下）。
const HOSTS: &str = "hosts";

/// 连接表的端口那一列。
const PORTS: &str = "ports";

/// 连接表的令牌那一列。
const TOKENS: &str = "tokens";

/// 驱动的配置项：清单里的名字（`Driver::settings`）。识人术的四项（施工 I-10 上，`crate::identity`）、群管理的几项（施工 Q-1，`crate::admin`）、查询信息的总开关（施工 I-44、I-91，`crate::lookups`）也是驱动的配置项。
pub const NAMES: &[&str] = &[
    HOST,
    LISTEN,
    TOKEN,
    HOSTS,
    PORTS,
    TOKENS,
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
    crate::lookups::ON,
];

/// 其中是密钥的（`Driver::secrets`，施工 I-6 中）。
pub const SECRETS: &[&str] = &[TOKEN];

/// 一个连接的三项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 听哪个地址（施工 I-27）：出厂 `0.0.0.0` 谁都能连，`127.0.0.1` 只给本机。
    pub host: IpAddr,
    /// NapCat 反连进来的端口；`0` 是让系统挑一个（测试用）。
    pub port: u16,
    /// NapCat 连进来时要出示的访问令牌；没有的是空的。`Debug` 只印 `Secret(…)`。
    pub token: Option<Secret>,
}

impl Settings {
    /// 读宿主交来的 `config`（照名字放）：地址、端口读不出的照 `default`（照名字给清单的默认值）。端口都读不出的是空的。
    pub fn read(
        config: &Map<String, Value>,
        default: impl Fn(&str) -> Option<Value>,
    ) -> Option<Settings> {
        Some(Settings {
            host: config
                .get(HOST)
                .and_then(host)
                .or_else(|| default(HOST).as_ref().and_then(host))
                .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            port: config
                .get(LISTEN)
                .and_then(port)
                .or_else(|| default(LISTEN).as_ref().and_then(port))?,
            token: config.get(TOKEN).and_then(secret),
        })
    }

    /// 连接表（模块的说明）：`ports` 写了的照三个列表，一个都没有的是空的；没写的照上面三项当一个连接（[`Settings::read`]）。
    pub fn all(
        config: &Map<String, Value>,
        default: impl Fn(&str) -> Option<Value>,
    ) -> Vec<Settings> {
        let Some(ports) = config.get(PORTS).and_then(Value::as_array) else {
            return Settings::read(config, default).into_iter().collect();
        };
        let column = |name| {
            config
                .get(name)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        let (hosts, tokens) = (column(HOSTS), column(TOKENS));
        let fallback = default(HOST)
            .as_ref()
            .and_then(host)
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let mut all: Vec<Settings> = Vec::new();
        for (index, wanted) in ports.iter().enumerate() {
            let Some(port) = port(wanted) else {
                continue;
            };
            if all.iter().any(|one| one.port == port && port != 0) {
                continue;
            }
            all.push(Settings {
                host: hosts.get(index).and_then(host).unwrap_or(fallback),
                port,
                token: tokens.get(index).and_then(secret),
            });
        }
        all
    }
}

/// 一个令牌：是字的照它，去掉前后空白；别的、空的是没有。
fn secret(value: &Value) -> Option<Secret> {
    value.as_str().and_then(|token| Secret::new(token).ok())
}

/// 一个地址：写得出 IP 地址的字（去掉前后空白）；别的是空的。
fn host(value: &Value) -> Option<IpAddr> {
    value.as_str().and_then(|host| host.trim().parse().ok())
}

/// 一个端口：0 到 65535 的整数；别的是空的。
fn port(value: &Value) -> Option<u16> {
    value.as_u64().and_then(|port| u16::try_from(port).ok())
}
