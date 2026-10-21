//! 群管理的两块界面（施工 Q-6）：内容照核心的界面块查得过，禁言的状态，按动作筛、新的在前。

use miyu_client::widgets::{WidgetPlace, check, localize};

use super::*;
use crate::admin::groups::View;

const NOW: u64 = 10_000_000;

fn record(at: u64, action: Action, user: i64, seconds: Option<u64>) -> Record {
    Record {
        at,
        action,
        user,
        name: Some(format!("u{user}")),
        by: By::Ai,
        seconds,
        reason: Some("刷屏".to_string()),
        block: false,
        verdict: None,
        comment: None,
    }
}

fn records() -> Vec<Record> {
    vec![
        record(NOW - 5000, Action::Mute, 7, Some(600)),
        record(NOW - 4000, Action::Mute, 8, Some(600)),
        record(NOW - 3900, Action::Unmute, 8, None),
        record(NOW - 100, Action::Mute, 9, Some(600)),
        record(NOW - 50, Action::Kick, 10, None),
    ]
}

/// 一段照中文挑好字（`localize` 认整块：包成一块再挑）。
fn zh(segment: &Value) -> Value {
    localize(&json!([segment]), "zh")[0].clone()
}

/// 面板的一项照中文挑好字。
fn zh_section(item: &Value) -> Value {
    zh(&json!({"type": "sections", "items": [item]}))["items"][0].clone()
}

#[test]
fn the_panel_and_a_section_pass_the_cores_widget_check() {
    let settings = Group::default();
    let panel = panel(31, &settings, &records(), true, NOW);
    assert_eq!(check(WidgetPlace::Page, &[], &panel), Ok(()), "{panel}");
    let whole = json!([{"type": "sections", "items": [
        section((31, Some("测试群")), Some(true), (&settings, &records()), (true, NOW)),
        section((32, None), None, (&settings, &[]), (false, NOW)),
    ]}]);
    assert_eq!(check(WidgetPlace::Page, &[], &whole), Ok(()), "{whole}");
}

#[test]
fn mutes_say_whether_they_are_on_lifted_or_over() {
    let list = zh(&events(&records(), "mute", NOW, 10));
    let states: Vec<(&str, &str)> = list["items"]
        .as_array()
        .expect("是列表")
        .iter()
        .map(|item| {
            (
                item["title"].as_str().unwrap_or_default(),
                item["state"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            ("禁言 u9 10 分钟", "doing"),
            ("禁言 u8 10 分钟", "done"),
            ("禁言 u7 10 分钟", "off"),
        ],
        "只看禁言，新的在前"
    );
    let kicks = zh(&events(&records(), "kick", NOW, 10));
    assert_eq!(kicks["items"][0]["title"], "踢出 u10");
    assert_eq!(kicks["items"][0]["detail"], "AI · 刷屏");
    assert_eq!(
        zh(&events(&[], "all", NOW, 10))["type"],
        "text",
        "没有的一段淡字"
    );
}

#[test]
fn a_section_follows_the_view_the_role_and_the_clear_step() {
    let mut settings = Group {
        view: View {
            action: "kick".to_string(),
            period: "month".to_string(),
        },
        ..Group::default()
    };
    let shown = zh_section(&section(
        (31, Some("测试群")),
        Some(false),
        (&settings, &records()),
        (true, NOW),
    ));
    assert_eq!(shown["badge"]["text"], "不是管理员");
    assert_eq!(shown["state"], "warn");
    let blocks = shown["blocks"].as_array().expect("是数组");
    assert_eq!(blocks[0]["fields"][0]["value"], "kick");
    assert_eq!(blocks[0]["fields"][1]["value"], "month");
    assert_eq!(
        blocks[2]["items"].as_array().map(Vec::len),
        Some(1),
        "只看踢人"
    );
    let clear = |shown: &Value| {
        shown["blocks"]
            .as_array()
            .and_then(|blocks| blocks.last())
            .map(|last| last["items"][0]["label"].clone())
    };
    assert_eq!(clear(&shown), Some(json!("清空这个群的记录")));
    settings.clearing = Some(NOW - 61);
    let stale = zh_section(&section(
        (31, None),
        Some(true),
        (&settings, &records()),
        (false, NOW),
    ));
    assert_eq!(
        clear(&stale),
        Some(json!("清空这个群的记录")),
        "点过超过一分钟的从头来"
    );
    settings.clearing = Some(NOW - 10);
    let asked = zh_section(&section(
        (31, None),
        Some(true),
        (&settings, &records()),
        (false, NOW),
    ));
    assert_eq!(clear(&asked), Some(json!("确认清空（一分钟内再点一次）")));
    assert!(
        asked["blocks"]
            .as_array()
            .is_some_and(|blocks| blocks.len() == 4),
        "成员汇总关了没有排行榜：{asked}"
    );
}

#[test]
fn lengths_are_said_in_each_language() {
    assert_eq!(
        lengths(5400),
        [
            "1 小时 30 分钟".to_string(),
            "1 hour 30 minutes".to_string(),
            "1時間30分".to_string()
        ]
    );
    assert_eq!(lengths(30)[0], "1 分钟");
    assert_eq!(lengths(2 * 86_400)[2], "2日");
}

#[test]
fn a_ranking_leaves_out_what_is_zero_and_mutes_say_their_state() {
    let list = zh(&ranking(&records(), Period::All, NOW, 10));
    let details: Vec<&str> = list["items"]
        .as_array()
        .expect("是列表")
        .iter()
        .map(|item| item["detail"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        details,
        [
            "禁言 1 次，共 10 分钟",
            "禁言 1 次，共 10 分钟",
            "禁言 1 次，共 10 分钟",
            "踢出 1 次"
        ]
    );
    let events = zh(&events(&records(), "mute", NOW, 10));
    assert_eq!(
        events["items"][0]["right"][0],
        json!({"text": "禁言中", "tone": "warn"})
    );
    assert_eq!(
        events["items"][1]["right"][0],
        json!({"text": "已解禁", "tone": "good"})
    );
    assert_eq!(
        events["items"][2]["right"][0],
        json!({"text": "已过期", "tone": "dim"})
    );
}

#[test]
fn a_mute_overridden_by_a_later_one_is_not_shown_as_on() {
    // 施工 I-17：同一个人禁言两次，前一条以前也写「禁言中」。
    let later = record(NOW - 50, Action::Mute, 7, Some(600));
    let both = [record(NOW - 100, Action::Mute, 7, Some(600)), later];
    let list = zh(&events(&both, "mute", NOW, 10));
    let said: Vec<(&str, &str)> = list["items"]
        .as_array()
        .expect("是列表")
        .iter()
        .map(|item| {
            (
                item["state"].as_str().unwrap_or_default(),
                item["right"][0]["text"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(said, [("doing", "禁言中"), ("off", "已覆盖")], "新的在前");
    assert_eq!(
        list["items"][0]["detail"], "AI · 刷屏",
        "不记谁让她做的（施工 I-20）"
    );
    let raw = json!([events(&both, "mute", NOW, 10)]);
    assert_eq!(check(WidgetPlace::Page, &[], &raw), Ok(()), "{raw}");
}
