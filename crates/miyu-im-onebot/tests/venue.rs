//! `miyu im venue show <场所>`（施工 O-21，`onebot.md` 第一条「场所规则和出厂数据」第 7 条）：读的文件和桥一样，配置项
//! 那一层向核心要（施工 O-41：来处印配置项的键；要不到的说连不上核心）；规则设到的一项一行印值和来处，接一句没列出的参数照出厂的，问题印在后面；场所编号认不出是用法不对，出厂的写错说
//! 是哪些问题。真的程序照系统的语言说；`serve` 出厂的写错握手以前就退，系统的写错照常起来、问题进运行日志。

use std::time::Duration;

use miyu_im::rules::Configured;
use miyu_im::texts::Texts;
use miyu_im::venue::show;
use miyu_session::testkit::Script;
use miyu_store::resources::ResourceRoot;
use miyu_store::root::DataRoot;

use crate::rules::{copied_resources, system_rule};
use crate::support::ports::on_free_port;
use crate::support::spawning::{finished, program, ran_to_end, served_up, text};
use crate::support::*;

/// 中文的字，资源目录是 `resources`。
fn zh(resources: &std::path::Path) -> Texts {
    Texts::load(ResourceRoot::at(resources), package_in(resources), "zh").expect("读得出来")
}

/// 照中文跑一次 `venue show <venue>`，配置项都没设：交回退出码、标准输出、标准错误。
fn shown(root: &DataRoot, venue: &str, texts: &Texts) -> (u8, String, String) {
    shown_with(root, venue, texts, Ok(Configured::default()))
}

/// 同 [`shown`]，配置项那一层要来的是 `configured`。
fn shown_with(
    root: &DataRoot,
    venue: &str,
    texts: &Texts,
    configured: Result<Configured, String>,
) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = show(root, venue, || configured, texts, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).expect("UTF-8"),
        String::from_utf8(err).expect("UTF-8"),
    )
}

/// 删掉临时目录；删不掉就留在临时目录里，不影响测试。
fn clean(dir: &std::path::Path) {
    if std::fs::remove_dir_all(dir).is_err() {
        // 留着。
    }
}

/// 系统的两份：`80-test.toml` 给群 1 设限流和一项参数，`90-bad.toml` 第 2 条规则的限流写错了。
fn test_rules(root: &DataRoot) {
    system_rule(
        root,
        "80-test.toml",
        "[[rule]]\nmatch = { group = [1] }\nrate = \"30/60s\"\nchatty = { probability = 80 }\n",
    );
    system_rule(
        root,
        "90-bad.toml",
        "[[rule]]\nmatch = { group = [2] }\nparallel = 2\n\n[[rule]]\nrate = \"abc\"\n",
    );
}

/// 数据根 `root` 里的卡片写成 `json`（施工 O-58 上）。
fn write_cards(root: &DataRoot, json: &str) {
    let dir = root.state().join("packages").join("miyu-im");
    std::fs::create_dir_all(&dir).expect("建得了目录");
    std::fs::write(dir.join("cards.json"), json).expect("写得进");
}

#[test]
fn values_and_origins_come_one_a_line_and_problems_after() {
    let (dir, root) = temp_root();
    test_rules(&root);
    assert_eq!(
        shown(&root, "qq:group:1", &zh(&resources())),
        (
            0,
            "allow = false（卡片「其他群」）\n\
             chatty.probability = 80（系统 80-test.toml 第 1 条规则，第 4 行）\n\
             discipline = \"chatty\"（出厂 50-defaults.toml 第 1 条规则，第 14 行）\n\
             outbound.final_only = true（出厂 50-defaults.toml 第 1 条规则，第 17 行）\n\
             parallel = 3（出厂 50-defaults.toml 第 1 条规则，第 16 行）\n\
             rate = \"30/60s\"（系统 80-test.toml 第 1 条规则，第 3 行）\n\
             recent = 30（出厂 50-defaults.toml 第 1 条规则，第 18 行）\n\
             show_ids = true（出厂 50-defaults.toml 第 1 条规则，第 19 行）\n\
             没列出的参数照出厂的 defaults.toml。\n\
             读文件时发现的问题（写错的那一项、那一条规则、那一份文件不用，别的照用）：\n  \
             系统 90-bad.toml 第 2 条规则，第 6 行：rate 写法不对：\"abc\"\n"
                .to_string(),
            String::new()
        )
    );
    clean(&dir);
}

