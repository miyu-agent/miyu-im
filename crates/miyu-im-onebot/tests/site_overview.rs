//! 会话浮窗的「概况」（施工 I-7 上补）：真核心拉起真桥、假 NapCat，群里来消息、她回了以后，页面经通道调 `cards.overview`：名单里
//! 的群照它自己的数和最近判断（谁、正文的开头、回了、为什么），「陌生群」是名单以外的群并起来，私聊的没有判断。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::browser::{Channel, call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 名单里的群。
const LISTED: i64 = 777;

/// 名单以外的群。
const OTHER: i64 = 666;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 小林在群 `group` 里 @ 她，平台编号是 `message`。
fn called(group: i64, message: i64, text: &str) -> Value {
    group_frame(
        group,
        LIN,
        message,
        json!([at(BOT), plain(text)]),
        ("小林", "lin"),
    )
}

/// 一张卡的概况，等到最近判断有 `least` 笔。
async fn overview(open: &mut Channel, card: Value, least: usize) -> Value {
    within("判断记下", async {
        let mut id = 100;
        loop {
            id += 1;
            let answer = call(open, id, "cards.overview", json!({"card": card})).await;
            let result = answer["result"].clone();
            if result["recent"]
                .as_array()
                .is_some_and(|recent| recent.len() >= least)
            {
                return result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_overview_counts_today_and_says_why_each_message_was_answered() {
    let script = Script::new([Play::Says("在的。")]);
    let (home, mut napcat, _) = started(&script, "", "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |status| {
            status["napcat"]["connected"] == true && status["web"].is_u64()
        })
        .await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    let added = call(
        &mut open,
        1,
        "cards.add",
        json!({"value": LISTED.to_string()}),
    )
    .await;
    assert_eq!(
        added["result"],
        json!({"id": LISTED.to_string()}),
        "{added}"
    );

    napcat.send(called(LISTED, 1, " 在吗"));
    assert_eq!(napcat.group_reply(LISTED).await, "在的");
    // 「陌生群」关掉（测试的系统配置里开着）。
    let off = json!({"card": {"kind": "other_groups"}, "values": {"on": false}});
    let saved = call(&mut open, 4, "cards.save_one", off).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    napcat.send(called(OTHER, 2, " 有人吗"));
    let listed = overview(&mut open, json!({"kind": "group", "id": LISTED}), 1).await;
    let note = &listed["recent"][0];
    assert_eq!(
        (&note["who"], &note["text"], &note["outcome"]),
        (&json!("小林"), &json!("@米尤 在吗"), &json!("reply")),
        "{listed}"
    );
    // 出厂被叫时也过判断；测试里没有判断的模型：判不了，冲她来的照回。
    assert_eq!(
        note["why"],
        json!({"reason": "unjudged", "trigger": "direct"}),
        "{listed}"
    );
    assert!(note["at"].as_u64().is_some_and(|at| at > 0), "{note}");
    let today = &listed["today"];
    assert_eq!(
        (&today["received"], &today["replied"]),
        (&json!(1), &json!(1)),
        "{listed}"
    );

    // 「陌生群」：名单以外的群并起来，关着的只记下。
    let others = overview(&mut open, json!({"kind": "other_groups"}), 1).await;
    assert_eq!(
        others["recent"].as_array().map(Vec::len),
        Some(1),
        "{others}"
    );
    assert_eq!(
        others["recent"][0]["why"],
        json!({"reason": "off"}),
        "{others}"
    );
    assert_eq!(others["recent"][0]["outcome"], "record", "{others}");
    assert_eq!(others["today"]["recorded"], 1, "{others}");
    assert_eq!(others["today"]["received"], 1, "名单里的不算进来：{others}");

    // 私聊的：今天的数照算，没有判断。
    let private = call(
        &mut open,
        2,
        "cards.overview",
        json!({"card": {"kind": "other_privates"}}),
    )
    .await;
    assert_eq!(private["result"]["recent"], json!([]), "{private}");
    assert_eq!(private["result"]["today"]["received"], 0, "{private}");
    // 卡写错的拒。
    let bad = call(
        &mut open,
        3,
        "cards.overview",
        json!({"card": {"kind": "x"}}),
    )
    .await;
    assert_eq!(bad["error"]["reason"], "method_failed", "{bad}");

    // 统计页（施工 I-14）：两个群各收一条，为什么回、为什么没回带上判断的结论（判不了照回的是 unjudged-reply）。
    let stats = call(&mut open, 5, "stats", json!({"period": "today"})).await;
    let messages = &stats["result"]["messages"];
    assert_eq!(
        (&messages["now"]["received"], &messages["now"]["replied"]),
        (&json!(2), &json!(1)),
        "{stats}"
    );
    assert_eq!(
        messages["reasons"],
        json!({"unjudged-reply": 1, "off": 1}),
        "{stats}"
    );
    let hours: u64 = messages["hours"]
        .as_array()
        .map(|hours| hours.iter().filter_map(Value::as_u64).sum())
        .unwrap_or_default();
    assert_eq!(hours, 2, "{stats}");
    assert!(stats["result"]["running"]["uptime"].is_u64(), "{stats}");
    // 不认的段照今天。
    let wrong = call(&mut open, 6, "stats", json!({"period": "year"})).await;
    assert_eq!(wrong["result"]["messages"]["now"]["received"], 2, "{wrong}");
    stopped(home).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn called_ones_skip_the_judge_and_listed_people_skip_it_when_talking_to_her() {
    // 施工 I-7 中三：卡上 `rules` 关掉被叫时也过判断，叫她的直接回；小林在跳过判断的人里，没叫她的（续聊）只记下。
    let script = Script::new([Play::Says("在的。"), Play::Says("嗯。")]);
    let (home, mut napcat, _) = started(&script, "", "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |status| {
            status["napcat"]["connected"] == true && status["web"].is_u64()
        })
        .await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    let added = call(
        &mut open,
        1,
        "cards.add",
        json!({"value": LISTED.to_string()}),
    )
    .await;
    assert_eq!(
        added["result"],
        json!({"id": LISTED.to_string()}),
        "{added}"
    );
    let card = json!({"kind": "group", "id": LISTED});
    let rules = json!({"rules": {"chatty.judge_called": false}});
    let saved = call(
        &mut open,
        2,
        "cards.save_one",
        json!({"card": card, "values": rules}),
    )
    .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");

    napcat.send(called(LISTED, 1, " 在吗"));
    assert_eq!(napcat.group_reply(LISTED).await, "在的");
    let first = overview(&mut open, card.clone(), 1).await;
    assert_eq!(
        first["recent"][0]["why"],
        json!({"reason": "called"}),
        "{first}"
    );

    let skip = json!({"rules": {"chatty.skip_users": [format!("qq:{LIN}")]}});
    let saved = call(
        &mut open,
        3,
        "cards.save_one",
        json!({"card": card, "values": skip}),
    )
    .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    // 她刚回了小林：小林接着说是续聊，照说要判；在跳过判断的人里，直接回（施工 I-16，项目主人 10-20 定）。
    let follow = group_frame(
        LISTED,
        LIN,
        2,
        json!([plain("那我先走了")]),
        ("小林", "lin"),
    );
    napcat.send(follow);
    assert_eq!(napcat.group_reply(LISTED).await, "嗯");
    let second = overview(&mut open, card, 2).await;
    assert_eq!(
        second["recent"][0]["why"],
        json!({"reason": "continued"}),
        "{second}"
    );
    assert_eq!(second["recent"][0]["outcome"], "reply", "{second}");
    stopped(home).await;
}
