//! 时不时看一眼（施工 I-89，设计 05）：真核心照开关拉起真桥、假 NapCat，`bridge.json` 的 `glance_seconds` 改成 2 秒，经网页的样子存卡片。
//! 名单里的群卡打开看一眼：核心的定时到点，上次以后有没人叫她的新消息的开一轮（看一眼那一句接在请求最末尾，施工 I-243），她照剧本说的发回群里；那一轮
//! 结束了照间隔再排；没有新消息的不开、照间隔再排；卡片关了删掉定时，再来新消息也不看。睡着的不开，顺延到醒。私聊（I-89 下）到点
//! 不看有没有新消息，开一轮只带事实；她用 `qq_glance` 定了下一次的照她的，那一轮结束不再照出厂的间隔排。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20005;

/// 名单里的群。
const GROUP: i64 = 778;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 两个间隔多一点：照间隔再排的那一次到过点了。
const TWO_GLANCES: Duration = Duration::from_secs(5);

/// 小林在群里随口说的一句（没叫她），平台编号是 `message`。
fn chat(message: i64, text: &str) -> Value {
    group_frame(GROUP, LIN, message, json!([plain(text)]), ("小林", "lin"))
}

/// 存卡片：这个群的卡只写看一眼开没开。
async fn glance(head: &mut Head, on: bool) {
    let cards = json!({"groups": [{"id": GROUP, "set": {"glance": on}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
}

/// 这个群的会话开过几轮。
fn turns(home: &Home, venue: &str) -> usize {
    of_kind(&venue_events(&home.root, venue), "turn.started").len()
}

#[tokio::test]
async fn a_glance_opens_on_new_messages_only_and_stops_when_turned_off() {
    let script = Script::new([Play::Says("在看呢"), Play::Says("又来了")]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:group:{GROUP}");
    // 1：没人叫她：只记下，群会话有了。
    napcat.send(chat(1, "今天吃什么"));
    until_event(&home.root, &venue, |event| {
        event["kind"] == "ext.im.chat.decided"
    })
    .await;
    // 打开看一眼：存了当场排上定时，从这一刻起来的才算新的。
    glance(&mut head, true).await;
    napcat.send(chat(2, "有人吗"));
    assert_eq!(napcat.group_reply(GROUP).await, "在看呢");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 1
    })
    .await;
    let opened = of_kind(&events, "turn.started");
    assert_eq!(opened.len(), 1, "{opened:#?}");
    // 那一轮的 `cause` 是看一眼的命令编号：投影照它认出这一轮不自动引用、@（施工 I-187）。
    assert!(
        opened[0]["cause"]
            .as_str()
            .is_some_and(|cause| cause.starts_with("glance/")),
        "{opened:#?}"
    );
    // 群里看一眼的那一句接在请求最末尾、说话方式后面（施工 I-243），不是事实、不进历史。
    let requests = script.requests();
    let tail = requests[0].1.tail.as_deref().unwrap_or_default();
    assert!(
        tail.starts_with("<context-style-note>") && tail.ends_with("</glance>"),
        "说话方式在前、看一眼在最后：{tail:?}"
    );
    let messages = serde_json::to_string(&requests[0].1.messages).expect("写得成");
    assert!(!messages.contains("<glance>"), "不进历史：{messages}");
    // 再到点没有新消息：不开，照间隔再排。
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 1, "没有新消息不请求模型");
    let log = run_log(&home.root);
    assert!(log.matches("glance armed").count() >= 3, "{log}");
    // 3：又来一句：下一次到点开第二轮。
    napcat.send(chat(3, "人呢"));
    assert_eq!(napcat.group_reply(GROUP).await, "又来了");
    until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 2
    })
    .await;
    // 关掉：对账删掉定时，再来新消息也不看。
    glance(&mut head, false).await;
    let log = until_log(&home, "cancelled=1").await;
    assert!(log.contains("glances reconciled"), "{log}");
    napcat.send(chat(4, "走了"));
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 2, "关了不再看");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

