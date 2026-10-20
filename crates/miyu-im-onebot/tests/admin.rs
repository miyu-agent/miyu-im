//! 群管理（施工 Q-1，设计 02 第四节）：真核心拉起真桥、假 NapCat，她照台词说。禁言归群管理：她在这个群是管理员、插件在这个群开着
//! 才做，不然答 `admin-off`、不调 NapCat；秒数照设置的最短、最长（分钟），出了范围答 `bad-seconds` 带这两个时长；禁言工具的开关、
//! 总开关关了的，im 重新登记，之后的回合工具表里没有禁言。施工 Q-2：踢人（能拉黑）同禁言；做成了的禁言、解禁、踢人记进这个群的
//! 管理记录（`<im 的状态目录>/qq/admin/<群号>.jsonl`），带她给的理由；记录开关关了的不记。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 照网页、命令行的样子改系统配置。
async fn set(home: &Home, changes: Value) {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    core.call(
        "q1",
        "config.set",
        json!({"layer": "system", "changes": changes}),
    )
    .await
    .unwrap_or_else(|refused| panic!("config.set 被拒：{refused}"));
}

/// 桥的运行日志里 `tools provided` 有几行（登记了几次）。
fn provided(home: &Home) -> usize {
    std::fs::read_to_string(home.root.state().join("logs").join("im.log"))
        .unwrap_or_default()
        .matches("tools provided")
        .count()
}

