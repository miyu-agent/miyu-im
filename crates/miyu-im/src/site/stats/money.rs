//! 金额（施工 I-104，项目主人 11-02：数据统计要有金额，没价钱的请求写「另有几次请求没记录金额」）：照核心 `usage.query` 每一行的
//! `converted`（照每一笔那天的汇率换成 `usage.currency`，核心 UC-1）、`amounts`、`unpriced` 加起来。
//!
//! - 换算好的加进一个数；没换成的（核心的 `rest`，一笔都没换成的整行照 `amounts`）照币种另记，页面另写一行。
//! - 一笔都没换成过的没有币种，金额是空的（页面写「—」）。

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// 几行用量的金额。
#[derive(Default)]
pub(super) struct Money {
    /// 换成的币种（`usage.currency`）；一笔都没换成过的是空的。
    currency: Option<String>,
    /// 换算好的合计。
    amount: f64,
    /// 没换成的：币种 → 金额。
    rest: BTreeMap<String, f64>,
    /// 没价钱的请求有几次。
    unpriced: u64,
}

impl Money {
    /// 加上一行 `row`。
    pub(super) fn add(&mut self, row: &Value) {
        self.unpriced += row["unpriced"].as_u64().unwrap_or_default();
        let converted = &row["converted"];
        let rest = if converted.is_object() {
            if let Some(currency) = converted["currency"].as_str() {
                self.currency.get_or_insert_with(|| currency.to_string());
            }
            self.amount += converted["amount"].as_f64().unwrap_or_default();
            &converted["rest"]
        } else {
            &row["amounts"]
        };
        for one in rest.as_array().into_iter().flatten() {
            if let Some(currency) = one["currency"].as_str() {
                *self.rest.entry(currency.to_string()).or_default() +=
                    one["amount"].as_f64().unwrap_or_default();
            }
        }
    }

    /// 几行加起来。
    pub(super) fn of(rows: &[&Value]) -> Money {
        let mut money = Money::default();
        for row in rows {
            money.add(row);
        }
        money
    }

    /// 换算好的合计；一笔都没换成过的是空的。
    pub(super) fn amount(&self) -> Option<f64> {
        self.currency.as_ref().map(|_| self.amount)
    }

    /// 页面上的样子：`{currency, amount, unconverted: [{currency, amount}], unpriced}`。
    pub(super) fn shown(&self) -> Value {
        let unconverted: Vec<Value> = self
            .rest
            .iter()
            .filter(|(_, amount)| **amount > 0.0)
            .map(|(currency, amount)| json!({"currency": currency, "amount": amount}))
            .collect();
        json!({"currency": self.currency, "amount": self.amount(), "unconverted": unconverted, "unpriced": self.unpriced})
    }
}

#[cfg(test)]
mod tests;
