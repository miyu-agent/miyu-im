//! 群的测试共用的（施工 O-22，`onebot.md` 第一条「群消息」「撤回」；O-23 从 `group.rs` 测试挪过来）：照 NapCat 的样子拼群消息、
//! 撤回、禁言（施工 O-25 中）的事件；真核心照开关拉起真桥、系统的场所规则写好、假 NapCat 连上交给任务应答（[`started`]，应答
//! 在 `answering.rs`）；读一个群的会话的事件（[`venue_events`]、[`until_event`]）。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use miyu_http::testkit::Server;
use miyu_session::Models;
use miyu_session::testkit::Script;
use miyu_store::root::DataRoot;
use miyu_tool::Catalog;

#[allow(unused_imports, reason = "几个测试程序各用其中一部分")]
pub use super::answering::{
    Answering, FIRST_SENT, GROUP_NAME, Member, Rank, UNRECALLABLE, unrecallable,
};
pub use super::events::*;
use super::ports::on_free_port;
use super::spawning::{bridge_up, cli, ports_config_with, text};
use super::{BOT, Home, NapCat, TIME, admin_napcat, resources, tuned_resources, within};

/// 正向的等待最多多久：真的程序、真的核心，负载高时慢。
pub const WAIT: Duration = Duration::from_secs(60);

/// 一条群消息事件，照 NapCat 发的样子：群 `group` 里 `user` 发的、编号 `message_id`，群名片 `card`、昵称 `nickname`，时刻是
/// [`TIME`]。`message` 是段的数组或者 CQ 字符串。
pub fn group_frame(
    group: i64,
    user: i64,
    message_id: i64,
    message: Value,
    names: (&str, &str),
) -> Value {
    group_frame_as(group, user, message_id, message, names, "member")
}

/// 同 [`group_frame`]，发的人在群里的身份 `sender.role` 是 `role`（`owner`、`admin`、`member`，施工 O-31）。
pub fn group_frame_as(
    group: i64,
    user: i64,
    message_id: i64,
    message: Value,
    (card, nickname): (&str, &str),
    role: &str,
) -> Value {
    json!({
        "time": TIME,
        "self_id": BOT,
        "post_type": "message",
        "message_type": "group",
        "sub_type": "normal",
        "message_id": message_id,
        "group_id": group,
        "user_id": user,
        "message": message,
        "raw_message": "",
        "font": 14,
        "sender": {"user_id": user, "nickname": nickname, "card": card, "role": role},
    })
}

/// 群里的撤回：群 `group` 里 `user` 发的第 `message_id` 条，`operator` 撤的。
pub fn group_recall(group: i64, user: i64, operator: i64, message_id: i64) -> Value {
    json!({
        "time": TIME,
        "self_id": BOT,
        "post_type": "notice",
        "notice_type": "group_recall",
        "group_id": group,
        "user_id": user,
        "operator_id": operator,
        "message_id": message_id,
    })
}

/// 私聊里的撤回：`user` 撤了自己发的第 `message_id` 条。
pub fn friend_recall(user: i64, message_id: i64) -> Value {
    json!({
        "time": TIME,
        "self_id": BOT,
        "post_type": "notice",
        "notice_type": "friend_recall",
        "user_id": user,
        "message_id": message_id,
    })
}

/// 群 `group` 里 `user` 被禁言（施工 O-25 中，「出站队列」第 7 条）：`sub_type` 是 `ban`、`lift_ban` 这类，`duration` 照给的写
/// （没有的不写这一格）。`user` 是 0 的是全员禁言。
pub fn group_ban(group: i64, user: i64, sub_type: &str, duration: Option<i64>) -> Value {
    let mut frame = json!({
        "time": TIME,
        "self_id": BOT,
        "post_type": "notice",
        "notice_type": "group_ban",
        "sub_type": sub_type,
        "group_id": group,
        "operator_id": 40004,
        "user_id": user,
    });
    if let Some(duration) = duration {
        frame["duration"] = json!(duration);
    }
    frame
}

