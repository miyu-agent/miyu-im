//! 真机实测（施工 I-183）：接一个真的 Telegram 机器人跑一阵，人在 Telegram 里私聊、在群里 @ 它、回复它，看它回不回、回在哪。模型照剧本
//! 回「（实测）第 n 句」，不请求真模型。标 `ignore`，照环境变量跑（`MIYU_IM_LIVE_TG_OWNER` 写了的，那个号算主人、私聊里有命令菜单）：
//! `MIYU_IM_LIVE_TG_TOKEN=<令牌> MIYU_IM_LIVE_SECONDS=600 cargo test -p miyu-im-app --test all -- --ignored telegram_live --nocapture`。
//! Telegram 的「其他群」「其他私聊」打开（谁都回）。跑的时候把运行日志里 Telegram、判断、发出去的几行印出来。

use std::time::Duration;

use miyu_session::testkit::{Play, Script};

use crate::support::ports::on_free_port;
use crate::support::spawning::{bridge_up, cli, ports_config_with, text};
use crate::support::*;

#[tokio::test]
#[ignore = "真机实测：要真的机器人令牌，手动跑"]
async fn telegram_live() {
    let seconds: u64 = std::env::var("MIYU_IM_LIVE_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    let plays: Vec<Play> = (1..=60)
        .map(|n| Play::Says(Box::leak(format!("（实测）第 {n} 句").into_boxed_str())))
        .collect();
    let script = Script::new(plays);
    let extra = "tg.tokens = [{ secret = \"tg\" }]";
    // 主人的 Telegram 号（`MIYU_IM_LIVE_TG_OWNER`）写进对应表：她的私聊里登记命令菜单（I-183 下上）。没写的不登记。
    let owner = std::env::var("MIYU_IM_LIVE_TG_OWNER")
        .map(|owner| format!("\"tg:{owner}\" = \"owner\"\n"))
        .unwrap_or_default();
    let home = on_free_port(async |listen| {
        let config = format!("{owner}{}", ports_config_with(listen, extra));
        let home = Home::spawning(&script, &config);
        let dir = miyu_im::dirs::state(&home.root);
        std::fs::create_dir_all(&dir).expect("建得了目录");
        let cards = r#"{"others": {"tg": {"groups": {"on": true, "final_only": false}, "privates": {"on": true}}}}"#;
        std::fs::write(dir.join("cards.json"), cards).expect("写得进");
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok(home)
    })
    .await;
    eprintln!("LIVE 起来了，跑 {seconds} 秒");
    let log = miyu_im::dirs::log_file(&home.root);
    let mut shown = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    while tokio::time::Instant::now() < deadline {
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        for line in text.lines().skip(shown) {
            if [
                "telegram",
                "command menu",
                "command",
                "chat decided",
                "reply sent",
                "reply aimed",
                "private",
                "message sent in",
                "WARN",
                "ERROR",
            ]
            .iter()
            .any(|key| line.contains(key))
            {
                eprintln!("LIVE {line}");
            }
        }
        shown = text.lines().count();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    home.stop_extensions().await;
}
