//! 令牌表（施工 I-183 下中，设计 07 第八节）：驱动报了令牌表的（Telegram，`Driver::token_table`）一个连接一个令牌，连接照令牌查出的
//! 机器人号认，不开端口。后台页新建连接、连接卡齿轮里改、删、重新检查，在这份登录的人的连接上替人读、写配置（人的权限）。
//!
//! - `connection.add {platform, token, <参数>…}` → `{"saved": true, "bot", "name", "username", "privacy"}`：先请驱动照令牌查
//!   （`Driver::probe`），查成了再存：令牌 `secret.set` 存成密钥（`im-<平台>-token-<号>-<时刻>`），令牌那一列加引用、另外几列加页面交的
//!   （照 `TokenTable::columns` 的参数名，没交的、空的写那一列的默认：核心的一列字不收空字），一次 `config.set`。这个机器人已经在表里的：`bot_taken`。
//! - `connection.edit {platform, bot | index, values: {token?, <参数>?…}}` → `{"saved": true}`：改号是 `bot` 的那一个（还没认出号的照
//!   第几个 `index`）。换令牌的先查，认出过号的查出别的机器人的：`other_bot`；空的令牌不改。
//! - `connection.remove {platform, bot | index}` → `{"saved": true}`：删那一个。
//! - `connection.check {platform, bot | index}` → 查到的（同 `add` 交回的那几格）：请驱动照表里那一个现在的令牌再查一次（隐私模式改了以后）。
//!
//! 号是第几个照驱动报的状态（`connections` 里每项的 `index`、`bot`）认；不认得的号：`bad_params`。写成了把不再引用的旧密钥删掉
//! （同端口表，施工 I-118）。查不成的照驱动的原因码交回（`bad_token`、`unreachable`、`bad_proxy`……）。

use serde_json::{Map, Value, json};

use super::plugin_settings::{now_millis, secret_name};
use super::{Person, Site};
use crate::driver::TokenTable;
use crate::settings::key;

/// 写错了。
fn bad() -> (String, String) {
    ("bad_params".to_string(), String::new())
}

/// 驱动交回的原因码。
fn refused(why: String) -> (String, String) {
    (why, String::new())
}

/// 在这份登录的人的连接上调核心的 `method`。
async fn call(person: &Person, method: &str, params: Value) -> Result<Value, (String, String)> {
    person
        .lock()
        .await
        .call("config", method, params)
        .await
        .map_err(|why| (why.reason, why.message))
}

/// 这一刻的表：令牌那一列（引用原样）、另外几列（字），一列一个 `Vec`。
struct Rows {
    tokens: Vec<Value>,
    columns: Vec<Vec<Value>>,
}

impl Rows {
    /// 几个连接：照令牌那一列。
    fn len(&self) -> usize {
        self.tokens.len()
    }

    /// 写回的几项（一次 `config.set`）。
    fn changes(&self, platform: &str, table: &TokenTable) -> Vec<Value> {
        let mut changes = vec![json!({"key": key(platform, table.tokens), "value": self.tokens})];
        for ((name, _, empty), column) in table.columns.iter().zip(&self.columns) {
            let mut column = column.clone();
            column.resize(self.len(), json!(empty));
            changes.push(json!({"key": key(platform, name), "value": column}));
        }
        changes
    }
}

