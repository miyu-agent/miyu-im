//! 统计页（施工 I-14，设计 01 第八节第 4 条、第一节第 25 条：看统计主要是看 token，放最上面）：`stats {period, platform?}` →
//! `{"token", "messages", "running"}`。`period` 是 `today`、`7d`、`30d`（不认的照 `today`），每一样都和前一段同样长的比。
//!
//! - **token**：照核心的用量账本 `usage.query`（这份登录的人的连接），只算这个平台的场所会话（`venue` 以 `<平台>:` 开头的）：合计、
//!   缓存命中率（读缓存 ÷ 输入）、每次回复平均（合计 ÷ 回复的段数）、请求次数，每天的四样（读缓存、没命中、写缓存、输出），按用途、
//!   按场所（最多 10 个）、按模型。金额算不准不给（项目主人的规矩）。
//! - **messages**：照宿主自己记的（`crate::counters`）：收到、回复、只记录、没发出去，每天的，最热闹的场所（最多 10 个），判断的原因，
//!   按小时收到的。
//! - **running**：桥起来多久、进程的内存和 CPU（照 `ps`，Linux、macOS 有；别的系统是空的）、插件开着几个、出错几个。

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::plugins::core;
use super::{Person, Site};
use crate::counters::Daily;

/// 按场所、按会话最多列几个。
const TOP: usize = 10;

/// 一段：几天。
fn days_of(period: &str) -> i64 {
    match period {
        "7d" => 7,
        "30d" => 30,
        _ => 1,
    }
}

/// 一行用量的四样（缺的当 0）。
fn usage(row: &Value) -> [u64; 4] {
    let take = |key: &str| row["usage"][key].as_u64().unwrap_or_default();
    [
        take("cache_read"),
        take("uncached"),
        take("cache_write"),
        take("output"),
    ]
}

/// 照 `platform` 挑出场所会话的那几行（纯逻辑）。
pub(super) fn ours<'a>(rows: &'a Value, platform: &str) -> Vec<&'a Value> {
    let prefix = format!("{platform}:");
    rows.as_array()
        .into_iter()
        .flatten()
        .filter(|row| {
            row["venue"]
                .as_str()
                .is_some_and(|venue| venue.starts_with(&prefix))
        })
        .collect()
}

/// 用量的那一段（纯逻辑）：这一段的行 `rows`（按场所、天、用途、模型分的）、前一段的 `before`、这一段回复了几段 `replied`。
pub(super) fn token(rows: &[&Value], before: &[&Value], replied: u64) -> Value {
    let sum = |rows: &[&Value]| -> ([u64; 4], u64) {
        rows.iter().fold(([0; 4], 0), |(mut total, requests), row| {
            for (slot, value) in total.iter_mut().zip(usage(row)) {
                *slot += value;
            }
            (
                total,
                requests + row["requests"].as_u64().unwrap_or_default(),
            )
        })
    };
    let (total, requests) = sum(rows);
    let (prev, prev_requests) = sum(before);
    let all = |parts: [u64; 4]| parts.iter().sum::<u64>();
    #[allow(clippy::cast_precision_loss)]
    let rate = |parts: [u64; 4]| {
        let input = parts[0] + parts[1] + parts[2];
        (input > 0).then(|| parts[0] as f64 / input as f64)
    };
    let mut by: [BTreeMap<String, ([u64; 4], u64)>; 4] = Default::default();
    for row in rows {
        for (map, key) in by.iter_mut().zip(["day", "purpose", "venue", "model"]) {
            let name = row[key].as_str().unwrap_or("").to_string();
            let entry = map.entry(name).or_default();
            for (slot, value) in entry.0.iter_mut().zip(usage(row)) {
                *slot += value;
            }
            entry.1 += row["requests"].as_u64().unwrap_or_default();
        }
    }
    let [days, purposes, venues, models] = by;
    let listed = |map: BTreeMap<String, ([u64; 4], u64)>, top: Option<usize>| -> Vec<Value> {
        let mut list: Vec<(String, ([u64; 4], u64))> = map.into_iter().collect();
        list.sort_by(|a, b| all(b.1.0).cmp(&all(a.1.0)).then(a.0.cmp(&b.0)));
        list.into_iter()
            .take(top.unwrap_or(usize::MAX))
            .map(|(name, (parts, requests))| json!({"name": name, "tokens": all(parts), "requests": requests}))
            .collect()
    };
    let days: Vec<Value> = days
        .into_iter()
        .map(|(day, (parts, _))| json!({"day": day, "parts": parts}))
        .collect();
    json!({
        "total": all(total), "prev_total": all(prev),
        "hit_rate": rate(total), "prev_hit_rate": rate(prev),
        "per_reply": (replied > 0).then(|| all(total) / replied),
        "requests": requests, "prev_requests": prev_requests,
        "parts": total,
        "days": days,
        "purposes": listed(purposes, None),
        "venues": listed(venues, Some(TOP)),
        "models": listed(models, None),
    })
}