#[test]
fn without_system_rules_the_factory_ones_are_shown() {
    let (dir, root) = temp_root();
    assert_eq!(
        shown(&root, "qq:private:5", &zh(&resources())),
        (
            0,
            "allow = false（卡片「其他私聊」）\n\
             discipline = \"every-message\"（出厂 50-defaults.toml 第 2 条规则，第 23 行）\n\
             parallel = 0（出厂 50-defaults.toml 第 2 条规则，第 25 行）\n\
             rate = \"5/300s\"（出厂 50-defaults.toml 第 2 条规则，第 24 行）\n\
             show_ids = true（出厂 50-defaults.toml 第 2 条规则，第 26 行）\n\
             没列出的参数照出厂的 defaults.toml。\n"
                .to_string(),
            String::new()
        )
    );
    clean(&dir);
}

#[test]
fn a_venue_nothing_sets_says_so() {
    let (dir, root) = temp_root();
    // 同名的空文件替换出厂的那一份：什么规则都没有（白名单成员的私聊：卡片「其他群」只管群，「其他私聊」不管白名单成员）。
    system_rule(&root, "50-defaults.toml", "");
    let listed = Configured::handed(&serde_json::json!({"im.whitelist": ["qq:1"]}));
    assert_eq!(
        shown_with(&root, "qq:private:1", &zh(&resources()), Ok(listed)),
        (
            0,
            "没有规则设到这个场所。\n没列出的参数照出厂的 defaults.toml。\n".to_string(),
            String::new()
        )
    );
    clean(&dir);
}

#[test]
fn a_bad_venue_id_is_a_usage_error() {
    let (dir, root) = temp_root();
    for wrong in [
        "qq:channel:1",
        "qq:group:",
        "qq:group:1 2",
        "QQ:group:1",
        "nonsense",
        "",
    ] {
        assert_eq!(
            shown(&root, wrong, &zh(&resources())),
            (
                2,
                String::new(),
                format!(
                    "认不出场所编号 {wrong}。写成 <平台>:group:<群号> 或 <平台>:private:<号>，例如 qq:group:123456。\n"
                )
            ),
            "{wrong:?}"
        );
    }
    clean(&dir);
}

#[test]
fn broken_factory_data_is_said_with_every_problem() {
    let (dir, root) = temp_root();
    let resources = copied_resources();
    let defaults = resources.join("packages/miyu-im/data/defaults.toml");
    let good = std::fs::read_to_string(&defaults).expect("读得了");
    let line = good
        .lines()
        .position(|line| line.starts_with("probability = 50 "))
        .expect("有这一行")
        + 1;
    std::fs::write(
        &defaults,
        good.replace("probability = 50 ", "probability = 5000 "),
    )
    .expect("写得进");
    let texts = zh(&resources);
    let (code, out, err) = shown(&root, "qq:group:1", &texts);
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(
        err,
        format!(
            "接入通讯平台的出厂数据有问题（是打包的错），起不来：\n  出厂 defaults.toml 第 {line} 行：chatty.probability 超出范围：5000\n"
        )
    );
    clean(&dir);
    clean(resources.parent().expect("有上一级"));
}

#[test]
fn settings_are_named_by_key_and_an_unreachable_core_is_said() {
    let (dir, root) = temp_root();
    let configured = Configured::handed(&serde_json::json!({
        "im.show_ids": true,
        "im.sleep": "abc",
    }));
    // 卡片那一层照数据根里的（施工 O-58 上）：群 2 有卡、设了限流。
    write_cards(
        &root,
        r#"{"groups": [{"id": 2, "set": {"rate": "7/60s"}}]}"#,
    );
    assert_eq!(
        shown_with(&root, "qq:group:2", &zh(&resources()), Ok(configured)),
        (
            0,
            "allow = true（群 2 的卡片）\n\
             discipline = \"chatty\"（出厂 50-defaults.toml 第 1 条规则，第 14 行）\n\
             outbound.final_only = true（出厂 50-defaults.toml 第 1 条规则，第 17 行）\n\
             parallel = 3（出厂 50-defaults.toml 第 1 条规则，第 16 行）\n\
             rate = \"7/60s\"（群 2 的卡片）\n\
             recent = 30（出厂 50-defaults.toml 第 1 条规则，第 18 行）\n\
             show_ids = true（配置项 im.show_ids）\n\
             没列出的参数照出厂的 defaults.toml。\n\
             读文件时发现的问题（写错的那一项、那一条规则、那一份文件不用，别的照用）：\n  \
             配置项 im.sleep：sleep 写法不对：\"abc\"\n"
                .to_string(),
            String::new()
        )
    );
    let (code, out, err) = shown_with(
        &root,
        "qq:group:2",
        &zh(&resources()),
        Err("refused".to_string()),
    );
    assert_eq!((code, out.as_str()), (1, ""));
    assert_eq!(err, "连不上核心：refused\n");
    clean(&dir);
}

