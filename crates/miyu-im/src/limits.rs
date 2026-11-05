//! 使用限制（设计 08）：最近 5 小时、7 天、30 天三条线（[`Lines`]），会话卡片上的三格（规则键 [`KEYS`]）和每人的设置（im 配置
//! 目录里的 [`FILE`]，[`People`]）都照它写。哪一条没写就是不限；碰到任何一条就算满。
//!
//! 每人的设置：一组默认的三条线对所有人生效，单独设了的人整组替换掉默认的（不逐条合并），三条都不写的就是不限：
//!
//! ```toml
//! [default]
//! limit_5h = 2000000
//!
//! [[person]]
//! id = "qq:10002"
//! limit_month = 50000000
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

use crate::TARGET;

#[cfg(test)]
mod tests;

/// 每人的设置的文件名（im 配置目录里）。
pub(crate) const FILE: &str = "limits.toml";

/// 三条线的规则键（卡片的 `rules` 里）和设置文件里的键，照 [`WINDOWS`] 的先后。
pub(crate) const KEYS: [&str; 3] = ["limit_5h", "limit_week", "limit_month"];

/// 三条线的窗口，毫秒：最近 5 小时、7 天、30 天，滚动的。
pub(crate) const WINDOWS: [i64; 3] = [5 * HOUR, 7 * DAY, 30 * DAY];

/// 一小时、一天，毫秒。
const HOUR: i64 = 60 * 60 * 1000;
const DAY: i64 = 24 * HOUR;

/// 设置文件隔多久看一眼变没变。
const RECHECK: Duration = Duration::from_secs(5);

/// 三条线：token 数，没写的不限。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Lines {
    /// 最近 5 小时。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) limit_5h: Option<u64>,
    /// 最近 7 天。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) limit_week: Option<u64>,
    /// 最近 30 天。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) limit_month: Option<u64>,
}

impl Lines {
    /// 照 [`WINDOWS`] 的先后排的三条线。
    pub(crate) fn caps(&self) -> [Option<u64>; 3] {
        [self.limit_5h, self.limit_week, self.limit_month]
    }

    /// 照 [`Lines::caps`] 的先后交回的三条线拼成的。
    pub(crate) fn from_caps([limit_5h, limit_week, limit_month]: [Option<u64>; 3]) -> Lines {
        Lines {
            limit_5h,
            limit_week,
            limit_month,
        }
    }

    /// 一条都没写：不限。
    pub(crate) fn is_empty(&self) -> bool {
        self.caps().iter().all(Option::is_none)
    }

    /// 用量 `used`（照 [`WINDOWS`] 的先后）碰到了哪一条没有。
    pub(crate) fn reached(&self, used: [u64; 3]) -> bool {
        self.caps()
            .into_iter()
            .zip(used)
            .any(|(cap, used)| cap.is_some_and(|cap| used >= cap))
    }
}

/// 单独设了的一个人。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Person {
    /// 平台上的人：`qq:10002`。
    pub(crate) id: String,
    /// 最近 5 小时。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limit_5h: Option<u64>,
    /// 最近 7 天。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limit_week: Option<u64>,
    /// 最近 30 天。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limit_month: Option<u64>,
}

impl Person {
    /// 这个人的三条线。
    pub(crate) fn lines(&self) -> Lines {
        Lines {
            limit_5h: self.limit_5h,
            limit_week: self.limit_week,
            limit_month: self.limit_month,
        }
    }
}

/// 每人的设置。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct People {
    /// 默认的三条线：没单独设的人照它。
    pub(crate) default: Lines,
    /// 单独设了的人。
    pub(crate) person: Vec<Person>,
}

impl People {
    /// `person` 的三条线：单独设了的照他的，没设的照默认的。
    pub(crate) fn lines_for(&self, person: &str) -> Lines {
        self.person
            .iter()
            .find(|one| one.id == person)
            .map_or(self.default, Person::lines)
    }

    /// 读 `path`：没有的是空的（谁都不限）；读不成、写错的记一行运行日志，也当空的。
    pub(crate) fn read(path: &Path) -> People {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return People::default(),
            Err(error) => {
                tracing::warn!(target: TARGET, path = %path.display(), error = %error, "limits not read");
                return People::default();
            }
        };
        toml::from_str(&text).unwrap_or_else(|error| {
            tracing::warn!(target: TARGET, path = %path.display(), error = %error, "limits not understood");
            People::default()
        })
    }
}

/// 一条线最多写多大：一万亿（同卡片上那三格）。
const CAP: u64 = 1_000_000_000_000;

