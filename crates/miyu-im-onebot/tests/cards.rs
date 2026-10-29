//! 场所卡片（施工 O-58 上，`onebot.md` 第一条「场所卡片」）：真核心照开关拉起真桥、假 NapCat，经网页的样子调 `cards.get`、
//! `cards.save`。存了的卡当场套：名单里的群回复、「其他群」关着时名单以外的只记下，长文、附加提示词照卡片；写错的卡不存、
//! 交回问题；去句号照通用的配置项（O-58 补二），关了的哪里都留着句号；`cards.get` 交卡片、默认值和来处、她在的群、好友、人格、白名单成员。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群里的别人、白名单成员。
const LIN: i64 = 20004;

/// 名单里的群。
const LISTED: i64 = 777;

/// 名单以外的群。
const OTHER: i64 = 666;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：群都不抽样。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\nchatty = { probability = 0 }\n";

/// 平台编号是 `message` 的那一条判下来的样子。
fn decided(events: &[Value], message: i64) -> Option<Value> {
    let cause = format!("qq:{BOT}:{message}:{TIME}/decided");
    events
        .iter()
        .find(|event| event["kind"] == "ext.im.chat.decided" && event["cause"] == cause)
        .map(|event| event["body"].clone())
}

/// 小林在群 `group` 里 @ 她，平台编号是 `message`。
fn called(group: i64, message: i64) -> Value {
    group_frame(
        group,
        LIN,
        message,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    )
}

/// 存卡片 `cards`：交回整条回应。
async fn save(head: &mut Head, cards: Value) -> Value {
    head.page_call_with("cards.save", json!({"cards": cards}))
        .await
}

/// 盘上的 `cards.json`。
fn on_disk(home: &Home) -> Value {
    let path = home.root.state().join("packages/im/cards.json");
    serde_json::from_slice(&std::fs::read(path).expect("读得出")).expect("是 JSON")
}