#[tokio::test]
async fn mute_keeps_to_the_range_and_to_groups_where_group_admin_is_on() {
    let lines = Lines::new(
        [
            uses("mute", r#"{"seconds":60}"#),
            uses("mute", r#"{"seconds":600}"#),
            uses("mute", r#"{"seconds":600}"#),
            uses("kick", "{}"),
            uses("moderation_log", "{}"),
        ]
        .into_iter()
        .flatten(),
    );
    // 最短 5 分钟（施工 Q-1：设置是分钟）。
    let config = format!("{}qq.admin_mute_min = 5\n", onebot());
    let (home, mut napcat, _) =
        started_ranked(Arc::new(lines.clone()), &rules(), &config, (MEMBERS, RANKS)).await;
    let words = json!([at(MEMBER), plain(" 禁言")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 501, words.clone(), 1).await,
        refused("bad-seconds", &[("min", "5m"), ("max", "30d")])
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 502, words.clone(), 2).await,
        done("muted", &[("who", "小明"), ("duration", "10m")])
    );
    assert_eq!(
        next_platform(&mut napcat).await["params"]["user_id"],
        MEMBER
    );
    // 在这个群关掉群管理（「陌生群」卡上的插件开关，同识人术）：不做，不调 NapCat。
    let mut head = crate::support::head::Head::connect(&home.root).await;
    let off = json!({"card": {"kind": "other_groups"}, "values": {"plugins": {"miyu-im-qq-admin": false}}});
    let saved = head.page_call_with("cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(
        asks(&home, &napcat, MANAGER, 503, words.clone(), 3).await,
        refused("admin-off", &[])
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 504, words.clone(), 4).await,
        refused("admin-off", &[]),
        "踢人同禁言"
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 505, words, 5).await,
        refused("admin-off", &[]),
        "查记录同禁言"
    );
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn mute_needs_her_to_be_an_admin_of_the_group() {
    let lines = Lines::new(
        [
            uses("mute", r#"{"seconds":60}"#),
            uses("kick", "{}"),
            uses("moderation_log", "{}"),
        ]
        .into_iter()
        .flatten(),
    );
    // 她是普通成员。
    let ranks = [(OWNER, "owner")];
    let (home, mut napcat, _) = started_ranked(
        Arc::new(lines.clone()),
        &rules(),
        &onebot(),
        (MEMBERS, &ranks),
    )
    .await;
    let words = json!([at(MEMBER), plain(" 禁言")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 601, words.clone(), 1).await,
        refused("admin-off", &[])
    );
    assert!(napcat.asked().contains(&BOT), "问了她自己的身份");
    assert_eq!(
        asks(&home, &napcat, MANAGER, 602, words.clone(), 2).await,
        refused("admin-off", &[]),
        "踢人同禁言"
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 603, words, 3).await,
        refused("admin-off", &[]),
        "查记录同禁言"
    );
    no_platform(&mut napcat);
    assert!(records(&home).is_empty(), "没做成的不记");
    stopped(home).await;
}

#[tokio::test]
async fn switching_the_mute_tool_off_takes_it_out_of_her_tools() {
    let lines = Lines::new([Line::says("好。"), Line::says("好。")]);
    let (home, napcat) = up(&lines).await;
    let words = json!([plain("在吗")]);
    let seq = says(&home, &napcat, (MEMBER, "member"), 701, words.clone()).await;
    respond(&home, &venue(), "open-701", &[json!(seq)]).await;
    until_events(&home.root, &venue(), |events| {
        !of_kind(events, "turn.ended").is_empty()
    })
    .await;
    assert_eq!(
        platform_tools(&lines.requests()[0]),
        ["mute", "poke", "recall"]
    );
    // 关掉禁言工具：im 重新登记（日志多一行 `tools provided`），之后的回合没有禁言。
    let before = provided(&home);
    set(
        &home,
        json!([
            {"key": "im.qq.admin_mute", "value": false},
            {"key": "im.qq.admin_log", "value": false},
        ]),
    )
    .await;
    within("重新登记", async {
        while provided(&home) <= before {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let seq = says(&home, &napcat, (MEMBER, "member"), 702, words).await;
    respond(&home, &venue(), "open-702", &[json!(seq)]).await;
    until_events(&home.root, &venue(), |events| {
        of_kind(events, "turn.ended").len() >= 2
    })
    .await;
    assert_eq!(platform_tools(&lines.requests()[1]), ["poke", "recall"]);
    let named = |index: usize| -> Vec<String> {
        lines.requests()[index]
            .tools
            .iter()
            .map(|tool| tool.name.clone())
            .collect()
    };
    assert!(named(0).contains(&"moderation_log".to_string()));
    assert!(
        !named(1).contains(&"moderation_log".to_string()),
        "查记录工具关了不登记"
    );
    stopped(home).await;
}

/// 这个群的管理记录（施工 Q-2）：照先后，一行一条。
fn records(home: &Home) -> Vec<Value> {
    let path = home
        .root
        .state()
        .join("packages/im/qq/admin")
        .join(format!("{GROUP}.jsonl"));
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("一行一条 JSON"))
        .collect()
}

#[tokio::test]
async fn kick_and_mute_are_done_and_recorded_with_her_reason() {
    let lines = Lines::new(
        [
            uses("mute", r#"{"seconds":600,"reason":"刷屏"}"#),
            uses("mute", r#"{"seconds":0}"#),
            uses("kick", r#"{"block":true,"reason":"发广告"}"#),
            uses("kick", r#"{"user":40002}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    let words = json!([at(MEMBER), plain(" 管一下")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 801, words.clone(), 1).await,
        done("muted", &[("who", "小明"), ("duration", "10m")])
    );
    next_platform(&mut napcat).await;
    assert_eq!(
        asks(&home, &napcat, MANAGER, 802, words.clone(), 2).await,
        done("unmuted", &[("who", "小明")])
    );
    next_platform(&mut napcat).await;
    // 踢 @ 的那个人，拉黑；再踢她给的号（没在群里说过话：名字没缓存，结果写号、记录不带名字）。
    assert_eq!(
        asks(&home, &napcat, MANAGER, 803, words.clone(), 3).await,
        done("kicked-blocked", &[("who", "小明")])
    );
    let kicked = next_platform(&mut napcat).await;
    assert_eq!(kicked["action"], "set_group_kick", "{kicked}");
    assert_eq!(
        kicked["params"],
        json!({"group_id": GROUP, "user_id": MEMBER, "reject_add_request": true})
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 804, words, 4).await,
        done("kicked", &[("who", "40002")])
    );
    let kicked = next_platform(&mut napcat).await;
    assert_eq!(kicked["params"]["reject_add_request"], false, "{kicked}");
    // 管理记录：四条，照先后；AI 做的，带她给的理由、那时的名字。
    let written = within("记下四条", async {
        loop {
            let written = records(&home);
            if written.len() >= 4 {
                return written;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let shape: Vec<Value> = written
        .iter()
        .map(|record| {
            let mut record = record.clone();
            assert!(record["at"].as_u64().is_some_and(|at| at > 0), "{record}");
            record.as_object_mut().expect("是对象").remove("at");
            record
        })
        .collect();
    let ai = json!({"kind": "ai"});
    assert_eq!(
        shape,
        [
            json!({"action": "mute", "user": MEMBER, "name": "小明", "by": ai, "seconds": 600, "reason": "刷屏"}),
            json!({"action": "unmute", "user": MEMBER, "name": "小明", "by": ai}),
            json!({"action": "kick", "user": MEMBER, "name": "小明", "by": ai, "reason": "发广告", "block": true}),
            json!({"action": "kick", "user": IMPOSTOR, "by": ai}),
        ]
    );
    stopped(home).await;
}

#[tokio::test]
async fn nothing_is_recorded_when_records_are_off_and_kick_can_be_switched_off() {
    let lines = Lines::new(
        [
            uses("mute", r#"{"seconds":60}"#),
            uses("moderation_log", r#"{"view":"ranking"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let config = format!(
        "{}qq.admin_records = false\nqq.admin_kick = false\nqq.admin_summary = false\n",
        onebot()
    );
    let (home, mut napcat, _) =
        started_ranked(Arc::new(lines.clone()), &rules(), &config, (MEMBERS, RANKS)).await;
    let words = json!([at(MEMBER), plain(" 禁言")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 901, words.clone(), 1).await,
        done("muted", &[("who", "小明"), ("duration", "1m")])
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 902, words, 2).await,
        refused("ranking-off", &[]),
        "成员汇总关了没有排行榜"
    );
    next_platform(&mut napcat).await;
    assert!(records(&home).is_empty(), "记录关了不记");
    let tools: Vec<String> = lines.requests()[0]
        .tools
        .iter()
        .map(|tool| tool.name.clone())
        .collect();
    assert!(tools.contains(&"mute".to_string()), "{tools:?}");
    assert!(
        !tools.contains(&"kick".to_string()),
        "踢人工具关了不登记：{tools:?}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn the_moderation_log_lists_records_and_a_ranking() {
    let lines = Lines::new(
        [
            uses("moderation_log", "{}"),
            uses("mute", r#"{"seconds":600,"reason":"刷屏"}"#),
            uses("mute", r#"{"seconds":0}"#),
            uses("moderation_log", "{}"),
            uses("moderation_log", r#"{"view":"ranking","period":"week"}"#),
            uses("moderation_log", r#"{"user":40002}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    let words = json!([at(MEMBER), plain(" 查一下")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1001, words.clone(), 1).await,
        done("log-empty", &[])
    );
    asks(&home, &napcat, MANAGER, 1002, words.clone(), 2).await;
    next_platform(&mut napcat).await;
    asks(&home, &napcat, MANAGER, 1012, words.clone(), 3).await;
    next_platform(&mut napcat).await;
    within("记下了", async {
        while records(&home).len() < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let (events, state) = asks(&home, &napcat, MANAGER, 1003, words.clone(), 4).await;
    assert_eq!(state, "ok");
    let lines: Vec<&str> = events.trim_end().lines().collect();
    let template = sentence("log-events", &[("shown", "2"), ("total", "2")]);
    assert_eq!(lines[0], template.trim_end(), "{events}");
    assert!(
        lines[1].ends_with(" unmuted 小明 (40001); by you"),
        "新的在前：{events}"
    );
    assert!(
        lines[2].ends_with(" muted 小明 (40001) for 10 minutes; by you; reason: 刷屏"),
        "{events}"
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1004, words.clone(), 5).await,
        (
            format!(
                "{}\n1. 小明 (40001): muted 1 time for 10 minutes; last reason: 刷屏",
                sentence("log-ranking", &[("period", "week")])
            ),
            "ok".to_string()
        )
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1005, words, 6).await,
        done("log-empty", &[]),
        "只看这个人：他没有记录"
    );
    stopped(home).await;
}
