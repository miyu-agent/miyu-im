//! 连接表的令牌经真核心交过来（施工 I-184，核心 SL-1）：连接表 `qq.tokens` 是一列密钥引用，核心交配置时每一项解成真值的字，取不到的
//! 那一项交 `null`、不删（第几项对第几个连接）。以前核心交的是引用本身，后台页存过连接表的 QQ 连接都拿不到令牌，NapCat 连进来一律
//! 401；im 的测试一直用假核心直接交字，没测出来。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::head::Head;
use crate::support::napcat::admin_napcat;
use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, other_port, ports_config_with, text};
use crate::support::*;

#[tokio::test]
async fn tokens_in_the_connection_table_reach_the_bridge_as_values() {
    let script = Script::new([Play::Says("在")]);
    let (home, (gone, listen)) = on_free_port(async |listen| {
        // 头一个连接引用一个没有的密钥，第二个引用测试的令牌：第二个照旧对得上它自己的。
        let gone = other_port(listen);
        let table = format!(
            "qq.hosts = [\"127.0.0.1\", \"127.0.0.1\"]\nqq.ports = [{gone}, {listen}]\nqq.tokens = [{{ secret = \"im-gone\" }}, {{ secret = \"im\" }}]"
        );
        let home = Home::spawning(&script, &ports_config_with(listen, &table));
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, (gone, listen)))
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |result| {
            result["connections"]
                .as_array()
                .is_some_and(|all| all.len() == 2)
        })
        .await;
    assert_eq!(
        status["connections"],
        json!([
            {"host": "127.0.0.1", "listen": gone, "token": "none"},
            {"host": "127.0.0.1", "listen": listen, "token": "set"},
        ])
    );
    let mut napcat = admin_napcat(listen).await;
    assert!(napcat.alive().await, "照连接表的令牌连得进来");
    napcat.close().await;
    home.stop_extensions().await;
}
