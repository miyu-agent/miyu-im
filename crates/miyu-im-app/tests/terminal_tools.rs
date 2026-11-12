//! 终端、网页里用 im 的场所工具（设计 13，施工 I-218）：真核心拉起真桥、假 NapCat。本机的会话里查看的几件在工具面上，照参数
//! `group`、`private` 去那个场所办，结果前头写是哪个群；没说场所的答 `chat-needed`；那个场所关着这件的答 `chat-off`（照场所，预设
//! 开着不算数）。群里写了别的群的：普通成员叫的答 `only-here`，终端管理员直接叫的照办。

use serde_json::json;

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 别的群：假 NapCat 问谁的群成员都照 [`MEMBERS`] 回。
const ELSEWHERE: i64 = 999;

/// 造一个本机的会话（终端、网页那种），切成完全放开，叫她做 `text`（同 `outreach.rs`）。
async fn local_session(home: &Home, text: &str) {
    local_session_then(home, text, |_| {}).await;
}

/// 同 [`local_session`]，叫她以前先照这个会话的工作目录做 `before`（比如放一个要传的文件）。
async fn local_session_then(home: &Home, text: &str, before: impl FnOnce(&std::path::Path)) {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let cwd = home.root.path().to_string_lossy().into_owned();
    let made = core
        .call("local-1", "session.create", json!({"cwd": cwd}))
        .await
        .expect("造得出");
    let session = made["session"].clone();
    let id = session.as_str().expect("有编号").to_string();
    let log = home.root.path().join("root/sessions").join(&id);
    let created = std::fs::read_dir(&log)
        .expect("有会话日志")
        .filter_map(Result::ok)
        .find_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .expect("读得出");
    let first: serde_json::Value =
        serde_json::from_str(created.lines().next().expect("有一行")).expect("是 JSON");
    before(std::path::Path::new(
        first["body"]["cwd"].as_str().expect("有工作目录"),
    ));
    let full = json!({"session": session, "level": "full"});
    core.call("local-2", "session.set_permission_level", full)
        .await
        .expect("切得了");
    let asked = json!({"session": session, "text": text});
    core.call("local-3", "session.send", asked)
        .await
        .expect("发得进");
}