/// 跑一次真的程序 `miyu-im <args>` 到完（系统的语言是英文），不占着测试的异步线程。
async fn ran(root: &DataRoot, args: &[&str]) -> std::process::Output {
    ran_to_end(program(root, args)).await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_program_shows_in_the_system_language() {
    // 配置项那一层向真核心要（施工 O-41）：核心照系统配置交 `im.show_ids`；卡片照数据根里的（施工 O-58 上）。
    let home = Home::spawning(
        &Script::new([]),
        &crate::support::spawning::ports_config_with(
            crate::support::spawning::free_port(),
            "show_ids = true",
        ),
    );
    let root = home.root.clone();
    test_rules(&root);
    write_cards(&root, r#"{"groups": [{"id": 1}]}"#);
    let shown = ran(&root, &["venue", "show", "qq:group:1"]).await;
    assert_eq!(shown.status.code(), Some(0), "{}", text(&shown.stderr));
    assert_eq!(text(&shown.stderr), "");
    assert_eq!(
        text(&shown.stdout),
        "allow = true (card of group 1)\n\
         chatty.probability = 80 (system 80-test.toml, rule 1, line 4)\n\
         discipline = \"chatty\" (factory 50-defaults.toml, rule 1, line 14)\noutbound.final_only = true (factory 50-defaults.toml, rule 1, line 17)\n\
         parallel = 3 (factory 50-defaults.toml, rule 1, line 16)\n\
         rate = \"30/60s\" (system 80-test.toml, rule 1, line 3)\n\
         recent = 30 (factory 50-defaults.toml, rule 1, line 18)\n\
         show_ids = true (setting im.show_ids)\n\
         Parameters not listed follow the factory defaults.toml.\n\
         Problems found while reading (the wrong item, rule or file is not used; the rest is):\n  \
         system 90-bad.toml, rule 2, line 6: rate is not written right: \"abc\"\n"
    );
    for wrong in [
        &["venue"][..],
        &["venue", "show"],
        &["venue", "list", "qq:group:1"],
    ] {
        let refused = ran(&root, wrong).await;
        assert_eq!(refused.status.code(), Some(2), "{wrong:?}");
        assert!(
            text(&refused.stderr).starts_with("usage: "),
            "{wrong:?}：{}",
            text(&refused.stderr)
        );
    }
    let refused = ran(&root, &["venue", "show", "qq:room:1"]).await;
    assert_eq!(refused.status.code(), Some(2));
    assert!(text(&refused.stderr).starts_with("Not a venue id: qq:room:1."));
}

#[test]
fn serve_with_broken_factory_data_stops_before_the_handshake() {
    let (dir, root) = temp_root();
    let resources = copied_resources();
    std::fs::write(
        resources.join("packages/miyu-im/data/venues.d/50-defaults.toml"),
        "[[rule]]\nrate = \"abc\"\n",
    )
    .expect("写得进");
    let mut command = program(&root, &["serve"]);
    command
        .env("MIYU_RESOURCES", &resources)
        .env("MIYU_PACKAGE_DIR", package_in(&resources).dir())
        .stdin(std::process::Stdio::null());
    let exited = finished(command);
    assert_eq!(exited.status.code(), Some(1), "{}", text(&exited.stderr));
    assert_eq!(
        text(&exited.stdout),
        "",
        "握手以前就退：标准输出上什么都没有"
    );
    assert_eq!(
        text(&exited.stderr),
        "Connect Messaging cannot start: its factory data has problems (a packaging error):\n  \
         factory 50-defaults.toml, rule 1, line 2: rate is not written right: \"abc\"\n"
    );
    clean(&dir);
    clean(resources.parent().expect("有上一级"));
}

#[tokio::test]
async fn serve_with_a_system_mistake_starts_and_logs_it() {
    let (dir, root) = temp_root();
    test_rules(&root);
    let mut served = on_free_port(async |listen| served_up(&root, listen).await).await;
    // 核心请它退出：关它的标准输入；好好停下，运行日志写完。
    drop(served.stdin.take());
    let exited = tokio::time::timeout(Duration::from_secs(5), served.child.wait_with_output())
        .await
        .expect("5 秒内退出")
        .expect("等得到");
    assert_eq!(exited.status.code(), Some(0), "{}", text(&exited.stderr));
    let log =
        std::fs::read_to_string(root.state().join("logs").join("miyu-im.log")).expect("有运行日志");
    let problem = log
        .lines()
        .find(|line| line.contains("venue rules problem"))
        .unwrap_or_else(|| panic!("记了问题：{log}"));
    for must in [
        "WARN",
        "bad_format",
        "90-bad.toml",
        "rule=2",
        "key=rate",
        "line=6",
    ] {
        assert!(problem.contains(must), "{must}：{problem}");
    }
    assert!(log.contains("venue rules read problems=1"), "{log}");
    clean(&dir);
}