/// 引用平台编号是 `id` 的那一条（施工 O-31）。
pub fn quote(id: i64) -> Value {
    json!({"type": "reply", "data": {"id": id.to_string()}})
}

/// 一段文字。
pub fn plain(text: &str) -> Value {
    json!({"type": "text", "data": {"text": text}})
}

/// @ 一个号（`all` 是全体）。
pub fn at(qq: impl Into<Value>) -> Value {
    json!({"type": "at", "data": {"qq": qq.into()}})
}

/// 起一个照开关拉起桥的核心：系统的场所规则 `80-test.toml` 写成 `rules`，系统配置的 `[im]` 多写 `onebot`（白名单成员这类），
/// 桥起来、假 NapCat 连上交给任务应答，群成员照 `members`。挑的空端口被别人先占了的换一个再来（`ports.rs`）。交回核心、假
/// NapCat 和 NapCat 的端口（重启以后等桥、再连）。
pub async fn started(
    script: &Script,
    rules: &str,
    onebot: &str,
    members: &[Member],
) -> (Home, Answering, u16) {
    started_with(script, rules, onebot, |napcat| napcat.answering(members)).await
}

/// 同 [`started`]，请求模型照 `models`（施工 O-25 上：她照台词说，[`super::speaking::Lines`]）。
pub async fn started_by(
    models: Arc<dyn Models>,
    rules: &str,
    members: &[Member],
) -> (Home, Answering, u16) {
    up(models, ("", &Value::Null), (rules, ""), "", |napcat| {
        napcat.answering(members)
    })
    .await
}

/// 同 [`started_by`]，系统配置的 `[im]` 多写 `onebot`（白名单成员这类），假 NapCat 问群成员时照 `ranks` 回身份（施工
/// O-31，[`NapCat::ranked`]）。
pub async fn started_ranked(
    models: Arc<dyn Models>,
    rules: &str,
    onebot: &str,
    (members, ranks): (&[Member], &[Rank]),
) -> (Home, Answering, u16) {
    up(models, ("", &Value::Null), (rules, ""), onebot, |napcat| {
        napcat.ranked(members, ranks)
    })
    .await
}

/// 同 [`started_by`]，`bridge.json` 照 `tuned` 改那几格（施工 O-25 中，[`Home::spawning_tuned`]），假 NapCat 连上以后交给
/// `answer` 去应答（[`NapCat::refusing`] 这类）。
pub async fn started_tuned(
    models: Arc<dyn Models>,
    rules: &str,
    tuned: &Value,
    answer: impl FnOnce(NapCat) -> Answering,
) -> (Home, Answering, u16) {
    up(models, ("", tuned), (rules, ""), "", answer).await
}

/// 同 [`started`]，假 NapCat 连上以后交给 `answer` 去应答（[`NapCat::reversing`] 这类）。
pub async fn started_with(
    script: &Script,
    rules: &str,
    onebot: &str,
    answer: impl FnOnce(NapCat) -> Answering,
) -> (Home, Answering, u16) {
    up(
        Arc::new(script.clone()),
        ("", &Value::Null),
        (rules, ""),
        onebot,
        answer,
    )
    .await
}

/// 同 [`started`]，判官那一次（`model.call`）发到假服务器 `judge`（施工 O-23 下，[`super::judge`]）：她的回合照剧本 `script`。
/// `words` 不空的写成系统的违规词表（桥起来以前写好，不等重读）。
pub async fn started_judged(
    script: &Script,
    judge: &Server,
    (rules, words): (&str, &str),
    onebot: &str,
    members: &[Member],
) -> (Home, Answering, u16) {
    let models = super::judge::models(script);
    started_with_models(models, judge, (rules, words), onebot, members).await
}

