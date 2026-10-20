//! 群管理（施工 Q-1，设计 02 第四节）：真核心拉起真桥、假 NapCat，她照台词说。禁言归群管理：她在这个群是管理员、插件在这个群开着
//! 才做，不然答 `admin-off`、不调 NapCat；秒数照设置的最短、最长（分钟），出了范围答 `bad-seconds` 带这两个时长；禁言工具的开关、
//! 总开关关了的，im 重新登记，之后的回合工具表里没有禁言。

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
        asks(&home, &napcat, MANAGER, 503, words, 3).await,
        refused("admin-off", &[])
    );
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn mute_needs_her_to_be_an_admin_of_the_group() {
    let lines = Lines::new(uses("mute", r#"{"seconds":60}"#));
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
        asks(&home, &napcat, MANAGER, 601, words, 1).await,
        refused("admin-off", &[])
    );
    assert!(napcat.asked().contains(&BOT), "问了她自己的身份");
    no_platform(&mut napcat);
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
    set(&home, json!([{"key": "im.qq.admin_mute", "value": false}])).await;
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
    stopped(home).await;
}
