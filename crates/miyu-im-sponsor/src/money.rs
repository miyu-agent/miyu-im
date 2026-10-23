//! 钱怎么写、怎么算（纯逻辑，时刻照传进来的时区）：金额写到两位小数、去掉多余的零；某个人的累计、明细；排行榜。累计只算换成
//! 了现在的默认币种的（`base` 对得上、`converted` 有数）：别的另数「未换算」，不猜（「算不准的钱不显示」）。

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::store::Entry;

/// 明细、排行榜最多列几条。
pub const SHOWN: usize = 10;

/// 金额写到两位小数，去掉末尾的零和小数点：`50`、`8.5`、`357.2`。
pub fn amount(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 四舍五入到两位小数（换算出来的金额冻结成这样）。
pub fn cents(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// 毫秒的时刻照时区 `zone` 写成日子：`2026-10-20`；读不成的是 `?`。
pub fn day(at: i64, zone: &TimeZone) -> String {
    Timestamp::from_millisecond(at)
        .map(|at| at.to_zoned(zone.clone()).strftime("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| "?".to_string())
}

/// 排行榜看多久。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// 这个月（照时区）。
    Month,
    /// 今年。
    Year,
    /// 全部。
    All,
}

impl Period {
    /// 参数的写法：`month`、`year`、`all`。
    pub fn read(text: &str) -> Option<Period> {
        match text {
            "month" => Some(Period::Month),
            "year" => Some(Period::Year),
            "all" => Some(Period::All),
            _ => None,
        }
    }

    /// 写进答的话里的：`this month` 这样。
    pub fn spoken(self) -> &'static str {
        match self {
            Period::Month => "this month",
            Period::Year => "this year",
            Period::All => "all time",
        }
    }

    /// 从哪一刻起算（毫秒）：此刻 `now` 照时区 `zone` 的这个月一号、今年一月一号零点；全部的没有。
    pub fn since(self, now: i64, zone: &TimeZone) -> Option<i64> {
        let now = Timestamp::from_millisecond(now)
            .ok()?
            .to_zoned(zone.clone());
        let start = match self {
            Period::Month => now.date().first_of_month(),
            Period::Year => now.date().first_of_year(),
            Period::All => return None,
        };
        let zoned = start.to_zoned(zone.clone()).ok()?;
        Some(zoned.timestamp().as_millisecond())
    }
}

/// 一个赞助人：有号的照号认，只有昵称的照名字认。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sponsor {
    /// 平台身份（`qq:123`）。
    Id(String),
    /// 只有昵称（10-24 补定第 1 条）。
    Name(String),
}

impl Sponsor {
    /// 这一笔是谁的；号、名字都没有的（照说不会）是空的。
    pub fn of(entry: &Entry) -> Option<Sponsor> {
        match (&entry.user, &entry.name) {
            (Some(user), _) => Some(Sponsor::Id(user.clone())),
            (None, Some(name)) => Some(Sponsor::Name(name.clone())),
            (None, None) => None,
        }
    }
}

/// 累计：换成 `base` 的合计、没换算的几笔。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Total {
    /// 换成默认币种的合计。
    pub sum: f64,
    /// 换算了的几笔。
    pub counted: usize,
    /// 没换算、或者换成了别的币种（默认币种改过）的几笔。
    pub pending: usize,
}

/// `entries` 照默认币种 `base` 合计。
pub fn total<'a>(entries: impl IntoIterator<Item = &'a Entry>, base: &str) -> Total {
    let mut total = Total::default();
    for entry in entries {
        match entry.converted.filter(|_| entry.base == base) {
            Some(converted) => {
                total.sum += converted;
                total.counted += 1;
            }
            None => total.pending += 1,
        }
    }
    total
}

/// 排行榜（纯逻辑）：`entries` 里 `since` 以后的，照赞助人合计（默认币种 `base`），多的在前，一样多的照先赞助的在前；最多
/// [`SHOWN`] 个。交回赞助人、最近一次记下的名字、合计。
pub fn ranking(
    entries: &[Entry],
    base: &str,
    since: Option<i64>,
) -> Vec<(Sponsor, Option<String>, f64)> {
    let mut rows: Vec<(Sponsor, Option<String>, f64, i64)> = Vec::new();
    for entry in entries
        .iter()
        .filter(|entry| since.is_none_or(|since| entry.at >= since))
    {
        let (Some(sponsor), Some(converted)) = (
            Sponsor::of(entry),
            entry.converted.filter(|_| entry.base == base),
        ) else {
            continue;
        };
        match rows.iter_mut().find(|row| row.0 == sponsor) {
            Some(row) => {
                row.2 += converted;
                if entry.name.is_some() {
                    row.1.clone_from(&entry.name);
                }
            }
            None => rows.push((sponsor, entry.name.clone(), converted, entry.at)),
        }
    }
    rows.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.3.cmp(&b.3)));
    rows.truncate(SHOWN);
    rows.into_iter()
        .map(|(sponsor, name, sum, _)| (sponsor, name, sum))
        .collect()
}

#[cfg(test)]
mod tests;