/// 读这一刻的表。
async fn read(
    person: &Person,
    platform: &str,
    table: &TokenTable,
) -> Result<Rows, (String, String)> {
    let mut keys = vec![key(platform, table.tokens)];
    keys.extend(table.columns.iter().map(|(name, _, _)| key(platform, name)));
    let got = call(person, "config.get", json!({"keys": keys})).await?;
    let column = |name: &str| -> Vec<Value> {
        got["items"][key(platform, name)]["value"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    };
    Ok(Rows {
        tokens: column(table.tokens),
        columns: table
            .columns
            .iter()
            .map(|(name, _, _)| column(name))
            .collect(),
    })
}

/// 页面交的是哪个连接：写了 `index` 的照它（还没认出号的连接，令牌一开始就失效、代理不对这类），不然照号 `bot` 在驱动报的状态里找。
/// 交回（第几个，号；没认出号的号是空的）。
fn which(status: &Map<String, Value>, params: &Value) -> Option<(usize, Option<String>)> {
    if let Some(index) = params["index"].as_u64() {
        return Some((usize::try_from(index).ok()?, None));
    }
    let bot = params["bot"].as_str()?;
    Some((index_of(status, bot)?, Some(bot.to_string())))
}

/// 号是 `bot` 的那个连接是表里第几个：照驱动报的状态。
fn index_of(status: &Map<String, Value>, bot: &str) -> Option<usize> {
    status
        .get("connections")?
        .as_array()?
        .iter()
        .find(|one| one["bot"].as_str() == Some(bot))
        .and_then(|one| usize::try_from(one["index"].as_u64()?).ok())
}

/// 一段令牌先存成密钥，交回引用。
async fn stored(
    person: &Person,
    platform: &str,
    bot: &str,
    text: &str,
) -> Result<Value, (String, String)> {
    let name = secret_name(
        &format!("{}-{platform}-token-{bot}", crate::PREFIX),
        now_millis(),
    );
    call(
        person,
        "secret.set",
        json!({"name": name, "value": text.trim()}),
    )
    .await?;
    Ok(json!({"secret": name}))
}

/// 写回表；写成了删掉不再引用的旧密钥 `retired`（删不成的记一行，照样算存成了）。
async fn write(
    person: &Person,
    (platform, table): (&str, &TokenTable),
    rows: &Rows,
    retired: Option<&Value>,
) -> Result<(), (String, String)> {
    let changes = rows.changes(platform, table);
    call(
        person,
        "config.set",
        json!({"layer": "system", "changes": changes}),
    )
    .await?;
    tracing::info!(target: crate::TARGET, platform, connections = rows.len(), "token table saved");
    if let Some(name) = retired.and_then(|token| token["secret"].as_str())
        && !rows
            .tokens
            .iter()
            .any(|token| token["secret"].as_str() == Some(name))
        && let Err((reason, _)) = call(person, "secret.delete", json!({"name": name})).await
    {
        tracing::warn!(target: crate::TARGET, secret = %name, reason, "old connection token not deleted");
    }
    Ok(())
}

/// 请驱动照 `params` 查。
async fn probe(site: &Site, params: &Value, ask: Value) -> Result<Value, (String, String)> {
    let (station, driver) = site.pick(params)?;
    let config = site
        .running
        .current
        .driver(station.platform)
        .borrow()
        .clone();
    driver.probe(ask, config).await.map_err(refused)
}

/// `connection.add`（模块的说明）。
pub(super) async fn add(
    site: &Site,
    person: &Person,
    params: &Value,
    table: TokenTable,
) -> Result<Value, (String, String)> {
    let token = params["token"]
        .as_str()
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(bad)?;
    let mut ask = json!({"token": token});
    for (_, param, empty) in table.columns {
        let given = params[*param]
            .as_str()
            .map(str::trim)
            .filter(|given| !given.is_empty());
        ask[*param] = json!(given.unwrap_or(empty));
    }
    let mut probed = probe(site, params, ask.clone()).await?;
    let bot = probed["bot"].as_str().ok_or_else(bad)?.to_string();
    let (station, _) = site.pick(params)?;
    let platform = station.platform.name().to_string();
    if index_of(&station.status(), &bot).is_some() {
        return Err(("bot_taken".to_string(), String::new()));
    }
    let _turn = site.table.lock().await;
    let mut rows = read(person, &platform, &table).await?;
    rows.tokens
        .push(stored(person, &platform, &bot, token).await?);
    for ((_, param, empty), column) in table.columns.iter().zip(rows.columns.iter_mut()) {
        column.resize(rows.tokens.len() - 1, json!(empty));
        column.push(ask[*param].clone());
    }
    write(person, (&platform, &table), &rows, None).await?;
    probed["saved"] = json!(true);
    Ok(probed)
}

/// `connection.edit`（模块的说明）。
pub(super) async fn edit(
    site: &Site,
    person: &Person,
    params: &Value,
    table: TokenTable,
) -> Result<Value, (String, String)> {
    let (station, _) = site.pick(params)?;
    let platform = station.platform.name().to_string();
    let (index, bot) = which(&station.status(), params).ok_or_else(bad)?;
    let values = &params["values"];
    let _turn = site.table.lock().await;
    let mut rows = read(person, &platform, &table).await?;
    if index >= rows.len() {
        return Err(bad());
    }
    let mut retired = None;
    if let Some(token) = values["token"]
        .as_str()
        .filter(|token| !token.trim().is_empty())
    {
        let mut ask = json!({"token": token});
        for ((_, param, empty), column) in table.columns.iter().zip(&rows.columns) {
            ask[*param] = values
                .get(*param)
                .cloned()
                .or_else(|| column.get(index).cloned())
                .unwrap_or(json!(empty));
        }
        let probed = probe(site, params, ask).await?;
        let found = probed["bot"].as_str().ok_or_else(bad)?;
        // 认出过号的只许换成同一个机器人的令牌；还没认出号的（令牌原来就不对）换哪个都行。
        if bot.as_deref().is_some_and(|bot| bot != found) {
            return Err(("other_bot".to_string(), String::new()));
        }
        let fresh = stored(person, &platform, found, token).await?;
        retired = Some(std::mem::replace(&mut rows.tokens[index], fresh));
    }
    for ((_, param, empty), column) in table.columns.iter().zip(rows.columns.iter_mut()) {
        if let Some(value) = values[*param].as_str() {
            column.resize(column.len().max(index + 1), json!(empty));
            let value = value.trim();
            column[index] = json!(if value.is_empty() { empty } else { value });
        }
    }
    write(person, (&platform, &table), &rows, retired.as_ref()).await?;
    Ok(json!({"saved": true}))
}

/// `connection.remove`（模块的说明）。
pub(super) async fn remove(
    site: &Site,
    person: &Person,
    params: &Value,
    table: TokenTable,
) -> Result<Value, (String, String)> {
    let (station, _) = site.pick(params)?;
    let platform = station.platform.name().to_string();
    let (index, _) = which(&station.status(), params).ok_or_else(bad)?;
    let _turn = site.table.lock().await;
    let mut rows = read(person, &platform, &table).await?;
    if index >= rows.len() {
        return Err(bad());
    }
    let retired = rows.tokens.remove(index);
    for column in &mut rows.columns {
        if index < column.len() {
            column.remove(index);
        }
    }
    write(person, (&platform, &table), &rows, Some(&retired)).await?;
    Ok(json!({"saved": true}))
}

/// `connection.check`（模块的说明）。
pub(super) async fn check(site: &Site, params: &Value) -> Result<Value, (String, String)> {
    let (station, _) = site.pick(params)?;
    let (index, _) = which(&station.status(), params).ok_or_else(bad)?;
    probe(site, params, json!({"index": index})).await
}

#[cfg(test)]
mod tests;
