//! 桥用的配置（`onebot.md` 第一条「对外的样子」「怎么走」第 1、2 条，施工 O-20）：核心拉起桥时，握手的回应里交来这个包自己
//! 的配置（`config`：`im.` 开头的键，最终值，密钥是真值，没设、取不到的那一键不放），之后变了推 `extension.config`
//! （`{"keys": {键: 新值或 null}}`，只放变了的键；`extensions.md`「配置」）。桥不读系统配置、密钥文件，不依赖核心的 crate
//! （「施工时定的」第 6 条）。
//!
//! - 驱动的配置项（施工 I-3，设计 01 第六节）：名字由驱动报（OneBot 是 `listen`、`token`），在平台名那一组，键照 [`key`] 拼
//!   （`im.qq.listen`，施工 I-3 补）；握手交来的、
//!   推来的照 [`driver_config`]、[`merge_driver`] 挑出来，没有的、`null` 的照清单 `[settings]` 的默认值（[`Defaults`]，
//!   「施工时定的」第 38 条），交给驱动读，宿主不认它们是什么。
//! - 白名单成员 `im.whitelist`（施工 O-23；O-27 从 `im.trusted` 改名，旧键不认）：跟核心的那一头照 [`whitelist`] 读
//!   一份、推来的换（「群里怎么叫她」第 3 条）。别的键不认。
//!
//! 键是 `<包的编号>.<名字>`（`packages.md`「配置项」第 1 条）：名字在这里写一次，清单的默认值、握手和推送的键都照它。驱动、
//! 页面服务的键多一层组 `<包的编号>.<组>.<名字>`（2026-10-19 项目主人定；核心 9287d31d 起清单认 `[settings.<组>.<名字>]`，施工
//! I-3 补），照 [`key`] 拼。

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use miyu_client::manifest::{self, read};

use crate::PREFIX;
use crate::package::Package;

/// 包文件夹里的清单。
const MANIFEST: &str = "package.toml";

/// 白名单成员：清单里的名字（施工 O-23；O-27 从 `trusted` 改名，旧键不认）。
const WHITELIST: &str = "whitelist";

/// 群里回执要撤回的命令：清单里的名字（施工 O-47）。
const RECALL_RECEIPTS: &str = "recall_receipts";

/// 页面服务的配置项在哪一组（施工 I-3 补：核心的设置名能带一层组以后，键是 `im.web.port`）。
pub const SITE_GROUP: &str = "web";

/// 页面服务的配置项的名字（施工 I-5 上）：端口。
pub const SITE: &[&str] = &["port"];

/// 清单 `[settings]` 里的默认值：握手没交、推来 `null` 的照它（「施工时定的」第 38 条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    /// `im.recall_receipts` 的默认值（出厂 `clear`、`clear-memory`，施工 O-47）。
    pub recall_receipts: Vec<String>,
    /// 驱动的配置项的默认值：照平台名（组），组里照名字（施工 I-3：OneBot 的 `listen` 出厂 8301；施工 I-70 中上：几个驱动各一组）；
    /// 没写默认值的不放，一项都没写的那一组也在，是空的。
    pub drivers: BTreeMap<String, Map<String, Value>>,
    /// 页面服务的配置项（[`SITE`]）的默认值，照名字（施工 I-5 上：`port` 出厂 8302）。
    pub site: Map<String, Value>,
}

impl Defaults {
    /// 读包文件夹 `package` 里这个包自己的清单（`package.toml`，施工 S-6 上：出厂的、自己装的都照核心交来的那一层），取撤回回执的
    /// 命令（施工 O-47）、驱动的配置项（施工 I-3：每个驱动在平台名 `group` 那一组，名字是 `names`；施工 I-70 中上起几个驱动各一组
    /// `drivers`）、页面服务的配置项（施工 I-5 上）的默认值。交回的照组里的名字放。
    ///
    /// # Errors
    ///
    /// 清单不在、读不成、写错了，撤回回执的命令没声明、默认值不是字的列表：原话里说是哪个文件。
    pub fn load(package: &Package, drivers: &[(&str, &[&str])]) -> Result<Defaults, String> {
        let path = package.dir().join(MANIFEST);
        let bad = |why: String| format!("{} not readable: {why}", path.display());
        let text = std::fs::read_to_string(&path).map_err(|error| bad(error.to_string()))?;
        let manifest = read(&text).map_err(|problem| bad(problem.message))?;
        let recall_receipts = manifest
            .settings
            .iter()
            .find(|setting| setting.name == RECALL_RECEIPTS)
            .and_then(|setting| match &setting.default {
                Some(manifest::Value::List(items)) => items
                    .iter()
                    .map(|item| match item {
                        manifest::Value::Text(text) => Some(text.to_string()),
                        _ => None,
                    })
                    .collect(),
                _ => None,
            })
            .ok_or_else(|| bad(format!("no list default for {PREFIX}.{RECALL_RECEIPTS}")))?;
        // 清单里分组的项名字是 `<组>.<名字>`（核心 9287d31d）：照组挑，交回组里的名字。
        let defaults = |group: &str, names: &[&str]| {
            manifest
                .settings
                .iter()
                .filter_map(|setting| {
                    let name = setting.name.strip_prefix(group)?.strip_prefix('.')?;
                    names.contains(&name).then_some((name, setting))
                })
                .filter_map(|(name, setting)| {
                    Some((name.to_string(), json(setting.default.as_ref()?)?))
                })
                .collect()
        };
        Ok(Defaults {
            recall_receipts,
            drivers: drivers
                .iter()
                .map(|(group, names)| ((*group).to_string(), defaults(group, names)))
                .collect(),
            site: defaults(SITE_GROUP, SITE),
        })
    }
}