#[tokio::test]
async fn a_card_caps_how_many_new_messages_a_glance_hands_over() {
    // 施工 I-175（项目主人 11-05）：卡上「看一眼带几条」写 2，到点有 3 条没回的新消息，这一轮只交最后 2 条。
    let script = Script::new([Play::Says("看到了")]);
    let tuned = json!({"glance_seconds": 4});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let cards = json!({"groups": [{"id": GROUP, "set": {"glance": true, "glance_records": 2}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    for (message, text) in [(1, "一"), (2, "二"), (3, "三")] {
        napcat.send(chat(message, text));
    }
    assert_eq!(napcat.group_reply(GROUP).await, "看到了");
    let log = until_log(&home, "glance opened").await;
    let opened = log
        .lines()
        .find(|line| line.contains("glance opened"))
        .unwrap_or_default();
    assert!(opened.contains("count=2"), "只交最后 2 条：{opened}");
    stopped(home).await;
}

#[tokio::test]
async fn a_glance_carries_the_task_written_on_the_card_and_other_turns_do_not() {
    // 施工 I-196（设计 12 第 2 条，项目主人 11-10）：卡片「看一眼时的额外提示词」写了的，看一眼那一轮带 `<glance-task>` 包着的
    // 那段（照模板转义）；被叫到的那一轮不带。
    use crate::support::speaking::{Line, Lines};
    let lines = Lines::new([Line::says("看到了"), Line::says("在")]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) = started_tuned(Arc::new(lines.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let set = json!({"glance": true, "rules": {"glance_note": "留意有没有人发\"好图\""}});
    let cards = json!({"groups": [{"id": GROUP, "set": set}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    napcat.send(chat(1, "随口一句"));
    assert_eq!(napcat.group_reply(GROUP).await, "看到了");
    let words = json!([at(BOT), plain(" 在吗")]);
    napcat.send(group_frame(GROUP, LIN, 2, words, ("小林", "lin")));
    assert_eq!(napcat.group_reply(GROUP).await, "在");
    let requests = lines.requests();
    let last = |at: usize| {
        serde_json::to_string(requests[at].messages.last().expect("有消息")).expect("写得成")
    };
    let task = "<glance-task>留意有没有人发\\\\u0022好图\\\\u0022</glance-task>";
    assert!(last(0).contains(task), "看一眼那一轮带：{}", last(0));
    let called = last(requests.len() - 1);
    assert!(!called.contains("glance-task"), "被叫到的不带：{called}");
    stopped(home).await;
}

#[tokio::test]
async fn a_glance_never_opens_where_replies_are_off() {
    // 施工 I-108（项目主人 11-02 部署上撞到：会话默认值打开看一眼，「是否回复」关着的群她也去看了、说了话）：回复关着的场所
    // 不排看一眼，到点也不开轮。
    let script = Script::new([Play::Says("不该说")]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:group:{GROUP}");
    let cards = json!({"groups": [{"id": GROUP, "set": {"glance": true, "on": false}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    napcat.send(chat(1, "有人吗"));
    until_event(&home.root, &venue, |event| {
        event["kind"] == "ext.im.chat.decided"
    })
    .await;
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 0, "回复关着，到点也不开轮");
    assert!(napcat.pending().is_none(), "群里什么都不发");
    assert!(
        !run_log(&home.root).contains("glance armed"),
        "回复关着的不排看一眼"
    );
    stopped(home).await;
}

#[tokio::test]
async fn a_card_sets_how_often_and_the_overview_shows_the_glance() {
    // 施工 I-114：卡上「多久看一眼」照它排（出厂 100 秒，卡上 1 分钟，一分钟按 2 秒算）；概况写上一次看了、看到几条，下一次几点。
    let script = Script::new([Play::Says("看到了")]);
    let tuned = json!({"glance_seconds": 100, "glance_minute_seconds": 2});
    let (home, mut napcat, _) = started_tuned(Arc::new(script.clone()), RULES, &tuned, |napcat| {
        napcat.answering(MEMBERS)
    })
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let cards = json!({"groups": [{"id": GROUP, "set": {"glance": true, "glance_minutes": 1}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    // 施工 I-165：卡下面几层的默认值是桥现在用着的间隔（出厂 100 秒，一分钟按 2 秒算就是 50 分钟），页面淡字照它写。
    let got = head.page_call("cards.get").await["result"].clone();
    assert_eq!(
        got["defaults"]["groups"][GROUP.to_string()]["glance_minutes"],
        json!({"value": 50, "from": "factory"}),
        "{got}"
    );
    napcat.send(chat(1, "有人吗"));
    assert_eq!(
        napcat.group_reply(GROUP).await,
        "看到了",
        "照卡上的一分钟（2 秒）看，不等出厂的 100 秒"
    );
    let log = until_log(&home, "by=every").await;
    assert!(log.contains("glance opened"), "{log}");
    let shown = head
        .page_call_with(
            "cards.overview",
            json!({"card": {"kind": "group", "id": GROUP}}),
        )
        .await;
    let glance = &shown["result"]["glance"];
    assert_eq!(glance["count"], 1, "{shown}");
    assert!(glance["last"].is_string(), "{shown}");
    stopped(home).await;
}

#[tokio::test]
async fn a_glance_while_asleep_waits_for_the_wake() {
    let script = Script::new([]);
    let tuned = json!({"glance_seconds": 2});
    let (home, mut napcat, _) =
        started_tuned(Arc::new(script.clone()), &sleepy(), &tuned, |napcat| {
            napcat.answering(MEMBERS)
        })
        .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:group:{GROUP}");
    napcat.send(chat(1, "今天吃什么"));
    until_event(&home.root, &venue, |event| {
        event["kind"] == "ext.im.chat.decided"
    })
    .await;
    glance(&mut head, true).await;
    napcat.send(chat(2, "有人吗"));
    // 到点睡着：顺延到睡觉时段结束，第二个定时排在一个小时左右以后，不是两秒以后。
    let log = until_log(&home, "schedule=t2").await;
    assert!(log.contains("glance after sleep"), "{log}");
    tokio::time::sleep(TWO_GLANCES).await;
    assert!(!run_log(&home.root).contains("schedule=t3"), "{log}");
    assert_eq!(turns(&home, &venue), 0, "睡着的不开");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

/// 白名单成员，有私聊卡。
const FRIEND: i64 = 40015;

#[tokio::test]
async fn a_private_glance_opens_without_new_messages_and_her_tool_sets_the_next() {
    let script = Script::new([
        Play::Says("在呢"),
        Play::calls(&[("qq_glance", r#"{"minutes": 60}"#)]),
        Play::Says("想你了"),
    ]);
    let tuned = json!({"glance_seconds": 2});
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started_tuned_with(
        Arc::new(script.clone()),
        (RULES, &onebot),
        &tuned,
        |napcat| napcat.answering(MEMBERS),
    )
    .await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let venue = format!("qq:{BOT}:private:{FRIEND}");
    // 1：私聊里说一句，她回了：会话有了。
    napcat.send(private_frame(FRIEND, 1, json!([plain("早")])));
    assert_eq!(private_reply(&mut napcat).await, "在呢");
    until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 1
    })
    .await;
    // 打开看一眼：到点没有新消息也开一轮，她定下一次 60 分钟以后，再说一句。
    let cards = json!({"privates": [{"id": FRIEND, "set": {"glance": true}}]});
    let saved = head
        .page_call_with("cards.save", json!({"cards": cards}))
        .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(private_reply(&mut napcat).await, "想你了");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "turn.ended").len() == 2
    })
    .await;
    let text = format!("{events:?}");
    assert!(text.contains("<glance>"), "带一块看一眼的事实：{events:#?}");
    assert!(
        text.contains("Next look in 60 min."),
        "工具答了：{events:#?}"
    );
    // 那一轮结束不再照两秒排：过两个间隔也没有第三轮。
    tokio::time::sleep(TWO_GLANCES).await;
    assert_eq!(turns(&home, &venue), 2, "照她定的 60 分钟");
    let log = run_log(&home.root);
    assert_eq!(log.matches("glance armed").count(), 2, "{log}");
    assert!(napcat.pending().is_none(), "别的什么都不发");
    stopped(home).await;
}

/// 下一个动作是发给 [`FRIEND`] 的 `send_private_msg`：交回里面的字。
async fn private_reply(napcat: &mut Answering) -> String {
    let action = napcat.action().await;
    assert_eq!(action["action"], "send_private_msg", "{action}");
    assert_eq!(action["params"]["user_id"], FRIEND, "{action}");
    action["params"]["message"][0]["data"]["text"]
        .as_str()
        .expect("有字")
        .to_string()
}

/// 系统的场所规则：群的睡觉时间盖住此刻（前后各一个小时，照本机的时区；同 `whitelist.rs`）。
fn sleepy() -> String {
    let now = jiff::Zoned::now();
    let minute = i64::from(now.hour()) * 60 + i64::from(now.minute());
    let clock = |minute: i64| {
        let minute = minute.rem_euclid(24 * 60);
        format!("{:02}:{:02}", minute / 60, minute % 60)
    };
    format!(
        "[[rule]]\nmatch = {{ kind = \"group\" }}\nsleep = \"{}-{}\"\n",
        clock(minute - 60),
        clock(minute + 60),
    )
}

/// 等到运行日志里有 `wanted`：交回那时的运行日志。
async fn until_log(home: &Home, wanted: &str) -> String {
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let log = run_log(&home.root);
        if log.contains(wanted) {
            return log;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到 {wanted}：\n{log}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
