//! 查一个令牌（施工 I-183 下中，`Driver::probe`）：新建连接时页面交来的令牌和代理，或者连接表里第几个（连接设置里「重新检查」），
//! 照 `getMe` 查：交回机器人的号、名字、用户名、隐私模式开没开。原因码：`bad_params`（没交令牌、第几个对不上）、`bad_proxy`
//! （代理地址读不懂）、`bad_token`（401）、`unreachable`（连不上、等不到、回的读不懂）、别的照 Telegram 说的（`failed`）。

use std::time::Duration;

use serde_json::{Map, Value, json};

use miyu_client::settings::Secret;

use crate::api::{Api, Failure, Proxy};
use crate::bot::full_name;
use crate::settings;

/// 查一次最多等多久：页面上等着。
const WAIT: Duration = Duration::from_secs(20);

/// 照 `params`、驱动最新的配置 `config` 查。
pub async fn probe(params: Value, config: Map<String, Value>) -> Result<Value, String> {
    let (connections, api) = settings::read(&config, |_| None);
    let (token, proxy) = match params["index"].as_u64() {
        Some(index) => {
            let found = connections
                .iter()
                .find(|one| u64::try_from(one.index).ok() == Some(index))
                .ok_or("bad_params")?;
            (
                found.token.clone().ok_or("bad_params")?,
                found.proxy.clone(),
            )
        }
        None => {
            let token = params["token"]
                .as_str()
                .and_then(|token| Secret::new(token.trim()).ok())
                .ok_or("bad_params")?;
            (
                token,
                Proxy::parse(params["proxy"].as_str().unwrap_or_default()),
            )
        }
    };
    let client = Api::new(&api, &token, &proxy).map_err(|_| "bad_proxy".to_string())?;
    match client.call("getMe", &json!({}), WAIT).await {
        Ok(me) => {
            let bot = me["id"].as_i64().ok_or("unreachable")?;
            Ok(json!({
                "bot": bot.to_string(),
                "name": full_name(&me),
                "username": me["username"].as_str().unwrap_or_default(),
                "privacy": me["can_read_all_group_messages"] != true,
            }))
        }
        Err(Failure::Unauthorized) => Err("bad_token".to_string()),
        Err(Failure::Network(_) | Failure::Retry(_) | Failure::Conflict) => {
            Err("unreachable".to_string())
        }
        Err(Failure::Failed { .. }) => Err("failed".to_string()),
    }
}