/// 消息的那一段（纯逻辑）：这一段的几天 `days`、前一段的 `before`。
pub(super) fn messages(days: &[(String, Daily)], before: &[(String, Daily)]) -> Value {
    let total = |days: &[(String, Daily)]| {
        days.iter().fold([0u64; 4], |mut sum, (_, daily)| {
            let day = daily.total;
            for (slot, value) in
                sum.iter_mut()
                    .zip([day.received, day.replied, day.recorded, day.undelivered])
            {
                *slot += value;
            }
            sum
        })
    };
    let (now, prev) = (total(days), total(before));
    let mut venues: BTreeMap<String, u64> = BTreeMap::new();
    let mut reasons: BTreeMap<String, u64> = BTreeMap::new();
    let mut hours = [0u64; 24];
    for (_, daily) in days {
        for (venue, day) in &daily.venues {
            *venues.entry(venue.clone()).or_default() += day.received;
        }
        for (reason, count) in &daily.reasons {
            *reasons.entry(reason.clone()).or_default() += count;
        }
        for (slot, count) in hours.iter_mut().zip(&daily.hours) {
            *slot += count;
        }
    }
    let mut busiest: Vec<(String, u64)> =
        venues.into_iter().filter(|(_, count)| *count > 0).collect();
    busiest.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    busiest.truncate(TOP);
    let field = |sum: [u64; 4]| json!({"received": sum[0], "replied": sum[1], "recorded": sum[2], "undelivered": sum[3]});
    json!({
        "now": field(now),
        "prev": field(prev),
        "days": days.iter().map(|(date, daily)| json!({"day": date, "received": daily.total.received, "replied": daily.total.replied})).collect::<Vec<_>>(),
        "venues": busiest.into_iter().map(|(venue, count)| json!({"name": venue, "received": count})).collect::<Vec<_>>(),
        "reasons": reasons,
        "hours": hours,
    })
}

/// `stats`（模块的说明）。
pub(super) async fn stats(
    site: &Site,
    person: &Person,
    params: &Value,
) -> Result<Value, (String, String)> {
    let period = days_of(params["period"].as_str().unwrap_or("today"));
    let platform = site.running.platform.name();
    let today = jiff::Zoned::now().date();
    let dates = |from: i64, count: i64| -> Vec<String> {
        (0..count)
            .filter_map(|back| {
                today
                    .checked_sub(jiff::Span::new().days(from + count - 1 - back))
                    .ok()
            })
            .map(|date| date.to_string())
            .collect()
    };
    let (now_days, prev_days) = (dates(0, period), dates(period, period));
    let start = |days_back: i64| -> Option<String> {
        let date = today.checked_sub(jiff::Span::new().days(days_back)).ok()?;
        let zoned = date.to_zoned(jiff::tz::TimeZone::system()).ok()?;
        let millis = zoned.timestamp().as_millisecond();
        Some(miyu_client::time::Timestamp::from_unix_millis(millis)?.to_string())
    };
    let (from, prev_from) = (start(period - 1), start(2 * period - 1));
    let now = miyu_client::time::Timestamp::from_unix_millis(
        i64::try_from(unix_millis(SystemTime::now())).unwrap_or(i64::MAX),
    )
    .map(|at| at.to_string());
    let offset = offset();
    let mut asked = json!({"group": ["venue", "day", "purpose", "model"], "offset": offset, "from": from, "until": now});
    let rows = core(person, "usage.query", asked.take()).await?;
    let before = core(
        person,
        "usage.query",
        json!({"group": ["venue"], "offset": offset, "from": prev_from, "until": from}),
    )
    .await?;
    let counted = site.counters.days(&now_days);
    let counted_before = site.counters.days(&prev_days);
    let replied: u64 = counted.iter().map(|(_, daily)| daily.total.replied).sum();
    let token = token(
        &ours(&rows["rows"], platform),
        &ours(&before["rows"], platform),
        replied,
    );
    let messages = messages(&counted, &counted_before);
    let running = running(site, person).await;
    Ok(json!({"token": token, "messages": messages, "running": running}))
}

/// 本机此刻的时区偏移：`+08:00` 这样。
fn offset() -> String {
    let seconds = jiff::Zoned::now().offset().seconds();
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

/// 毫秒的 Unix 时间。
fn unix_millis(at: SystemTime) -> u128 {
    at.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// 运行的那一段。
async fn running(site: &Site, person: &Person) -> Value {
    let uptime = SystemTime::now()
        .duration_since(site.running.since)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let measured = tokio::task::spawn_blocking(measure).await.ok().flatten();
    let (rss, cpu) = measured.map_or((Value::Null, Value::Null), |(rss, cpu)| {
        (json!(rss), json!(cpu))
    });
    let plugins = super::plugins::list(site, person).await.ok();
    let listed: Vec<Value> = plugins
        .as_ref()
        .and_then(|plugins| plugins["plugins"].as_array().cloned())
        .unwrap_or_default();
    let on = listed
        .iter()
        .filter(|plugin| plugin["enabled"] == true)
        .count();
    let failed = listed
        .iter()
        .filter(|plugin| plugin["state"] == "failed")
        .count();
    let mut running = Map::new();
    running.insert("uptime".into(), json!(uptime));
    running.insert("rss".into(), rss);
    running.insert("cpu".into(), cpu);
    running.insert(
        "plugins".into(),
        json!({"total": listed.len(), "on": on, "failed": failed}),
    );
    Value::Object(running)
}

/// 照 `ps` 量这个进程：（常驻内存的字节，CPU 百分比）。没有 `ps` 的系统（Windows）、量不出的是空的。
fn measure() -> Option<(u64, f64)> {
    if cfg!(windows) {
        return None;
    }
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-o", "%cpu=", "-p", &pid])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut parts = text.split_whitespace();
    let rss: u64 = parts.next()?.parse().ok()?;
    let cpu: f64 = parts.next()?.replace(',', ".").parse().ok()?;
    Some((rss * 1024, cpu))
}

#[cfg(test)]
mod tests;