/// 清单里的一个默认值写成 JSON（同握手交来的值的样子）；密钥这类写不出的是空的。
fn json(value: &manifest::Value) -> Option<Value> {
    Some(match value {
        manifest::Value::Bool(on) => Value::from(*on),
        manifest::Value::Int(number) => Value::from(*number),
        manifest::Value::Text(text) => Value::from(text.to_string()),
        manifest::Value::List(items) => {
            Value::Array(items.iter().map(json).collect::<Option<_>>()?)
        }
        _ => return None,
    })
}

/// 组 `group` 里的配置项 `name` 的键：`im.<组>.<名字>`（施工 I-3 补：驱动的组是平台名，`im.qq.listen`；页面服务的是
/// [`SITE_GROUP`]，`im.web.port`）。
pub fn key(group: &str, name: &str) -> String {
    format!("{PREFIX}.{group}.{name}")
}

/// 握手交来的 `config` 里驱动的那几项（组 `group` 里的 `names`，施工 I-3）：照名字放，没交的、`null` 的照 `defaults`，默认值
/// 也没有的不放。`config` 不是对象的当什么都没交。
pub fn driver_config(
    config: &Value,
    (group, names): (&str, &[&str]),
    defaults: &Map<String, Value>,
) -> Map<String, Value> {
    pick(config, (group, names), defaults)
}

/// 同 [`driver_config`]（施工 I-5 上：驱动、页面服务各一组）。
pub(crate) fn pick(
    config: &Value,
    (group, names): (&str, &[&str]),
    defaults: &Map<String, Value>,
) -> Map<String, Value> {
    let mut driver = Map::new();
    for name in names {
        let value = config
            .get(key(group, name))
            .filter(|value| !value.is_null());
        if let Some(value) = value.or_else(|| defaults.get(*name)) {
            driver.insert((*name).to_string(), value.clone());
        }
    }
    driver
}

/// 推来的变化 `keys` 里驱动的那几项 `names` 合进 `driver`：带了的照新值，`null` 的照 `defaults`（没有默认值的拿掉）；没带的不动。
/// 交回变没变。
pub fn merge_driver(
    driver: &mut Map<String, Value>,
    keys: &Map<String, Value>,
    (group, names): (&str, &[&str]),
    defaults: &Map<String, Value>,
) -> bool {
    merge(driver, keys, (group, names), defaults)
}

/// 同 [`merge_driver`]（施工 I-5 上）。
pub(crate) fn merge(
    driver: &mut Map<String, Value>,
    keys: &Map<String, Value>,
    (group, names): (&str, &[&str]),
    defaults: &Map<String, Value>,
) -> bool {
    let before = driver.clone();
    for name in names {
        let Some(value) = keys.get(&key(group, name)) else {
            continue;
        };
        match Some(value)
            .filter(|value| !value.is_null())
            .or_else(|| defaults.get(*name))
        {
            Some(value) => driver.insert((*name).to_string(), value.clone()),
            None => driver.remove(*name),
        };
    }
    *driver != before
}

/// 配置项的键 `keys`（`im.sleep` 这种）照包文件夹 `package` 里自己的清单换成给人看的名字，照语言 `language` 挑（施工 O-41
/// 补二：`miyu im status` 说没认出来的配置项用名字）。清单读不成的、清单里没有这一项的照键。
pub fn labels(package: &Package, keys: &[String], language: &str) -> Vec<String> {
    let manifest = std::fs::read_to_string(package.dir().join(MANIFEST))
        .ok()
        .and_then(|text| read(&text).ok());
    keys.iter()
        .map(|key| {
            let name = key
                .strip_prefix(PREFIX)
                .and_then(|rest| rest.strip_prefix('.'));
            manifest
                .as_ref()
                .and_then(|manifest| {
                    manifest
                        .settings
                        .iter()
                        .find(|setting| Some(setting.name.as_str()) == name)
                })
                .and_then(|setting| {
                    // 照语言挑，没写的照 `en`、`zh`、`ja` 的先后（同核心挑包的名字）。
                    [language, "en", "zh", "ja"]
                        .iter()
                        .find_map(|language| setting.label.get(*language))
                })
                .map_or_else(|| key.clone(), String::clone)
        })
        .collect()
}

/// 白名单成员那一键（`im.whitelist`）：握手交来的 `config`、推来的 `keys` 里照它取（施工 O-23）。
pub fn whitelist_key() -> String {
    format!("{PREFIX}.{WHITELIST}")
}

/// 白名单成员的平台身份（`im.whitelist` 的值，施工 O-23，「群里怎么叫她」第 3 条）：字的列表照收；没有、`null`、不是列表的是
/// 空的（核心照清单查过类型，照说不会），列表里不是字的不要。
pub fn whitelist(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// 撤回回执的命令那一键（`im.recall_receipts`，施工 O-47）。
pub fn recall_receipts_key() -> String {
    format!("{PREFIX}.{RECALL_RECEIPTS}")
}

/// 撤回回执的命令（`im.recall_receipts` 的值，施工 O-47）：字的列表照收，去掉开头的 `/`（写了也认）；没有、`null`、不是
/// 列表的是空的（照 `defaults`，交了 `null` 就是回到默认），列表里不是字的不要。
pub fn recall_receipts(value: &Value, defaults: &[String]) -> Vec<String> {
    match value.as_array() {
        Some(list) => list
            .iter()
            .filter_map(Value::as_str)
            .map(|name| name.trim().trim_start_matches('/').to_string())
            .collect(),
        None => defaults.to_vec(),
    }
}