/// 同 [`started_judged`]，核心起来以前照 `install` 装别的包（施工 O-43 末：挂判断打分挂接点的测试包）。
pub async fn started_judged_installed(
    script: &Script,
    judge: &Server,
    (rules, words): (&str, &str),
    members: &[Member],
    install: impl Fn(&DataRoot) + Clone,
) -> (Home, Answering, u16) {
    let more = super::judge::config(judge);
    up_installed(
        super::judge::models(script),
        (&more, &Value::Null),
        (rules, words),
        "",
        |napcat| napcat.answering(members),
        (install, Catalog::default()),
    )
    .await
}

/// 同 [`started_by`]，工具目录里有基础系统的 `show`（施工 O-46，[`super::hooked::showing`]），假 NapCat 照 `answer` 回。
pub async fn started_showing(
    models: Arc<dyn Models>,
    rules: &str,
    answer: impl FnOnce(NapCat) -> Answering,
) -> (Home, Answering, u16) {
    up_installed(
        models,
        ("", &Value::Null),
        (rules, ""),
        "",
        answer,
        (|_: &DataRoot| {}, super::hooked::showing()),
    )
    .await
}

/// 同 [`started_judged`]，请求模型照 `models`（[`super::judge::holding`] 这类）。
pub async fn started_with_models(
    models: Arc<dyn Models>,
    judge: &Server,
    (rules, words): (&str, &str),
    onebot: &str,
    members: &[Member],
) -> (Home, Answering, u16) {
    let more = super::judge::config(judge);
    up(
        models,
        (&more, &Value::Null),
        (rules, words),
        onebot,
        |napcat| napcat.answering(members),
    )
    .await
}

/// 同 [`started_judged`]，问群成员照 `ranks` 回身份（施工 Q-7：入群审批要她是管理员）。
pub async fn started_judged_ranked(
    script: &Script,
    judge: &Server,
    rules: &str,
    (members, ranks): (&[Member], &[Rank]),
) -> (Home, Answering, u16) {
    let more = super::judge::config(judge);
    up(
        super::judge::models(script),
        (&more, &Value::Null),
        (rules, ""),
        "",
        |napcat| napcat.ranked(members, ranks),
    )
    .await
}

/// 起核心、拉起桥、连上假 NapCat：请求模型照 `models`，系统配置在端口、`[im]` 后面再接 `more`，`bridge.json` 照 `tuned`
/// 改（[`Home::spawning_tuned`]）；系统的场所规则写成 `rules`，`words` 不空的写成系统的违规词表。
async fn up(
    models: Arc<dyn Models>,
    (more, tuned): (&str, &Value),
    (rules, words): (&str, &str),
    onebot: &str,
    answer: impl FnOnce(NapCat) -> Answering,
) -> (Home, Answering, u16) {
    up_installed(
        models,
        (more, tuned),
        (rules, words),
        onebot,
        answer,
        (|_: &DataRoot| {}, Catalog::default()),
    )
    .await
}

/// 同 [`started_installed`]，`bridge.json` 照 `tuned` 改几项（施工 O-51：排着的过期）。
pub async fn started_installed_tuned(
    script: &Script,
    tuned: &Value,
    members: &[Member],
    install: impl Fn(&DataRoot) + Clone,
) -> (Home, Answering, u16) {
    started_installed_tuned_by(Arc::new(script.clone()), tuned, members, install).await
}

/// 同 [`started_installed_tuned`]，她照 `models` 说（施工 I-33：要压着某一句的照台词 `Lines` 说）。
pub async fn started_installed_tuned_by(
    models: Arc<dyn Models>,
    tuned: &Value,
    members: &[Member],
    install: impl Fn(&DataRoot) + Clone,
) -> (Home, Answering, u16) {
    up_installed(
        models,
        ("", tuned),
        ("", ""),
        "",
        |napcat| napcat.answering(members),
        (install, Catalog::default()),
    )
    .await
}

/// 同 [`started_installed`]，系统配置的 `[im]` 照 `onebot` 写（施工 O-51：白名单成员的私聊）。
pub async fn started_installed_with(
    script: &Script,
    onebot: &str,
    members: &[Member],
    install: impl Fn(&DataRoot) + Clone,
) -> (Home, Answering, u16) {
    up_installed(
        Arc::new(script.clone()),
        ("", &Value::Null),
        ("", ""),
        onebot,
        |napcat| napcat.answering(members),
        (install, Catalog::default()),
    )
    .await
}

