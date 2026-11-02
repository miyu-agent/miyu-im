//! 踢人照旧版（施工 I-17）：真核心拉起真桥、假 NapCat，她照台词说。谁叫的都直接做（施工 I-20：不要二次确认，2026-10-21 项目主人
//! 定）。一次几个人：一个个做、一个个报，群主和管理员不踢、不在群里的不踢；NapCat 踢成了却回失败的（实测的毛病）现问成员
//! 核实，走了的算成、记进管理记录。禁言一次几个人同样一个个报。施工 I-97（项目主人 11-02）：禁言、踢人都先现问对方，群主谁都
//! 动不了、管理员只有她是群主时动得了，挡下的不调 NapCat。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::answering::Member;
use crate::support::group::*;
use crate::support::kicking::KICKED_ANYWAY;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 不在群里的号。
const STRANGER: i64 = 40099;

/// 这几个测试的群成员：多一个踢成了却回失败的。
const KICKING: &[Member] = &[
    (BOT, "米尤", "miyu"),
    (MEMBER, "小明", "ming"),
    (IMPOSTOR, "终端管理员", "fake"),
    (MANAGER, "群管", "guan"),
    (OWNER, "群主", "zhu"),
    (KICKED_ANYWAY, "阿强", "qiang"),
];

/// 起核心、桥、假 NapCat：她照 `lines` 说。
async fn kicking(lines: &Lines) -> (Home, crate::support::answering::Answering) {
    ranked_as(lines, KICKING, RANKS).await
}

/// 同 [`kicking`]，群成员照 `members`、身份照 `ranks`。
async fn ranked_as(
    lines: &Lines,
    members: &'static [Member],
    ranks: &'static [crate::support::answering::Rank],
) -> (Home, crate::support::answering::Answering) {
    let (home, napcat, _) = started_ranked(
        Arc::new(lines.clone()),
        &rules(),
        &onebot(),
        (members, ranks),
    )
    .await;
    (home, napcat)
}

/// 多一个 QQ 的群管理员（施工 I-97）。
const WITH_ADMIN: &[Member] = &[
    (BOT, "米尤", "miyu"),
    (MEMBER, "小明", "ming"),
    (MANAGER, "群管", "guan"),
    (OWNER, "群主", "zhu"),
    (QQ_ADMIN, "管理员", "admin"),
];

/// 这个群第 `turn` 轮完以后所有的工具结果：（字，状态），照先后。
async fn results(home: &Home, turn: usize) -> Vec<(String, String)> {
    let events = until_events(&home.root, &venue(), |events| {
        of_kind(events, "turn.ended").len() >= turn
    })
    .await;
    of_kind(&events, "tool.result")
        .iter()
        .map(|result| {
            let text = result["body"]["blocks"][0]["text"]
                .as_str()
                .unwrap_or_default();
            let status = result["body"]["status"].as_str().unwrap_or_default();
            (text.trim_end().to_string(), status.to_string())
        })
        .collect()
}

/// 这个群的管理记录，照先后（同 `admin.rs`）。
fn records(home: &Home) -> Vec<Value> {
    let path = miyu_im::dirs::state(&home.root)
        .join("qq/admin")
        .join(format!("{GROUP}.jsonl"));
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("一行一条 JSON"))
        .collect()
}

