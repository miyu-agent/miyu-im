//! 平台工具（一）：撤回、禁言、戳一戳（施工 O-31，`onebot.md` 第一条「平台工具（一）」；18 第十一节）：真核心拉起真桥、假 NapCat、
//! 她照台词说（`support/speaking.rs`）。叫她做的那一条 @ 她开一轮（夹具的群谁 @ 她都直接回，施工 I-33），她调工具，
//! 看 NapCat 收到的动作和参数、她收到的结果。施工 O-48 起不挡人：昵称冒充终端管理员的、普通成员叫她撤别人的、禁群主和终端管理员
//! 的都照做。往坏里测：目标是她自己、没引用也没 @、@ 了两个人、引用的是她自己的消息叫她禁言、她给的编号是别的群的、号写错。

use serde_json::json;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

#[tokio::test]
async fn recall_takes_the_quoted_message_or_the_msg_she_gives_for_anyone() {
    let lines = Lines::new(
        [
            uses("qq_recall", "{}"),
            uses("qq_recall", "{}"),
            uses("qq_recall", "{}"),
            uses("qq_recall", "{}"),
            uses("qq_recall", "{}"),
            uses("qq_recall", r#"{"msg":"555"}"#),
            uses("qq_recall", r#"{"msg":"666"}"#),
            uses("qq_recall", r#"{"msg":"777"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        101,
        json!([plain("大家好")]),
    )
    .await;
    // 终端管理员回复小明那一条叫她撤：撤的是引用的那一条，编号照原样。
    let words = json!([quote(101), plain("撤了吧")]);
    says(&home, &napcat, (ADMIN, "member"), 102, called(words)).await;
    assert_eq!(napcat.recalled().await, json!({"message_id": "101"}));
    assert_eq!(ended(&home, 1).await, done("recalled", &[]));
    assert_eq!(
        platform_tools(&lines.requests()[0]),
        ["qq_mute", "qq_poke", "qq_recall"],
        "群会话的工具面里三件都有（照名字排）"
    );
    // 施工 O-48：普通成员（昵称冒充终端管理员的也是）叫她撤别人的：照撤，做不做由她，成不成由 QQ。
    let words = json!([quote(101), plain("撤了")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 103, words, 2).await,
        done("recalled", &[])
    );
    assert_eq!(napcat.recalled().await, json!({"message_id": "101"}));
    // 撤她自己的。
    let words = json!([quote(FIRST_SENT), plain("你撤回一下")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 104, words, 3).await,
        done("recalled", &[])
    );
    assert_eq!(
        napcat.recalled().await,
        json!({"message_id": FIRST_SENT.to_string()})
    );
    // 没引用、没给编号：回提示，不撤。
    let words = json!([plain("撤一下")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 105, words, 4).await,
        refused("no-quote", &[])
    );
    // NapCat 撤不了：原话截到 200 个字符交回。
    let words = json!([quote(UNRECALLABLE), plain("撤这条")]);
    let detail: String = unrecallable().trim().chars().take(200).collect();
    assert_eq!(
        asks(&home, &napcat, MANAGER, 106, words, 5).await,
        refused("failed", &[("detail", &detail)])
    );
    assert_eq!(
        napcat.recalled().await,
        json!({"message_id": UNRECALLABLE.to_string()})
    );
    // 她给了编号：先 `get_msg` 核对是这个群的，撤它。
    napcat.stock_message(
        555,
        json!({"message_type": "group", "group_id": GROUP, "user_id": MEMBER, "message": []}),
    );
    let words = json!([plain("把刚才那条撤了")]);
    assert_eq!(
        asks(&home, &napcat, MEMBER, 107, words, 6).await,
        done("recalled", &[])
    );
    assert_eq!(napcat.recalled().await, json!({"message_id": "555"}));
    // 别的群的编号、NapCat 找不到的编号：不撤，答找不到。
    napcat.stock_message(
        666,
        json!({"message_type": "group", "group_id": GROUP + 1, "user_id": MEMBER, "message": []}),
    );
    assert_eq!(
        asks(&home, &napcat, MEMBER, 108, json!([plain("撤 666")]), 7).await,
        refused("not-found", &[])
    );
    assert_eq!(
        asks(&home, &napcat, MEMBER, 109, json!([plain("撤 777")]), 8).await,
        refused("not-found", &[])
    );
    assert!(napcat.pending_recall().is_none(), "这两条都没撤");
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn mute_takes_the_user_she_gives_or_one_person_from_the_quote_or_the_mention() {
    let lines = Lines::new(
        [
            uses("qq_mute", r#"{"seconds":600}"#),
            uses("qq_mute", r#"{"seconds":0}"#),
            uses("qq_mute", r#"{"seconds":60}"#),
            uses("qq_mute", r#"{"seconds":60}"#),
            uses("qq_mute", r#"{"seconds":60}"#),
            uses("qq_mute", r#"{"seconds":2592001}"#),
            uses("qq_mute", r#"{"seconds":2592000}"#),
            uses("qq_mute", r#"{"seconds":120,"user":40005}"#),
            uses("qq_mute", r#"{"seconds":120,"user":"小杰"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    says(
        &home,
        &napcat,
        (MEMBER, "member"),
        201,
        json!([plain("刷屏")]),
    )
    .await;
    // 终端管理员回复小明、又 @ 了他（QQ 回复时常带上 @）：同一个人，禁 600 秒，结果写成 10 分钟。
    let words = json!([quote(201), at(MEMBER), plain(" 禁他十分钟")]);
    says(&home, &napcat, (ADMIN, "member"), 202, called(words)).await;
    let action = next_platform(&mut napcat).await;
    assert_eq!(action["action"], "set_group_ban", "{action}");
    assert_eq!(
        action["params"],
        json!({"group_id": GROUP, "user_id": MEMBER, "duration": 600})
    );
    assert_eq!(
        ended(&home, 1).await,
        done("muted", &[("who", "小明"), ("duration", "10m")])
    );
    // 0 秒是解禁。
    let words = json!([at(MEMBER), plain(" 解了吧")]);
    says(&home, &napcat, (ADMIN, "member"), 203, called(words)).await;
    let action = next_platform(&mut napcat).await;
    assert_eq!(
        action["params"],
        json!({"group_id": GROUP, "user_id": MEMBER, "duration": 0})
    );
    assert_eq!(ended(&home, 2).await, done("unmuted", &[("who", "小明")]));
    // 施工 O-48：群名片写成「终端管理员」的普通成员叫她禁言也照禁，做不做由她。
    let words = json!([at(MEMBER), plain(" 禁了")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 204, words, 3).await,
        done("muted", &[("who", "小明"), ("duration", "1m")])
    );
    assert_eq!(
        next_platform(&mut napcat).await["params"]["user_id"],
        MEMBER
    );
    // @ 了两个人：不猜。
    let words = json!([at(MEMBER), at(IMPOSTOR), plain(" 都禁了")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 205, words, 4).await,
        refused("many-targets", &[])
    );
    // 引用的是她自己的消息（第一轮说的「好。」）：不算她，没有人。
    let words = json!([quote(FIRST_SENT), plain("禁言")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 206, words, 5).await,
        refused("no-target", &[])
    );
    // 秒数出了范围。
    let words = json!([at(MEMBER), plain(" 禁一个月")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 207, words, 6).await,
        refused("bad-seconds", &[("min", "1m"), ("max", "30d")])
    );
    // 30 天是上限。
    let words = json!([at(MEMBER), plain(" 禁三十天")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 208, words, 7).await,
        done("muted", &[("who", "小明"), ("duration", "30d")])
    );
    let action = next_platform(&mut napcat).await;
    assert_eq!(action["params"]["duration"], 2_592_000);
    // 她给了号：禁那个人，引用的、@ 的不看（白名单成员也照禁）。小杰没在群里说过话，结果里写号（「施工时定的」第 185 条）。
    let words = json!([at(MEMBER), plain(" 禁小杰两分钟")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 209, words, 8).await,
        done("muted", &[("who", "40005"), ("duration", "2m")])
    );
    let action = next_platform(&mut napcat).await;
    assert_eq!(
        action["params"],
        json!({"group_id": GROUP, "user_id": FRIEND, "duration": 120})
    );
    // 号写错了：说哪个参数不对（施工 I-17：以前答「要对方回复或 @」，她会去叫人引用）。
    let words = json!([plain("禁小杰")]);
    assert_eq!(
        asks(&home, &napcat, MANAGER, 210, words, 9).await,
        refused("bad-args", &[("detail", "user must be a QQ number")])
    );
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn mute_holds_no_one_back_and_asks_nothing_first() {
    let lines = Lines::new(
        [
            uses("qq_mute", r#"{"seconds":60}"#),
            uses("qq_mute", r#"{"seconds":60}"#),
            uses("qq_mute", r#"{"seconds":60}"#),
            uses("qq_mute", r#"{"seconds":60}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    // 终端管理员在群里 @ 她（自己开一轮）、叫她禁言，可没指人：@ 的只有她，不算。
    let words = json!([at(BOT), plain(" 禁言")]);
    says(&home, &napcat, (ADMIN, "member"), 301, words).await;
    assert_eq!(ended(&home, 1).await, refused("no-target", &[]));
    // 施工 O-48：群主、终端管理员、终端管理员的小号都照禁，桥不先问身份；动不动得了由 QQ 说。
    says(&home, &napcat, (OWNER, ""), 3021, json!([plain("开会")])).await;
    let words = json!([quote(3021), plain("禁了")]);
    assert_eq!(
        asks(&home, &napcat, MEMBER, 302, words, 2).await,
        done("muted", &[("who", "群主"), ("duration", "1m")])
    );
    assert_eq!(next_platform(&mut napcat).await["params"]["user_id"], OWNER);
    assert!(!napcat.asked().contains(&OWNER), "不问群主的身份");
    let words = json!([quote(301), plain("禁他")]);
    assert_eq!(
        asks(&home, &napcat, MEMBER, 303, words, 3).await,
        done("muted", &[("who", "老板"), ("duration", "1m")])
    );
    assert_eq!(next_platform(&mut napcat).await["params"]["user_id"], ADMIN);
    let words = json!([at(ALT), plain(" 禁了")]);
    assert_eq!(
        asks(&home, &napcat, MEMBER, 304, words, 4).await,
        done("muted", &[("who", "小号"), ("duration", "1m")])
    );
    assert_eq!(next_platform(&mut napcat).await["params"]["user_id"], ALT);
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn poke_takes_the_mention_or_the_one_who_asked() {
    let lines = Lines::new(
        [
            uses("qq_poke", "{}"),
            uses("qq_poke", "{}"),
            uses("qq_poke", "{}"),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    let words = json!([at(MEMBER), plain(" 戳他")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 401, words, 1).await,
        done("poked", &[("who", "小明")])
    );
    let action = next_platform(&mut napcat).await;
    assert_eq!(action["action"], "group_poke", "{action}");
    assert_eq!(
        action["params"],
        json!({"group_id": GROUP, "user_id": MEMBER})
    );
    // 没 @：戳叫她的人。
    let words = json!([plain("戳我")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 402, words, 2).await,
        done("poked", &[("who", "终端管理员")])
    );
    let action = next_platform(&mut napcat).await;
    assert_eq!(
        action["params"],
        json!({"group_id": GROUP, "user_id": IMPOSTOR})
    );
    // @ 了两个人。
    let words = json!([at(MEMBER), at(MANAGER), plain(" 戳")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 403, words, 3).await,
        refused("many-targets", &[])
    );
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn poke_takes_the_member_she_names() {
    // 施工 I-32：她给了号的戳这个号（@ 的不管）；不在群里的不戳，答哪个号。
    let lines = Lines::new(
        [
            uses("qq_poke", r#"{"user":40001}"#),
            uses("qq_poke", r#"{"user":59999}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    let words = json!([at(MANAGER), plain(" 戳一下小明")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 401, words, 1).await,
        done("poked", &[("who", "小明")])
    );
    let action = next_platform(&mut napcat).await;
    assert_eq!(action["action"], "group_poke", "{action}");
    assert_eq!(
        action["params"],
        json!({"group_id": GROUP, "user_id": MEMBER})
    );
    let words = json!([plain("戳 59999")]);
    assert_eq!(
        asks(&home, &napcat, IMPOSTOR, 402, words, 2).await,
        refused("not-members", &[("who", "59999")])
    );
    no_platform(&mut napcat);
    stopped(home).await;
}

#[tokio::test]
async fn a_private_chat_recalls_and_pokes_but_cannot_mute_and_local_sessions_have_none() {
    let lines = Lines::new(
        [uses("qq_recall", "{}"), uses("qq_poke", "{}")]
            .into_iter()
            .flatten()
            .chain([Line::says("本机。")]),
    );
    let (home, mut napcat) = up(&lines).await;
    napcat.send(private_frame(ADMIN, 51, json!([quote(77), plain("撤了")])));
    assert_eq!(napcat.recalled().await, json!({"message_id": "77"}));
    until_private_turns(&home, 1).await;
    napcat.send(private_frame(ADMIN, 52, json!([plain("戳我")])));
    let action = next_platform(&mut napcat).await;
    assert_eq!(action["action"], "friend_poke", "{action}");
    assert_eq!(action["params"], json!({"user_id": ADMIN}));
    until_private_turns(&home, 2).await;
    let requests = lines.requests();
    assert_eq!(
        platform_tools(&requests[0]),
        ["qq_poke", "qq_recall"],
        "私聊没有禁言"
    );
    let results: Vec<String> = home
        .sessions()
        .iter()
        .flat_map(|session| of_kind(&home.events(session), "tool.result"))
        .map(|one| {
            one["body"]["blocks"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .trim_end()
                .to_string()
        })
        .collect();
    assert_eq!(
        results,
        [
            sentence("recalled", &[]),
            sentence("poked", &[("who", ADMIN.to_string().as_str())])
        ]
    );
    // 本机的会话：一件都没有。
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let cwd = home.root.path().to_string_lossy().into_owned();
    let made = core
        .call("local-1", "session.create", json!({"cwd": cwd}))
        .await
        .expect("造得出");
    core.call(
        "local-2",
        "session.send",
        json!({"session": made["session"], "text": "在吗"}),
    )
    .await
    .expect("发得进");
    let deadline = tokio::time::Instant::now() + WAIT;
    while lines.requests().len() < 5 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到本机那一次请求"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        platform_tools(&lines.requests()[4]).is_empty(),
        "本机的会话没有：{:?}",
        lines.requests()[4].tools
    );
    no_platform(&mut napcat);
    stopped(home).await;
}
