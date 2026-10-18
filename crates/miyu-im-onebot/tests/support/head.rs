//! 照网页软件的样子连着核心的头（施工 O-28 上起后台页的测试用，施工 O-42 挪来：给别的包调的方法也经 `package.call` 调）。

use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};

use miyu_ipc::Connection;
use miyu_store::root::DataRoot;

use super::within;

/// 照网页软件的样子连着核心的头：出示本机令牌，一问一答，交回整条回应（拒绝的 `data` 也在）。
pub struct Head {
    lines: BufReader<ReadHalf<Connection>>,
    write: WriteHalf<Connection>,
    next: u64,
    /// 等回应时读到的推送（施工 O-58 下：界面块的 `widget.changed`），照先后留着给 [`Head::pushed`]。
    pushes: std::collections::VecDeque<Value>,
}

impl Head {
    /// 连数据根 `root` 上跑着的核心、握手。
    pub async fn connect(root: &DataRoot) -> Head {
        let (connection, token) = miyu_ipc::connect(root).await.expect("连得上核心");
        let (read, write) = tokio::io::split(connection);
        let mut head = Head {
            lines: BufReader::new(read),
            write,
            next: 0,
            pushes: std::collections::VecDeque::new(),
        };
        let hello =
            json!({"protocol": [1, 1], "head": {"kind": "test", "version": "0"}, "token": token});
        let shaken = head.call("hello", hello).await;
        assert!(shaken.get("result").is_some(), "握手：{shaken}");
        head
    }

    /// 发一条请求，等到它的回应，原样交回。
    pub async fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = format!("head-{}", self.next);
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.write
            .write_all(format!("{request}\n").as_bytes())
            .await
            .expect("写得进");
        self.write.flush().await.expect("写得出");
        within("核心回", async {
            loop {
                let mut line = String::new();
                let read = self.lines.read_line(&mut line).await.expect("读得了");
                assert!(read > 0, "核心断开了");
                let Ok(reply) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if reply["id"] == id && reply.get("method").is_none() {
                    return reply;
                }
                if reply.get("method").is_some() && reply.get("id").is_none() {
                    self.pushes.push_back(reply);
                }
            }
        })
        .await
    }

    /// 等到一条合 `wanted` 的推送（先看等回应时留下的），交回它；不合的扔掉。最多等六十秒。
    pub async fn pushed(&mut self, wanted: impl Fn(&Value) -> bool) -> Value {
        while let Some(push) = self.pushes.pop_front() {
            if wanted(&push) {
                return push;
            }
        }
        within("核心推", async {
            loop {
                let mut line = String::new();
                let read = self.lines.read_line(&mut line).await.expect("读得了");
                assert!(read > 0, "核心断开了");
                let Ok(push) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if push.get("id").is_none() && wanted(&push) {
                    return push;
                }
            }
        })
        .await
    }

    /// 经 `package.call` 调接入QQ 登记的 `method`：交回整条回应。
    pub async fn page_call(&mut self, method: &str) -> Value {
        self.page_call_with(method, json!({})).await
    }

    /// 同 [`Head::page_call`]，带参数 `params`（施工 O-58 上：`cards.save`）。
    pub async fn page_call_with(&mut self, method: &str, params: Value) -> Value {
        self.call(
            "package.call",
            json!({"package": "onebot", "method": method, "params": params}),
        )
        .await
    }

    /// 调 `method`，直到回的 `result` 合 `wanted`（桥登记方法、换配置、NapCat 连上都是异步的）：交回那一次的 `result`。
    pub async fn until(&mut self, method: &str, wanted: impl Fn(&Value) -> bool) -> Value {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let reply = self.page_call(method).await;
            if wanted(&reply["result"]) {
                return reply["result"].clone();
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "等不到 {method}：{reply}"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}
