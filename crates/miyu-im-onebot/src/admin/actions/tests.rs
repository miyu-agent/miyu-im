//! 页面调群管理的动作（施工 Q-6）。

use std::sync::Arc;

use miyu_im::chat::Bots;

use super::*;

fn context(name: &str) -> Context {
    let store = std::env::temp_dir().join(format!("miyu-im-actions-{name}-{}", std::process::id()));
    if store.exists() {
        std::fs::remove_dir_all(&store).expect("删得了");
    }
    Context {
        platform: crate::onebot::PLATFORM,
        config: Map::new(),
        store,
        bots: Arc::new(Bots::default()),
    }
}

#[tokio::test]
async fn spam_settings_are_saved_and_wrong_values_refused() {
    let context = context("spam");
    let saved = call(
        &context,
        "admin.spam",
        &json!({"group": 31, "values": {"on": true, "mute": 2}}),
    )
    .await;
    assert_eq!(saved, Ok(json!({"saved": true})));
    let group = groups::read(&context.store).remove("31").expect("存了");
    assert_eq!(
        (group.spam.on, group.spam.window, group.spam.mute),
        (true, 60, 120),
        "没交的不动，分钟换成秒"
    );
    let refused = call(
        &context,
        "admin.spam",
        &json!({"group": 31, "values": {"window": 1, "on": "yes"}}),
    )
    .await;
    let keys: Vec<Value> = refused.expect("答了")["problems"]
        .as_array()
        .expect("有问题")
        .iter()
        .map(|one| one["key"].clone())
        .collect();
    assert_eq!(keys, [json!("on"), json!("window")]);
    assert_eq!(
        groups::read(&context.store)["31"].spam.window,
        60,
        "写错的整张不存"
    );
    let viewed = call(
        &context,
        "admin.view",
        &json!({"group": 31, "values": {"action": "kick"}}),
    )
    .await;
    assert_eq!(viewed, Ok(json!({"saved": true})));
    assert_eq!(groups::read(&context.store)["31"].view.action, "kick");
    assert_eq!(
        call(&context, "admin.spam", &json!({"values": {}})).await,
        Err("bad_params".to_string())
    );
    assert_eq!(
        call(&context, "admin.nope", &json!({})).await,
        Err("unknown_method".to_string())
    );
    std::fs::remove_dir_all(&context.store).expect("删得了");
}

#[tokio::test]
async fn clearing_takes_two_clicks() {
    let context = context("clear");
    let record = records::Record {
        at: 1,
        action: records::Action::Kick,
        user: 7,
        name: None,
        by: records::By::Ai,
        seconds: None,
        reason: None,
        block: false,
        verdict: None,
        comment: None,
    };
    records::append(context.store.clone(), 31, record, 10).await;
    let clear = json!({"group": 31});
    assert_eq!(call(&context, "admin.clear", &clear).await, Ok(json!({})));
    assert_eq!(
        records::read(&context.store, 31).len(),
        1,
        "头一次只记下点了"
    );
    assert!(groups::read(&context.store)["31"].clearing.is_some());
    // 点过超过一分钟：从头来，这一次只记下。
    groups::update(&context.store, 31, |group| {
        group.clearing = Some(records::now() - 61)
    })
    .expect("改得了");
    assert_eq!(call(&context, "admin.clear", &clear).await, Ok(json!({})));
    assert_eq!(
        records::read(&context.store, 31).len(),
        1,
        "过了一分钟的不算"
    );
    assert_eq!(call(&context, "admin.clear", &clear).await, Ok(json!({})));
    assert!(records::read(&context.store, 31).is_empty(), "再点一次清了");
    assert!(groups::read(&context.store)["31"].clearing.is_none());
    std::fs::remove_dir_all(&context.store).expect("删得了");
}

#[tokio::test]
async fn the_panel_is_for_groups_and_the_page_says_when_qq_is_away() {
    let context = context("panel");
    let panel = call(&context, "chat.panel", &json!({"venue": "qq:group:31"}))
        .await
        .expect("答了");
    assert_eq!(panel[0]["type"], "heading");
    assert_eq!(
        call(&context, "chat.panel", &json!({"venue": "qq:private:7"})).await,
        Err("bad_params".to_string())
    );
    let page = call(&context, "admin.page", &json!({}))
        .await
        .expect("答了");
    assert_eq!(page[0]["type"], "text", "没连着的一段字：{page}");
}

#[tokio::test]
async fn the_join_rule_is_saved_and_cleared() {
    let context = context("approval");
    let save = json!({"group": 31, "values": {"rule": "答案是芝麻开门"}});
    assert_eq!(
        call(&context, "admin.approval", &save).await,
        Ok(json!({"saved": true}))
    );
    assert_eq!(
        groups::read(&context.store)["31"].approval.rule,
        "答案是芝麻开门"
    );
    let cleared = json!({"group": 31, "values": {"rule": null}});
    assert_eq!(
        call(&context, "admin.approval", &cleared).await,
        Ok(json!({"saved": true}))
    );
    assert_eq!(groups::read(&context.store)["31"].approval.rule, "");
    let long = json!({"group": 31, "values": {"rule": "很".repeat(2001)}});
    let refused = call(&context, "admin.approval", &long).await.expect("答了");
    assert_eq!(refused["saved"], false, "{refused}");
    std::fs::remove_dir_all(&context.store).expect("删得了");
}

#[test]
fn a_group_is_admin_if_any_of_her_numbers_is() {
    // 施工 I-17：连着的号都看。
    assert_eq!(role([Some(false), Some(true)]), Some(true));
    assert_eq!(role([None, Some(true)]), Some(true));
    assert_eq!(role([Some(false), Some(false)]), Some(false));
    assert_eq!(role([Some(false), None]), None, "有问不到的");
    assert_eq!(role([]), None);
}