/// 同 [`started_installed_with`]，请求模型照 `models`（施工 O-57：她照台词边说边调工具，[`super::speaking::Lines`]）。
pub async fn started_installed_by(
    models: Arc<dyn Models>,
    (rules, onebot): (&str, &str),
    members: &[Member],
    install: impl Fn(&DataRoot) + Clone,
) -> (Home, Answering, u16) {
    up_installed(
        models,
        ("", &Value::Null),
        (rules, ""),
        onebot,
        |napcat| napcat.answering(members),
        (install, Catalog::default()),
    )
    .await
}

/// 同 [`started`]，核心起来以前照 `install` 往数据根里装别的包（施工 O-43 中：挂进站挂接点的测试包，`hooked::install`）。
pub async fn started_installed(
    script: &Script,
    rules: &str,
    members: &[Member],
    install: impl Fn(&DataRoot) + Clone,
) -> (Home, Answering, u16) {
    up_installed(
        Arc::new(script.clone()),
        ("", &Value::Null),
        (rules, ""),
        "",
        |napcat| napcat.answering(members),
        (install, Catalog::default()),
    )
    .await
}

/// 同 [`up`]，核心起来以前照 `install` 往数据根里放东西（换端口重来的每一次都放）。
async fn up_installed(
    models: Arc<dyn Models>,
    (more, tuned): (&str, &Value),
    (rules, words): (&str, &str),
    onebot: &str,
    answer: impl FnOnce(NapCat) -> Answering,
    (install, catalog): (impl Fn(&DataRoot) + Clone, Catalog),
) -> (Home, Answering, u16) {
    let (home, listen) = on_free_port(async |listen| {
        let config = format!("{}{more}", ports_config_with(listen, onebot));
        let home = Home::spawning_installed(
            Arc::clone(&models),
            &config,
            |dir| match tuned.as_object() {
                Some(keys) => tuned_resources(dir, keys),
                None => resources(),
            },
            (install.clone(), catalog.clone()),
        );
        let dir = home.root.system().join("venues.d");
        std::fs::create_dir_all(&dir).expect("建得了目录");
        std::fs::write(dir.join("80-test.toml"), rules).expect("写得进");
        // 卡片那一层（施工 O-58 上）：出厂「其他群」关着，这里打开，测试的群照老的默认接话；要测「不接话」的规则自己写。群出厂
        // 仅发送最终消息（施工 O-57），这里照老的每一步都发：测这一格的另外写卡片。
        let cards = home.root.state().join("packages").join("im");
        std::fs::create_dir_all(&cards).expect("建得了目录");
        std::fs::write(
            cards.join("cards.json"),
            r#"{"other_groups": {"on": true, "final_only": false}}"#,
        )
        .expect("写得进");
        if !words.is_empty() {
            let modules = home.root.system().join("modules").join("im");
            std::fs::create_dir_all(&modules).expect("建得了目录");
            std::fs::write(modules.join("moderation.txt"), words).expect("写得进");
        }
        let started = cli(&home.root, &["start"]).await;
        assert_eq!(started.status.code(), Some(0), "{}", text(&started.stderr));
        bridge_up(&home.root, listen).await?;
        Ok((home, listen))
    })
    .await;
    let napcat = answer(admin_napcat(listen).await);
    (home, napcat, listen)
}

/// 停下桥、核心拉起的扩展。
pub async fn stopped(home: Home) {
    // 停不下来的照六十秒报出来（施工 I-7 中三下一：CI 的 macOS 卡在这里到 45 分钟被掐，日志里什么都没有）。
    let stopped = within("桥停下", cli(&home.root, &["stop"])).await;
    assert_eq!(stopped.status.code(), Some(0), "{}", text(&stopped.stderr));
    within("核心停下扩展", home.stop_extensions()).await;
}