#[tokio::test]
async fn a_member_asking_for_a_kick_gets_it_done_at_once() {
    // 施工 I-20：群友叫的也不用确认；记录只有备注，不记谁叫的。
    let lines = Lines::new([
        Line::uses("", "qq_kick", r#"{"user":40002,"reason":"刷广告"}"#),
        Line::says("好。"),
    ]);
    let (home, mut napcat) = kicking(&lines).await;
    let words = json!([plain("把 40002 踢了")]);
    says(&home, &napcat, (MANAGER, "member"), 1101, called(words)).await;
    assert_eq!(
        results(&home, 1).await,
        [done("kicked", &[("who", "40002")])]
    );
    let kicked = next_platform(&mut napcat).await;
    assert_eq!(kicked["action"], "set_group_kick", "{kicked}");
    assert_eq!(kicked["params"]["user_id"], IMPOSTOR);
    let written = records(&home);
    assert_eq!(written.len(), 1, "{written:?}");
    assert_eq!(written[0]["by"], json!({"kind": "ai"}));
    assert_eq!(written[0]["reason"], "刷广告");
    stopped(home).await;
}

#[tokio::test]
async fn kicking_several_spares_owners_admins_and_strangers_and_checks_a_failed_kick() {
    let lines = Lines::new(
        [
            uses("qq_kick", r#"{"users":[40001,40004,40099,40008]}"#),
            uses("qq_mute", r#"{"seconds":60,"users":["40001",40002]}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = kicking(&lines).await;
    let words = json!([plain("这几个都踢了")]);
    // 群主叫的：不用确认。
    let (text, state) = asks(&home, &napcat, OWNER, 1201, words, 1).await;
    let wanted = [
        sentence("batch", &[("done", "2"), ("total", "4")]),
        sentence("kicked", &[("who", "40001")]),
        sentence("protected", &[("who", "群主")]),
        sentence("not-members", &[("who", &STRANGER.to_string())]),
        sentence("kicked", &[("who", &KICKED_ANYWAY.to_string())]),
    ];
    assert_eq!(text, wanted.join("\n"), "一人一行");
    assert_eq!(state, "ok", "有成了的");
    // 只踢了两个：群主、不在群里的没调 NapCat；回失败的那一个核实走了。
    let first = next_platform(&mut napcat).await;
    let second = next_platform(&mut napcat).await;
    assert_eq!(
        (
            first["params"]["user_id"].clone(),
            second["params"]["user_id"].clone()
        ),
        (json!(MEMBER), json!(KICKED_ANYWAY))
    );
    let written = within("记下两条", async {
        loop {
            let written = records(&home);
            if written.len() >= 2 {
                return written;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let users: Vec<&Value> = written.iter().map(|record| &record["user"]).collect();
    assert_eq!(users, [&json!(MEMBER), &json!(KICKED_ANYWAY)], "成了的才记");
    // 禁言一次两个（号写成字的也认）。
    let words = json!([plain("禁言这两个")]);
    let (text, state) = asks(&home, &napcat, MANAGER, 1202, words, 2).await;
    let wanted = [
        sentence("batch", &[("done", "2"), ("total", "2")]),
        sentence("muted", &[("who", "40001"), ("duration", "1m")]),
        sentence("muted", &[("who", "40002"), ("duration", "1m")]),
    ];
    assert_eq!((text, state), (wanted.join("\n"), "ok".to_string()));
    stopped(home).await;
}

#[tokio::test]
async fn muting_spares_the_owner_admins_and_strangers_when_she_is_an_admin() {
    // 施工 I-97：QQ 收得下禁言管理员的请求、实际没成；她是管理员时群主、管理员都不禁，不在群里的不禁，挡下的不调 NapCat。
    let lines = Lines::new(uses(
        "qq_mute",
        r#"{"seconds":60,"users":[40004,40006,40001,40099]}"#,
    ));
    let ranks: &'static [crate::support::answering::Rank] =
        &[(OWNER, "owner"), (BOT, "admin"), (QQ_ADMIN, "admin")];
    let (home, mut napcat) = ranked_as(&lines, WITH_ADMIN, ranks).await;
    let words = json!([plain("禁言这几个")]);
    let (text, state) = asks(&home, &napcat, MANAGER, 1301, words, 1).await;
    let wanted = [
        sentence("batch", &[("done", "1"), ("total", "4")]),
        sentence("protected", &[("who", "40004")]),
        sentence("protected", &[("who", "40006")]),
        sentence("muted", &[("who", "40001"), ("duration", "1m")]),
        sentence("not-members", &[("who", &STRANGER.to_string())]),
    ];
    assert_eq!((text, state), (wanted.join("\n"), "ok".to_string()));
    let banned = next_platform(&mut napcat).await;
    assert_eq!(banned["action"], "set_group_ban", "{banned}");
    assert_eq!(banned["params"]["user_id"], MEMBER, "只禁了普通成员");
    // 挡下的没调 NapCat（现问身份不算平台动作）。
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn as_the_owner_she_can_mute_and_kick_an_admin() {
    // 施工 I-97：QQ 允许群主禁言、踢管理员。
    let lines = Lines::new(
        [
            uses("qq_mute", r#"{"seconds":60,"user":40006}"#),
            uses("qq_kick", r#"{"user":40006}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let ranks: &'static [crate::support::answering::Rank] = &[(BOT, "owner"), (QQ_ADMIN, "admin")];
    let (home, mut napcat) = ranked_as(&lines, WITH_ADMIN, ranks).await;
    let (text, _) = asks(
        &home,
        &napcat,
        MANAGER,
        1401,
        json!([plain("禁言管理员")]),
        1,
    )
    .await;
    assert_eq!(
        text,
        sentence("muted", &[("who", "40006"), ("duration", "1m")])
    );
    let banned = next_platform(&mut napcat).await;
    assert_eq!(banned["params"]["user_id"], QQ_ADMIN, "{banned}");
    let (text, _) = asks(
        &home,
        &napcat,
        MANAGER,
        1402,
        json!([plain("把管理员踢了")]),
        2,
    )
    .await;
    assert_eq!(text, sentence("kicked", &[("who", "40006")]));
    let kicked = next_platform(&mut napcat).await;
    assert_eq!(kicked["action"], "set_group_kick", "{kicked}");
    stopped(home).await;
}
