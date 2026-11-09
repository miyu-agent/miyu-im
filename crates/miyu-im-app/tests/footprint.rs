//! 量尺（施工 I-183 中上：程序里编进 Telegram 的驱动，只用 QQ 的人多付多少）：真核心拉起程序，只配 QQ、不配 Telegram，等它起来、
//! 静 20 秒，读程序的常驻内存（`/proc/<进程号>/status` 的 `VmRSS`）和程序文件多大。标 `ignore`：照 release 编了手动跑，
//! `cargo test --release -p miyu-im-app --test all -- --ignored footprint --nocapture`，改前改后对比。只在 Linux 上量。

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "量尺：release 编了以后手动跑"]
async fn an_idle_program_with_only_qq_configured() {
    use std::time::Duration;

    use miyu_session::testkit::{Play, Script};

    use crate::support::Home;
    use crate::support::ports::on_free_port;
    use crate::support::spawning::{bridge_up, cli, ports_config, text};

    let script = Script::new([Play::Says("在")]);
    let (home, pid) = on_free_port(async |listen| {
        let home = Home::spawning(&script, &ports_config(listen));
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        let pid = bridge_up(&home.root, listen).await?;
        Ok((home, pid))
    })
    .await;
    tokio::time::sleep(Duration::from_secs(20)).await;
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).expect("读得到进程的状态");
    let rss = status
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .unwrap_or("VmRSS: ?");
    let size = std::fs::metadata(env!("CARGO_BIN_EXE_miyu-im"))
        .expect("有程序")
        .len();
    eprintln!("FOOTPRINT {rss} size={size}");
    home.stop_extensions().await;
}