/// 等她拿到第二次请求（工具的结果交回去了），交回她发出的请求写成的字。
async fn answered(lines: &Lines) -> (Vec<String>, String) {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    while lines.requests().len() < 2 {
        assert!(tokio::time::Instant::now() < deadline, "等不到她拿到结果");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let requests = lines.requests();
    let tools = requests[0]
        .tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect();
    (tools, format!("{:?}", requests[1]))
}

#[tokio::test]
async fn from_the_terminal_she_reads_a_group_s_members() {
    let lines = Lines::new(uses("qq_members", "{\"group\":31}"));
    let (home, _napcat) = up(&lines).await;
    local_session(&home, "看看 31 群有谁").await;
    let (tools, result) = answered(&lines).await;
    for tool in [
        "qq_members",
        "qq_chat_history",
        "qq_contacts",
        "qq_notices",
        "qq_fetch_media",
        "qq_avatar",
    ] {
        assert!(
            tools.iter().any(|one| one == tool),
            "终端的工具面里有 {tool}：{tools:?}"
        );
    }
    for tool in ["qq_mute", "qq_quote", "qq_skip_reply"] {
        assert!(
            !tools.iter().any(|one| one == tool),
            "{tool} 只在群里：{tools:?}"
        );
    }
    let head = sentence("chat-in-group", &[("group", &GROUP.to_string())]);
    assert!(result.contains(&head), "结果前头写是哪个群：{result}");
    assert!(result.contains("小明"), "列了群成员：{result}");
    stopped(home).await;
}

#[tokio::test]
async fn from_the_terminal_without_a_chat_she_is_asked_which() {
    let lines = Lines::new(uses("qq_members", "{}"));
    let (home, _napcat) = up(&lines).await;
    local_session(&home, "看看群里有谁").await;
    let (_, result) = answered(&lines).await;
    assert!(result.contains(&sentence("chat-needed", &[])), "{result}");
    stopped(home).await;
}

#[tokio::test]
async fn a_chat_with_the_tool_turned_off_says_so_even_from_the_terminal() {
    // 设计 13 第一节第 3 条：预设开着（本机的会话照出厂的全部功能），那个群的卡片关了查询信息，照场所拒。
    let lines = Lines::new(uses("qq_members", "{\"group\":31}"));
    let (home, _napcat) = up(&lines).await;
    let mut head = Head::connect(&home.root).await;
    let off =
        json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"im-qq-lookup": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    local_session(&home, "看看 31 群有谁").await;
    let (_, result) = answered(&lines).await;
    assert!(result.contains(&sentence("chat-off", &[])), "{result}");
    assert!(!result.contains("小明"), "没去问：{result}");
    stopped(home).await;
}

#[tokio::test]
async fn in_a_group_only_the_owner_asking_directly_reaches_another_group() {
    let args: &'static str = format!(r#"{{"group":{ELSEWHERE}}}"#).leak();
    let lines = Lines::new(
        [uses("qq_members", args), uses("qq_members", args)]
            .into_iter()
            .flatten(),
    );
    let (home, napcat) = up(&lines).await;
    // 普通成员叫她看别的群：不去。
    let (said, status) = asks(
        &home,
        &napcat,
        MEMBER,
        101,
        json!([plain(" 看看 999 群有谁")]),
        1,
    )
    .await;
    assert_eq!(said, sentence("only-here", &[]), "{status}");
    // 终端管理员直接叫她：去那个群看，结果前头写是哪个群。
    says(
        &home,
        &napcat,
        (ADMIN, "member"),
        102,
        called(json!([plain(" 看看 999 群有谁")])),
    )
    .await;
    let (said, _) = ended(&home, 2).await;
    let head = sentence("chat-in-group", &[("group", &ELSEWHERE.to_string())]);
    assert!(said.starts_with(&head), "{said}");
    assert!(said.contains("小明"), "{said}");
    stopped(home).await;
}

#[tokio::test]
async fn from_the_terminal_she_lists_a_group_s_files_and_sends_one() {
    // 施工 I-219：群文件、发文件也给终端。列的是那个群的根目录，结果前头写是哪个群；发的文件照那个群传（`upload_group_file`）。
    // 一轮里接连调两件：列群文件、传文件，再说一句。
    let lines = Lines::new([
        Line::uses("", "qq_group_files", "{\"group\":31}"),
        Line::uses("", "qq_send_file", "{\"group\":31,\"path\":\"report.txt\"}"),
        Line::says("好。"),
    ]);
    let (home, mut napcat) = up(&lines).await;
    napcat.stock_file(
        "root",
        Ok(json!({"folders": [], "files": [{"file_id": "/f1", "file_name": "排班.pdf", "size": 2048, "uploader_name": "小明", "busid": 102}]})),
    );
    local_session_then(
        &home,
        "看看 31 群的群文件，再把 report.txt 传上去",
        |cwd| {
            std::fs::write(cwd.join("report.txt"), b"hello").expect("写得进");
        },
    )
    .await;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    while lines.requests().len() < 3 {
        assert!(tokio::time::Instant::now() < deadline, "等不到两次结果");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let listed = format!("{:?}", lines.requests()[1]);
    let head = sentence("chat-in-group", &[("group", &GROUP.to_string())]);
    assert!(
        listed.contains(&head) && listed.contains("排班.pdf"),
        "{listed}"
    );
    let upload = loop {
        let action = napcat.action().await;
        if action["action"] == "upload_group_file" {
            break action;
        }
    };
    assert_eq!(upload["params"]["group_id"], GROUP, "{upload}");
    assert_eq!(upload["params"]["name"], "report.txt", "{upload}");
    stopped(home).await;
}