#[tokio::test]
async fn saved_cards_decide_where_she_talks_at_once() {
    let script = Script::new([Play::Says("在的。"), Play::Says("来了。")]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    // 名单里一个群，长文照合并转发、阈值 1（回话就打成合并转发，看得出卡片当场生效）；「其他群」没写：关着。
    let cards =
        json!({"groups": [{"id": LISTED, "set": {"long_text": "forward", "long_chars": 1}}]});
    let saved = save(&mut head, cards.clone()).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    assert_eq!(
        on_disk(&home),
        json!({"other_groups": {}, "groups": [{"id": LISTED, "set": {"long_text": "forward", "long_chars": 1}}], "privates": [], "other_privates": {}})
    );
    napcat.send(called(OTHER, 1));
    let other = format!("qq:group:{OTHER}");
    let events = until_events(&home.root, &other, |events| decided(events, 1).is_some()).await;
    let refused = decided(&events, 1).expect("记了");
    assert_eq!(
        (&refused["inbound"], &refused["why"]),
        (&json!("record_only"), &json!("not_allowed")),
        "{refused}"
    );
    napcat.send(called(LISTED, 2));
    let forwarded = napcat.action().await;
    assert_eq!(
        forwarded["action"], "send_group_forward_msg",
        "卡片的长文照合并转发：{forwarded}"
    );
    assert_eq!(
        forwarded["params"]["messages"][0]["data"]["content"][0]["data"]["text"], "在的",
        "去句号照通用的配置项"
    );
    // 打开「其他群」：存了就算，不重启。
    let opened = json!({"other_groups": {"on": true}, "groups": cards["groups"]});
    assert_eq!(
        save(&mut head, opened).await["result"],
        json!({"saved": true})
    );
    napcat.send(called(OTHER, 3));
    assert_eq!(
        napcat.group_reply(OTHER).await,
        "来了",
        "其他群照出厂去句号"
    );
    assert!(napcat.pending().is_none(), "别的什么都不发");
    assert!(run_log(&home.root).contains("venue cards saved groups=1 privates=0"));
    stopped(home).await;
}

#[tokio::test]
async fn a_wrong_card_is_refused_with_its_problems_and_nothing_changes() {
    let script = Script::new([]);
    let (home, _napcat, _) = started(&script, RULES, "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let before = on_disk(&home);
    let wrong = json!({"groups": [{"id": LISTED, "set": {"rate": "abc", "sleep": "8:00"}}], "privates": [{"id": LIN, "set": {"persona": "Bad Name"}}]});
    let refused = save(&mut head, wrong).await;
    assert_eq!(refused["result"]["saved"], false, "{refused}");
    let problems: Vec<(Value, Value, Value)> = refused["result"]["problems"]
        .as_array()
        .expect("问题的列表")
        .iter()
        .map(|problem| {
            (
                problem["card"].clone(),
                problem["key"].clone(),
                problem["code"].clone(),
            )
        })
        .collect();
    assert_eq!(
        problems,
        [
            (
                json!({"kind": "group", "platform": "qq", "id": LISTED}),
                json!("sleep"),
                json!("bad_format")
            ),
            (
                json!({"kind": "group", "platform": "qq", "id": LISTED}),
                json!("rate"),
                json!("bad_format")
            ),
            (
                json!({"kind": "private", "platform": "qq", "id": LIN}),
                json!("persona"),
                json!("bad_format")
            ),
        ]
    );
    // 不是卡片的样子：认不得的格、号不合写法（施工 I-79：号写成字的也收，带空白、`:` 的不收）。
    for bad in [
        json!({"groups": [{"id": "a b"}]}),
        json!({"groups": [{"id": "a:b"}]}),
        json!({"unknown": 1}),
        json!(null),
    ] {
        let refused = save(&mut head, bad.clone()).await;
        assert_eq!(
            refused["error"]["data"]["message"], "bad_params",
            "{bad} {refused}"
        );
    }
    assert_eq!(on_disk(&home), before, "盘上的不变");
    stopped(home).await;
}

#[tokio::test]
async fn the_page_gets_cards_defaults_groups_friends_and_the_whitelist() {
    let script = Script::new([]);
    let onebot = format!("whitelist = [\"qq:{LIN}\"]\n");
    let (home, _napcat, _) = started(&script, RULES, &onebot, MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    head.until("status", |status| status["napcat"]["connected"] == true)
        .await;
    let cards =
        json!({"groups": [{"id": LISTED, "set": {"rate": "2/60s"}}], "privates": [{"id": LIN}]});
    assert_eq!(
        save(&mut head, cards).await["result"],
        json!({"saved": true})
    );
    let got = head.page_call("cards.get").await["result"].clone();
    assert_eq!(got["cards"]["groups"][0]["set"]["rate"], "2/60s", "{got}");
    assert_eq!(got["whitelist"], json!([LIN]));
    assert_eq!(
        got["groups"],
        json!([{"id": 777, "name": "测试群", "members": 12, "bot": BOT}, {"id": 666, "name": "另一个群", "members": 30, "bot": BOT}])
    );
    assert_eq!(
        got["friends"],
        json!([{"id": BOT, "name": "miyu", "bot": BOT}, {"id": LIN, "name": "lin", "bot": BOT}])
    );
    assert!(got["personas"].is_array(), "{got}");
    // 默认值照卡片下面几层：限流是出厂的，插嘴照系统的规则（千分比 0 是少插嘴），群卡没写的回复。
    let group = &got["defaults"]["groups"][LISTED.to_string()];
    assert_eq!(
        group["rate"],
        json!({"value": "30/60s", "from": "factory"}),
        "{group}"
    );
    assert_eq!(group["chatty"], json!({"value": "few", "from": "system"}));
    assert_eq!(group["on"], json!({"value": true, "from": null}));
    assert_eq!(group["persona"], json!({"value": null, "from": null}));
    // 长文（施工 O-59 上）：转成图、300 个字都是出厂的。
    assert_eq!(
        group["long_text"],
        json!({"value": "image", "from": "factory"})
    );
    assert_eq!(
        group["long_chars"],
        json!({"value": 300, "from": "factory"})
    );
    // 仅发送最终消息（施工 O-57）：群出厂开，来处是出厂的规则。
    assert_eq!(
        group["final_only"],
        json!({"value": true, "from": "factory"})
    );
    assert_eq!(got["defaults"]["other_groups"]["rate"]["value"], "5/300s");
    // 卡上 `rules` 能写的每一个键（施工 I-7 中一）：系统的规则改了的照它，没改的照出厂，可以不写的没写是空的。
    let rules = &group["rules"];
    assert_eq!(
        rules["chatty.probability"],
        json!({"value": 0, "from": "system"}),
        "{rules}"
    );
    assert_eq!(
        rules["chatty.base"],
        json!({"value": 0.8, "from": "factory"}),
        "{rules}"
    );
    assert_eq!(
        rules["chatty.continuation_window"]["value"], "30s",
        "{rules}"
    );
    assert_eq!(
        rules["judge.model"],
        json!({"value": null, "from": null}),
        "{rules}"
    );
    assert!(rules["parallel"].is_object(), "{rules}");
    assert!(group["recent"].is_object(), "回复时带几条也列出来：{group}");
    let private = &got["defaults"]["privates"][LIN.to_string()];
    assert_eq!(
        private["final_only"],
        json!({"value": false, "from": "factory"}),
        "私聊出厂每一步都发：{private}"
    );
    assert!(
        private.get("strip_period").is_none(),
        "去句号不在卡片上：{private}"
    );
    assert!(private.get("rate").is_none(), "私聊卡没有限流：{private}");
    // 「其他私聊」（施工 O-60）：测具没写这张卡，出厂不回复；限流照出厂规则私聊的那一档。
    let others = &got["defaults"]["other_privates"];
    assert_eq!(
        others["on"],
        json!({"value": false, "from": null}),
        "{others}"
    );
    assert_eq!(
        others["rate"],
        json!({"value": "5/300s", "from": "factory"}),
        "{others}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn the_extra_prompt_goes_to_the_end_of_system_and_leaves_when_removed() {
    let script = Script::new([
        Play::Says("群里在"),
        Play::Says("私聊在"),
        Play::Says("又在"),
    ]);
    let onebot = format!("whitelist = [\"qq:{LIN}\"]\n");
    let (home, mut napcat, _) = started(&script, RULES, &onebot, MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let cards = json!({
        "other_groups": {"on": true},
        "groups": [{"id": LISTED, "set": {"extra_prompt": "大家都知道你是 AI。\n别刷屏。"}}],
        "privates": [{"id": LIN, "set": {"extra_prompt": "他在备考。"}}],
    });
    assert_eq!(
        save(&mut head, cards).await["result"],
        json!({"saved": true})
    );
    napcat.send(called(LISTED, 1));
    assert_eq!(napcat.group_reply(LISTED).await, "群里在");
    napcat.send(private_frame(LIN, 2, json!([plain("在吗")])));
    let reply = napcat.action().await;
    assert_eq!(
        reply["params"]["message"][0]["data"]["text"], "私聊在",
        "{reply}"
    );
    let requests = script.requests();
    let group_system = requests[0].1.system.trim_end().to_string();
    assert!(
        group_system.ends_with(&format!(
            "<venue>QQ group \"{GROUP_NAME}\" ({LISTED})</venue>\n<venue_note>\n大家都知道你是 AI。\n别刷屏。\n</venue_note>"
        )),
        "群名接着附加提示词，在 system 最后：{}",
        &group_system[group_system.len().saturating_sub(400)..]
    );
    let private_system = requests[1].1.system.trim_end().to_string();
    assert!(
        private_system.ends_with("<venue_note>\n他在备考。\n</venue_note>"),
        "私聊第一轮就带上：{private_system}"
    );
    // 删掉私聊卡的提示词：下一条消息照新的去掉那一层，这一轮就没有。
    let removed = json!({"other_groups": {"on": true}, "groups": [{"id": LISTED}], "privates": [{"id": LIN}]});
    assert_eq!(
        save(&mut head, removed).await["result"],
        json!({"saved": true})
    );
    napcat.send(private_frame(LIN, 3, json!([plain("还在吗")])));
    let reply = napcat.action().await;
    assert_eq!(
        reply["params"]["message"][0]["data"]["text"], "又在",
        "{reply}"
    );
    let requests = script.requests();
    assert!(
        !requests[2].1.system.contains("<venue_note>"),
        "{}",
        requests[2].1.system
    );
    assert!(
        run_log(&home.root).contains("venue tail set"),
        "去掉也记一行"
    );
    stopped(home).await;
}

#[tokio::test]
async fn the_final_period_setting_applies_to_every_venue() {
    // 去句号是通用的配置项（施工 O-58 补二）：关了以后群里的回话留着「。」。
    let script = Script::new([Play::Says("在的。")]);
    let (home, mut napcat, _) = started(&script, RULES, "strip_period = false\n", MEMBERS).await;
    napcat.send(called(OTHER, 1));
    assert_eq!(napcat.group_reply(OTHER).await, "在的。");
    stopped(home).await;
}
