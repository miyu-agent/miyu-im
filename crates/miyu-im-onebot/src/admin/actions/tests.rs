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
