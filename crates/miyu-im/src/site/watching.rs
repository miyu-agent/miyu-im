//! 通道上的实时日志（设计 10 第二节第 4 条，施工 I-160 中）：`watch.start {venue?}` 开始看一个场所（短编号，空的是全部），答
//! `{"batches"}`（最近留着的）；之后推 `{"event": "watch", "batch"}`；跟不上丢了的推 `{"event": "watch", "reset": true,
//! "batches"}`，页面照它整个换掉。`watch.stop` 停；`watch.venues` 答见过的场所 `{"venues": [{"venue", "name"}]}`（命令行认
//! `qq:号`）。一条通道同时只看一处，再开始看的先停前一处。通道断了照放下算，不再推。

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;

use crate::watch::hub::{Batch, Hub, Watching};

/// 一条通道正在看的。
pub(super) struct Looking {
    /// 看哪个场所（短编号），空的是全部。
    venue: Option<String>,
    receiver: Receiver<Batch>,
    /// 放下就不再算这条通道在看。
    _watching: Watching,
}

/// 通道上的实时日志方法：`watch.start`、`watch.stop`、`watch.venues` 答了的交回答的；别的方法交回空的。
pub(super) fn answer(
    hub: &Arc<Hub>,
    looking: &mut Option<Looking>,
    method: Option<&str>,
    params: &Value,
) -> Option<Value> {
    match method? {
        "watch.start" => {
            *looking = None;
            let venue = params["venue"]
                .as_str()
                .filter(|venue| !venue.is_empty())
                .map(str::to_string);
            let (recent, receiver, watching) = hub.watch(venue.as_deref());
            *looking = Some(Looking {
                venue,
                receiver,
                _watching: watching,
            });
            Some(json!({"batches": recent}))
        }
        "watch.stop" => {
            *looking = None;
            Some(json!({}))
        }
        "watch.venues" => {
            let venues: Vec<Value> = hub
                .venues()
                .into_iter()
                .map(|(venue, name)| json!({"venue": venue, "name": name}))
                .collect();
            Some(json!({"venues": venues}))
        }
        _ => None,
    }
}

/// 下一帧要推的：看着的场所来了一批、跟不上了重发最近的。没在看的一直等（交给 `select!` 的另一头）。
pub(super) async fn next(hub: &Hub, looking: &mut Option<Looking>) -> Value {
    let Some(look) = looking.as_mut() else {
        return std::future::pending().await;
    };
    loop {
        match look.receiver.recv().await {
            Ok(batch)
                if look
                    .venue
                    .as_deref()
                    .is_none_or(|venue| venue == batch.venue) =>
            {
                return json!({"event": "watch", "batch": batch});
            }
            Ok(_) => {}
            Err(RecvError::Lagged(_)) => {
                return json!({"event": "watch", "reset": true, "batches": hub.recent(look.venue.as_deref())});
            }
            // 汇总放下了（桥在停）：不再推。
            Err(RecvError::Closed) => return std::future::pending().await,
        }
    }
}