/// 单独设的人最多几个：只防写错。
const MOST_PEOPLE: usize = 1000;

/// 写进设置文件开头的说明。
const HEADER: &str = "# 使用限制（接入通讯平台的网页上改的）：最近 5 小时、7 天、30 天最多用多少 token；没写的不限。\n";

impl People {
    /// 页面交来的（`limits.set` 的 `default`、`people`，施工 I-136）：每条线是 1 到一万亿的整数或者 `null`；人照平台上的人的写法
    /// （`qq:10002`），不重。有一处不对的整份不收。
    pub(crate) fn from_page(
        default: &serde_json::Value,
        people: &serde_json::Value,
    ) -> Option<People> {
        let lines = |value: &serde_json::Value| -> Option<[Option<u64>; 3]> {
            let object = match value {
                serde_json::Value::Null => return Some([None; 3]),
                value => value.as_object()?,
            };
            if object
                .keys()
                .any(|key| key != "id" && !KEYS.contains(&key.as_str()))
            {
                return None;
            }
            let mut caps = [None; 3];
            for (cap, key) in caps.iter_mut().zip(KEYS) {
                *cap = match object.get(key) {
                    None | Some(serde_json::Value::Null) => None,
                    Some(value) => Some(value.as_u64().filter(|cap| (1..=CAP).contains(cap))?),
                };
            }
            Some(caps)
        };
        let default = Lines::from_caps(lines(default)?);
        let listed = match people {
            serde_json::Value::Null => &Vec::new(),
            people => people.as_array()?,
        };
        if listed.len() > MOST_PEOPLE {
            return None;
        }
        let mut person: Vec<Person> = Vec::new();
        for one in listed {
            let id = one["id"].as_str()?.trim();
            let known = miyu_client::ids::ExternalId::parse(id).ok()?;
            miyu_chat::parse_person(&known)?;
            if person.iter().any(|seen| seen.id == id) {
                return None;
            }
            let [limit_5h, limit_week, limit_month] = lines(one)?;
            person.push(Person {
                id: id.to_string(),
                limit_5h,
                limit_week,
                limit_month,
            });
        }
        Some(People { default, person })
    }

    /// 写成设置文件的样子：开头一行说明，`[default]`，一个人一段 `[[person]]`。人的号已经照写法查过，没有要转义的字。
    pub(crate) fn to_toml(&self) -> String {
        let mut text = String::from(HEADER);
        let lines = |text: &mut String, caps: [Option<u64>; 3]| {
            for (key, cap) in KEYS.iter().zip(caps) {
                if let Some(cap) = cap {
                    text.push_str(&format!("{key} = {cap}\n"));
                }
            }
        };
        text.push_str("\n[default]\n");
        lines(&mut text, self.default.caps());
        for one in &self.person {
            text.push_str(&format!("\n[[person]]\nid = \"{}\"\n", one.id));
            lines(&mut text, one.lines().caps());
        }
        text
    }

    /// 写进 `path`：先写旁边的 `.partial` 再换名，桥那头不会读到写了一半的。
    pub(crate) fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let partial = path.with_extension("toml.partial");
        std::fs::write(&partial, self.to_toml())?;
        std::fs::rename(&partial, path)
    }
}

/// 盯着设置文件的一份：隔 [`RECHECK`] 看一眼改的时刻，变了重读。
#[derive(Debug)]
pub(crate) struct Watched {
    /// 设置文件。
    path: PathBuf,
    /// 读到的。
    people: People,
    /// 上一次看的时刻；还没看过的是空的。
    checked: Option<Instant>,
    /// 上一次读时文件改的时刻；没有文件的是空的。
    modified: Option<SystemTime>,
}

impl Watched {
    /// 盯着 `path`，用到时才读。
    pub(crate) fn new(path: PathBuf) -> Watched {
        Watched {
            path,
            people: People::default(),
            checked: None,
            modified: None,
        }
    }

    /// 这时的设置：离上一次看过了 [`RECHECK`] 的，看一眼文件变没变，变了重读。
    pub(crate) fn people(&mut self) -> &People {
        if self
            .checked
            .is_none_or(|checked| checked.elapsed() >= RECHECK)
        {
            self.checked = Some(Instant::now());
            let modified = std::fs::metadata(&self.path)
                .and_then(|meta| meta.modified())
                .ok();
            if modified != self.modified {
                self.modified = modified;
                self.people = People::read(&self.path);
            }
        }
        &self.people
    }
}
