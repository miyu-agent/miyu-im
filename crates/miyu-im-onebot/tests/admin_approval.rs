//! 入群审批（施工 Q-7，设计 02 第二节）：真核心拉起真桥、假 NapCat 推加群申请，她照台词审（核心的 `model.call` 也照台词答）。写了
//! 通过条件的群：通过的同意、拒绝的带理由拒绝、读不出来（重试一次）的留待人工不处理，每一次进管理记录；没写的群不审、不问模型。

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::judge;
use crate::support::platform::*;
use crate::support::*;

/// 写这个群的通过条件。
fn rule(home: &Home, rule: &str) {
    let dir = home.root.state().join("packages/im/qq/admin");
    std::fs::create_dir_all(&dir).expect("建得了");
    let groups = json!({GROUP.to_string(): {"approval": {"rule": rule}}});
    std::fs::write(dir.join("groups.json"), groups.to_string()).expect("写得进");
}

/// `user` 申请进群，写了 `comment`，标记 `flag`。
fn applies(napcat: &Answering, user: i64, comment: &str, flag: &str) {
    napcat.send(json!({"post_type": "request", "request_type": "group", "sub_type": "add", "time": TIME, "self_id": BOT, "group_id": GROUP, "user_id": user, "comment": comment, "flag": flag}));
}

/// 这个群的管理记录。
async fn records(home: &Home, count: usize) -> Vec<Value> {
    let path = home
        .root
        .state()
        .join("packages/im/qq/admin")
        .join(format!("{GROUP}.jsonl"));
    within("记下了", async {
        loop {
            let lines: Vec<Value> = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .lines()
                .map(|line| serde_json::from_str(line).expect("是 JSON"))
                .collect();
            if lines.len() >= count {
                return lines;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
}

#[tokio::test]
async fn requests_are_approved_rejected_or_held_and_recorded() {
    // 审批走核心的 `model.call`：发到判官的假服务器，第几次回第几份。
    let server = judge::judge(vec![
        judge::says(r#"{"verdict": "approve", "reason": "答对了"}"#),
        judge::says(r#"好的 {"verdict": "reject", "reason": "暗号不对"}"#),
        judge::says("我觉得可以"),
        judge::says("还是说不好"),
    ])
    .await;
    let (home, mut napcat, _) = started_judged_ranked(
        &miyu_session::testkit::Script::new([]),
        &server,
        &rules(),
        (MEMBERS, RANKS),
    )
    .await;
    rule(&home, "暗号是芝麻开门");
    applies(&napcat, 50001, "问题：暗号\n答案：芝麻开门", "f1");
    let approved = next_platform(&mut napcat).await;
    assert_eq!(approved["action"], "set_group_add_request", "{approved}");
    assert_eq!(
        approved["params"],
        json!({"flag": "f1", "sub_type": "add", "approve": true})
    );
    // 问模型的照规矩：提示词英文，通过条件、申请人写的照原样放进去。
    let asked = judge::asked(&server, 0).to_string();
    assert!(asked.contains("暗号是芝麻开门"), "{asked}");
    assert!(
        asked.contains("join-approval") || asked.contains("You review requests"),
        "{asked}"
    );
    applies(&napcat, 50002, "问题：暗号\n答案：随便", "f2");
    let rejected = next_platform(&mut napcat).await;
    assert_eq!(
        rejected["params"],
        json!({"flag": "f2", "sub_type": "add", "approve": false, "reason": "暗号不对"})
    );
    // 读不出来、重试一次还读不出来：留待人工，不处理。
    applies(&napcat, 50003, "", "f3");
    let written = records(&home, 3).await;
    let shape: Vec<(Value, Value, Value)> = written
        .iter()
        .map(|record| {
            (
                record["action"].clone(),
                record["user"].clone(),
                record["verdict"].clone(),
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            (json!("join"), json!(50001), json!("approve")),
            (json!("join"), json!(50002), json!("reject")),
            (json!("join"), json!(50003), json!("hold")),
        ]
    );
    assert_eq!(written[1]["reason"], "暗号不对");
    assert_eq!(
        written[0]["comment"], "问题：暗号 答案：芝麻开门",
        "记录一条一行"
    );
    assert_eq!(server.received().len(), 4, "读不出来的重试了一次");
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn a_group_without_a_rule_is_left_to_people() {
    let server = judge::judge(vec![]).await;
    let (home, mut napcat, _) = started_judged_ranked(
        &miyu_session::testkit::Script::new([]),
        &server,
        &rules(),
        (MEMBERS, RANKS),
    )
    .await;
    applies(&napcat, 50001, "我想进群", "f1");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(server.received().is_empty(), "不问模型");
    let path = home
        .root
        .state()
        .join("packages/im/qq/admin")
        .join(format!("{GROUP}.jsonl"));
    assert!(!path.exists(), "不记");
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn groups_where_group_admin_is_not_on_are_left_to_people() {
    let server = judge::judge(vec![]).await;
    // 她是普通成员。
    let (home, mut napcat, _) = started_judged_ranked(
        &miyu_session::testkit::Script::new([]),
        &server,
        &rules(),
        (MEMBERS, &[(OWNER, "owner")]),
    )
    .await;
    rule(&home, "暗号是芝麻开门");
    applies(&napcat, 50001, "芝麻开门", "f1");
    within("问过她的身份", async {
        while !napcat.asked().contains(&BOT) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(server.received().is_empty(), "不是管理员不问模型");
    no_platform(&mut napcat);
    stopped(home).await;
    // 这个群关了群管理：不审；打开以后再申请的才审（模型只被问到这一次）。
    let server = judge::judge(vec![judge::says(
        r#"{"verdict": "approve", "reason": "好"}"#,
    )])
    .await;
    let (home, mut napcat, _) = started_judged_ranked(
        &miyu_session::testkit::Script::new([]),
        &server,
        &rules(),
        (MEMBERS, RANKS),
    )
    .await;
    rule(&home, "暗号是芝麻开门");
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let switch = |on: bool| json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-admin": on}}});
    let saved = head.page_call_with("cards.save_one", switch(false)).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    applies(&napcat, 50002, "关着的时候", "f2");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let saved = head.page_call_with("cards.save_one", switch(true)).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    applies(&napcat, 50003, "开着的时候", "f3");
    let approved = next_platform(&mut napcat).await;
    assert_eq!(approved["params"]["flag"], "f3", "{approved}");
    assert_eq!(server.received().len(), 1, "关着时的那一次没问");
    assert!(judge::asked(&server, 0).to_string().contains("开着的时候"));
    no_platform(&mut napcat);
    stopped(home).await;
}
