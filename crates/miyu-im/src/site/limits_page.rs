//! 使用限制页（设计 08 第二节第 6 条，施工 I-136）：每人的三条线。
//!
//! - `limits.get` → `{default, people, usage}`：im 配置目录里的 `limits.toml`（默认的三条线、单独设的人），和最近 30 天用过的人
//!   （照 5 小时的用量从多到少，最多 [`MOST`] 个）：`{id, name, used, caps, full}`，`used`、`caps` 照 5 小时、7 天、30 天排，没写的线
//!   `null`；`name` 照聊天记录里他最近一次说话时的名字，没有的 `null`。用量页面服务直接开库读（同消息记录页）。
//!   金额（施工 I-205，项目主人 11-10：要看得到每人已花的钱）：每个人多一格 `costs`，照 5 小时、7 天、30 天排，每一样是
//!   `{currency, amount, unconverted: [{currency, amount}], unpriced}`（同数据统计页的金额）：账本里照「日子、币种」记的原币种的数交
//!   核心的 `usage.convert` 换成 `usage.currency`；没换成的照币种另给；`unpriced` 是没记录金额的请求数。核心没有这个方法、换不成的
//!   `amount` 是 `null`，原币种的数都放在 `unconverted` 里。
//! - `limits.set {default, people}`：整份替换 `limits.toml`（`crate::limits::People::from_page` 查，有一处不对整份不收）。桥隔几秒
//!   看一眼文件变没变，下一条消息照新的。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};

use super::plugins::core;
use super::{Person, Site};
use crate::limits::{FILE, People};
use crate::spent::Costs;
use crate::{records, spent, status_file};

/// 列几个人。
const MOST: usize = 50;

/// 拒的原因和原话。
type Refused = (String, String);

/// 设置文件。
fn file(site: &Site) -> PathBuf {
    crate::dirs::config(&site.root).join(FILE)
}

/// `limits.get`。
pub(super) async fn get(site: &Arc<Site>, person: &Person) -> Result<Value, Refused> {
    let people = People::read(&file(site));
    let state = status_file::path(&site.root);
    let (used, records) = (
        state.with_file_name(spent::FILE),
        state.with_file_name(records::FILE),
    );
    let read = tokio::task::spawn_blocking(move || {
        let top = spent::top(&used, records::now(), MOST)?;
        let ids: Vec<String> = top.iter().map(|(id, _)| id.clone()).collect();
        // 聊天记录读不出的照没有名字。
        let names = records::connection(&records)
            .and_then(|connection| records::read::names(&connection, &ids))
            .unwrap_or_default();
        // 金额（施工 I-205）：读不出的照没有，页面那一格不写。
        let costs = spent::costs(&used, &ids, records::now()).unwrap_or_default();
        Ok::<_, rusqlite::Error>((top, names, costs))
    })
    .await
    .map_err(|error| ("limits_failed".to_string(), error.to_string()))?;
    let (top, names, costs) =
        read.map_err(|error| ("limits_failed".to_string(), error.to_string()))?;
    let money = converted(person, &top, &costs).await;
    let usage: Vec<Value> = top
        .into_iter()
        .enumerate()
        .map(|(at, (id, used))| {
            let lines = people.lines_for(&id);
            json!({
                "id": id,
                "name": names.get(&id),
                "used": used,
                "caps": lines.caps(),
                "full": lines.reached(used),
                "costs": money.get(at),
            })
        })
        .collect();
    let listed: Vec<Value> = people.person.iter().map(|one| json!(one)).collect();
    Ok(json!({"default": people.default, "people": listed, "usage": usage}))
}

/// `limits.set`。
pub(super) fn set(site: &Site, params: &Value) -> Result<Value, Refused> {
    let people = People::from_page(&params["default"], &params["people"])
        .ok_or_else(|| ("bad_params".to_string(), String::new()))?;
    people
        .write(&file(site))
        .map_err(|error| ("write_failed".to_string(), error.to_string()))?;
    Ok(json!({"saved": true}))
}

/// `top` 里每个人三个时间段的金额，照 `top` 的先后：`costs` 是账本里照「日子、币种」加好的，交核心的 `usage.convert` 换（一个人
/// 一个时间段一组）。换不成的（核心没有这个方法、拒了）照没换成的给。
async fn converted(
    person: &Person,
    top: &[(String, [u64; 3])],
    costs: &HashMap<String, [Costs; 3]>,
) -> Vec<Value> {
    let none = <[Costs; 3]>::default();
    let each: Vec<&Costs> = top
        .iter()
        .flat_map(|(id, _)| costs.get(id).unwrap_or(&none).iter())
        .collect();
    let groups: Vec<Vec<Value>> = each
        .iter()
        .map(|costs| {
            let part = |(day, currency, amount): &(String, String, f64)| {
                json!({"day": day, "currency": currency, "amount": amount})
            };
            costs.parts.iter().map(part).collect()
        })
        .collect();
    let asked = groups.iter().any(|group| !group.is_empty());
    let answered = match asked {
        true => core(person, "usage.convert", json!({ "groups": groups }))
            .await
            .ok(),
        false => None,
    };
    let shown: Vec<Value> = each
        .iter()
        .enumerate()
        .map(|(at, costs)| shown(costs, answered.as_ref().map(|got| &got["converted"][at])))
        .collect();
    shown.chunks(3).map(|three| json!(three)).collect()
}

/// 一个时间段的金额写成页面要的（纯逻辑）：`converted` 是核心换好的那一项（没问、没换成的是空的、`null`）。
fn shown(costs: &Costs, converted: Option<&Value>) -> Value {
    let converted = converted.filter(|one| one.is_object());
    let rest: Vec<Value> = match converted {
        Some(one) => one["rest"].as_array().cloned().unwrap_or_default(),
        None => {
            let mut sums: std::collections::BTreeMap<&str, f64> = Default::default();
            for (_, currency, amount) in &costs.parts {
                *sums.entry(currency).or_default() += amount;
            }
            sums.into_iter()
                .map(|(currency, amount)| json!({"currency": currency, "amount": amount}))
                .collect()
        }
    };
    json!({
        "currency": converted.map(|one| &one["currency"]),
        "amount": converted.map(|one| &one["amount"]),
        "unconverted": rest,
        "unpriced": costs.unpriced,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::shown;
    use crate::spent::Costs;

    #[test]
    fn a_window_shows_the_converted_amount_or_the_original_currencies() {
        let costs = Costs {
            parts: vec![
                ("2026-11-10".into(), "USD".into(), 0.5),
                ("2026-11-11".into(), "USD".into(), 0.25),
                ("2026-11-11".into(), "EUR".into(), 1.0),
            ],
            unpriced: 2,
        };
        let converted =
            json!({"currency": "CNY", "amount": 5.4, "rest": [{"currency": "EUR", "amount": 1.0}]});
        assert_eq!(
            shown(&costs, Some(&converted)),
            json!({"currency": "CNY", "amount": 5.4, "unconverted": [{"currency": "EUR", "amount": 1.0}], "unpriced": 2})
        );
        // 核心没换（没有这个方法、一笔都没换成）：原币种的数照币种加起来给。
        assert_eq!(
            shown(&costs, Some(&json!(null))),
            json!({"currency": null, "amount": null,
                "unconverted": [{"currency": "EUR", "amount": 1.0}, {"currency": "USD", "amount": 0.75}], "unpriced": 2})
        );
        assert_eq!(
            shown(&Costs::default(), None),
            json!({"currency": null, "amount": null, "unconverted": [], "unpriced": 0})
        );
    }
}
