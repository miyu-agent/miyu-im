//! 一个 im 同时挂几个驱动（施工 I-70 中上，`onebot.md` 第一条「在哪」「设置和状态」「提供者和不说话」）：OneBot 加一个假的第二个
//! 平台 `tg`（`support/second.rs`）。各走各的：tg 的私聊进 `tg:private:<号>` 的会话、回话经 tg 的连接发，NapCat 那头什么都没
//! 收到，反过来也一样；两个平台的机器人号、对方的号、消息编号写成一样也不串。两套工具合成一份登记，场所的平台对不上的工具不调。
//! 配置项改了哪个平台的只通知那个驱动。状态文件每个驱动一格，顶上照旧是 QQ 的。平台名重了、工具名撞了起不来。

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use miyu_im::chat::fake::Asked;
use miyu_im::chat::{Id, To};
use miyu_im::serve::{Failure, Serve, run};

use crate::status_file::until_file;
use crate::support::browser::{self, logged_in, page_port};
use crate::support::second::*;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 两个驱动都起来了：核心、桥、转接、假 NapCat、假平台和它的连接。
struct Two {
    home: Home,
    bridge: Bridge,
    relay: Relay,
    napcat: NapCat,
    tg: Telegram,
    link: mpsc::Receiver<Asked>,
}

/// 起核心、OneBot 加假平台：握手交的配置另带 `im.tg.mode = "a"`；她先后说 `said` 里的几句。QQ、tg 上的机器人号都是 [`BOT`]。
async fn up(said: &[&'static str]) -> Two {
    let lines = Lines::new(said.iter().map(|text| Line::says(text)));
    let home = Home::two_platforms(Arc::new(lines.clone()));
    let tg = Telegram::echoing();
    let package = package_with_tg(&home.scratch("package"));
    let mut config = handed(&settings());
    config["im.tg.mode"] = json!("a");
    let (serve, relay) = serve_handed(home.root.clone(), config, None);
    let bridge = start(with_tg(serve, &tg, &package)).await;
    let napcat = admin_napcat(bridge.port).await;
    let link = tg.connect(Id::from(BOT)).await;
    Two {
        home,
        bridge,
        relay,
        napcat,
        tg,
        link,
    }
}

/// 场所是 `venue` 的会话的编号（看它头一条事件）。
fn session_of(home: &Home, venue: &str) -> String {
    home.sessions()
        .iter()
        .find(|session| {
            home.events(session)
                .first()
                .is_some_and(|created| created["body"]["venue"] == venue)
        })
        .map(|session| session.as_str().to_string())
        .unwrap_or_else(|| panic!("有 {venue} 的会话"))
}

/// 照核心转来的样子经 `relay` 往桥推一次 `tool.call`（编号 `id`，会话 `session`，工具 `tool`），等到桥答了交回 `result`。
async fn call(relay: &Relay, id: &str, session: &str, tool: &str) -> Value {
    let params = json!({"session": session, "call_id": id, "tool": tool, "args": {}, "cwd": "/"});
    answered(relay, id, "tool.call", params).await["result"].clone()
}

/// 照核心转来的样子经 `relay` 往桥推一次 `method.call`（施工 I-85 中：后台页、会话页调 im 的方法），等到桥答了交回整句回应。
async fn method(relay: &Relay, id: &str, method: &str, params: Value) -> Value {
    let params = json!({"package": "im", "method": method, "params": params});
    answered(relay, id, "method.call", params).await
}

/// 经 `relay` 往桥推编号 `id` 的 `method`，等到桥答了交回整句回应。
async fn answered(relay: &Relay, id: &str, method: &str, params: Value) -> Value {
    relay.request(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
    within("桥答", async {
        loop {
            if let Some(answer) = relay.answers().into_iter().find(|one| one["id"] == id) {
                return answer;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
}

/// 答的那一句的字。
fn text(result: &Value) -> &str {
    result["blocks"][0]["text"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn ids_written_as_text_get_through_like_numbers() {
    // 施工 I-79：别的平台的机器人号、对方的号是一串字（QQ 官方机器人、微信）。场所、平台上的人照字拼，回话经那个号的连接发给那个号。
    let mut two = up(&["字也认"]).await;
    let bot = Id::new("tgbot_x").expect("合写法");
    let boss = Id::new(TEXT_ADMIN).expect("合写法");
    let mut second = two.tg.connect(bot.clone()).await;
    two.tg.says_as((&bot, &boss), 9, "在吗").await;
    assert_eq!(
        Telegram::sent(&mut second).await,
        (To::Private(boss), "字也认".to_string()),
        "经号是字的那条连接发给号是字的人"
    );
    assert!(two.link.try_recv().is_err(), "另一个号的连接没发");
    session_of(&two.home, "tg:private:boss_7");
    let said = two.home.said();
    let heard = said
        .iter()
        .find(|one| one["by"]["via"] == "tg:boss_7")
        .expect("平台上的人照字拼");
    assert_eq!(
        heard["cause"],
        json!(format!("tg:tgbot_x:9:{TIME}")),
        "命令编号里的机器人号也是字"
    );
    two.bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn each_platform_hears_and_answers_on_its_own_link() {
    let mut two = up(&["tg 在", "qq 在"]).await;
    // 同一个机器人号、同一个人、同一个消息编号和时刻：先 tg，再 QQ。
    two.tg.says(BOT, 501, "在吗").await;
    assert_eq!(
        Telegram::sent(&mut two.link).await,
        (To::Private(Id::from(ADMIN)), "tg 在".to_string()),
        "tg 的回话经 tg 的连接发"
    );
    two.napcat.admin_says(501, "在吗").await;
    assert_eq!(
        two.napcat.reply().await,
        "qq 在",
        "NapCat 收到的头一句就是 QQ 的：tg 的那句没经它；同编号的 QQ 消息不当成重发"
    );
    assert!(two.link.try_recv().is_err(), "QQ 的回话没经 tg 的连接");
    let home = &two.home;
    let (tg, qq) = (
        session_of(home, "tg:private:10001"),
        session_of(home, "qq:private:10001"),
    );
    assert_ne!(tg, qq, "两个平台各一个会话");
    let said = home.said();
    let mut seen: Vec<(Value, Value)> = said
        .iter()
        .map(|one| (one["by"]["via"].clone(), one["cause"].clone()))
        .collect();
    seen.sort_by_key(|(via, _)| via.to_string());
    assert_eq!(
        seen,
        [
            (json!("qq:10001"), json!(format!("qq:{BOT}:501:{TIME}"))),
            (json!("tg:10001"), json!(format!("tg:{BOT}:501:{TIME}"))),
        ],
        "身份、命令编号各照各的平台拼"
    );
    two.bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn both_toolsets_are_provided_and_a_tool_runs_only_on_its_platform() {
    let mut two = up(&["tg 在", "qq 在"]).await;
    two.tg.says(BOT, 7, "在吗").await;
    Telegram::sent(&mut two.link).await;
    two.napcat.admin_says(8, "在吗").await;
    two.napcat.reply().await;
    // 一份登记里两套都有，照驱动的先后：QQ 的在前，tg 的在后。
    let provided = two.relay.provided();
    assert_eq!(provided.len(), 1, "登记一次：{provided:?}");
    let names: Vec<&str> = provided[0]["tools"]
        .as_array()
        .expect("列表")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(names.last(), Some(&ECHO), "{names:?}");
    assert!(names.contains(&"qq_skip_reply"), "{names:?}");
    let (tg, qq) = (
        session_of(&two.home, "tg:private:10001"),
        session_of(&two.home, "qq:private:10001"),
    );
    // QQ 的场所里调 tg 的工具：不叫 tg 的驱动做，照 tg 那一套的话答够不着。
    let refused = call(&two.relay, "t1", &qq, ECHO).await;
    assert_eq!(refused["error"], true, "{refused}");
    assert_eq!(text(&refused), "tg is not reachable.");
    assert!(two.tg.calls().is_empty(), "没调");
    // tg 的场所里调：平台、配置、状态目录照 tg 的。
    let done = call(&two.relay, "t2", &tg, ECHO).await;
    assert_eq!(
        (done["error"].clone(), text(&done)),
        (json!(false), "Echoed on tg.")
    );
    let calls = two.tg.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].platform, "tg");
    assert_eq!(calls[0].venue, "tg:private:10001");
    assert_eq!(
        calls[0].config,
        json!({"mode": "a"}).as_object().cloned().expect("对象")
    );
    assert_eq!(
        calls[0].store,
        miyu_im::status_file::store(&two.home.root, TG),
        "tg 自己的状态目录"
    );
    // tg 的场所里调 QQ 的工具：一样不调，照 QQ 那一套的话答。
    let refused = call(&two.relay, "t3", &tg, "qq_poke").await;
    assert_eq!(refused["error"], true, "{refused}");
    assert!(text(&refused).contains("QQ"), "{refused}");
    two.bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn a_config_push_reaches_only_its_platforms_driver() {
    let two = up(&[]).await;
    let gate = two.tg.gate().await;
    let mut config = gate.config();
    assert_eq!(config.borrow_and_update()["mode"], "a");
    // QQ 的令牌改了：OneBot 换上（状态里的 `token` 变了），tg 的没收到通知。
    two.relay.config(json!({"im.qq.token": null}));
    let mut asked = 0;
    loop {
        asked += 1;
        let status = page_status(&two.relay, &format!("s{asked}")).await;
        if status["token"] == "none" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!config.has_changed().expect("还在"), "QQ 的变了不通知 tg");
    // tg 的变了：tg 收到。
    two.relay.config(json!({"im.tg.mode": "b"}));
    within("tg 收到新配置", config.changed())
        .await
        .expect("还在");
    assert_eq!(config.borrow_and_update()["mode"], "b");
    two.bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_status_has_each_driver_and_qq_on_top() {
    let two = up(&[]).await;
    let root = two.home.root.clone();
    let file = until_file(&root, |file| {
        file["drivers"]["tg"]["napcat"]["connected"] == true
            && file["drivers"]["qq"]["napcat"]["connected"] == true
    })
    .await;
    assert_eq!(
        file["drivers"]["tg"],
        json!({"napcat": {"connected": true, "self_id": BOT.to_string()}, "mode": "a"})
    );
    let qq = &file["drivers"]["qq"];
    for key in ["napcat", "listen", "host", "token"] {
        assert_eq!(file[key], qq[key], "顶上的 {key} 照旧是 QQ 的：{file}");
    }
    assert_eq!(file["listen"], two.bridge.port);
    assert!(file.get("mode").is_none(), "tg 报的不到顶上：{file}");
    let status = page_status(&two.relay, "s1").await;
    assert_eq!(
        (status["platform"].clone(), status["platforms"].clone()),
        (json!("qq"), json!(["qq", "tg"]))
    );
    two.bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn two_drivers_on_one_platform_do_not_start() {
    let (dir, root) = temp_root();
    let doubled = Serve {
        drivers: vec![
            Arc::new(miyu_im_onebot::OneBot),
            Arc::new(miyu_im_onebot::OneBot),
        ],
        ..serve(root.clone(), settings())
    };
    let ran = within(
        "桥退出",
        run(doubled, |_| {}, |_| {}, std::future::pending()),
    )
    .await;
    match ran {
        Err(Failure::Start(reason)) => assert!(reason.contains("qq"), "{reason}"),
        other => panic!("平台名重了起不来：{other:?}"),
    }
    let none = Serve {
        drivers: Vec::new(),
        ..serve(root, settings())
    };
    let ran = within("桥退出", run(none, |_| {}, |_| {}, std::future::pending())).await;
    assert!(
        matches!(ran, Err(Failure::Start(_))),
        "一个驱动都没有起不来：{ran:?}"
    );
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里，不影响测试。
    }
}

#[tokio::test]
async fn a_tool_name_given_by_two_drivers_does_not_start() {
    let (dir, _) = temp_root();
    let problems =
        factory_with(&package_with_tg(&dir), &Telegram::clashing()).expect_err("工具名撞了");
    // 两条：名字不是 `tg_` 开头（施工 I-83：核心照前缀放），又和 QQ 的撞了。
    assert_eq!(problems.len(), 2, "{problems:?}");
    let whys: Vec<String> = problems
        .iter()
        .map(|problem| {
            assert_eq!(problem.file, "tg/tools/qq_poke.json");
            problem.why.clone().unwrap_or_default()
        })
        .collect();
    assert!(whys[0].contains("does not start with tg_"), "{whys:?}");
    assert!(
        whys[1].contains("qq_poke")
            && whys[1].contains("qq driver")
            && whys[1].contains("tg driver"),
        "{whys:?}"
    );
    // 不撞的照读：两套合成一份。
    assert!(factory_with(&package_with_tg(&dir), &Telegram::echoing()).is_ok());
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里，不影响测试。
    }
}

#[tokio::test]
async fn the_page_sees_every_platform_and_asks_the_one_it_names() {
    // 施工 I-85 上：连接列出每个平台的号（编号、做得到的几样）；状态、表情照写的 `platform` 答，不写的照第一个驱动。
    let two = up(&[]).await;
    let port = page_port(&two.bridge, 1).await;
    let mut open = logged_in(&two.home.root, port).await;
    let listed = browser::call(&mut open, 1, "connections", json!({})).await;
    let connections = listed["result"]["connections"]
        .as_array()
        .expect("列表")
        .clone();
    let ids: Vec<&str> = connections
        .iter()
        .filter_map(|one| one["id"].as_str())
        .collect();
    assert_eq!(ids, [format!("qq:{BOT}"), format!("tg:{BOT}")], "{listed}");
    assert!(
        connections[0]["caps"]
            .as_array()
            .is_some_and(|caps| caps.contains(&json!("recall")))
    );
    assert_eq!(connections[1]["caps"], json!([]), "tg 什么都做不到");
    let tg = browser::call(&mut open, 2, "status", json!({"platform": "tg"})).await;
    assert_eq!(tg["result"]["platform"], "tg", "{tg}");
    let first = browser::call(&mut open, 3, "status", json!({})).await;
    assert_eq!(first["result"]["platform"], "qq", "{first}");
    let unknown = browser::call(&mut open, 4, "status", json!({"platform": "wx"})).await;
    assert_eq!(unknown["error"]["reason"], "bad_params", "{unknown}");
    let reactions = browser::call(&mut open, 5, "reactions", json!({"platform": "tg"})).await;
    assert_eq!(reactions["result"]["reactions"], json!([]), "{reactions}");
    let qq = browser::call(&mut open, 6, "reactions", json!({})).await;
    assert!(
        qq["result"]["reactions"]
            .as_array()
            .is_some_and(|all| !all.is_empty())
    );
    two.bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_cards_come_per_platform_when_a_platform_is_named() {
    // 施工 I-85 中：会话页照选中的连接问 `cards.get {platform}`，只交那个平台的卡、白名单、默认值；没写平台的照旧交整份（整份存不丢
    // 别的平台的）。
    let Two {
        home: _home,
        relay,
        bridge,
        napcat,
        tg: _tg,
        link: _link,
    } = up(&[]).await;
    // 假 NapCat 答群、好友列表（不答的话 QQ 的那两次各等到驱动超时）。
    let _napcat = napcat.answering(&[]);
    let cards = json!({
        "groups": [
            {"id": 777, "set": {"on": true}},
            {"platform": "tg", "id": -100_888, "set": {"on": true}},
        ],
        "privates": [{"platform": "tg", "id": 31, "set": {}}],
    });
    let saved = method(&relay, "c1", "cards.save", json!({"cards": cards})).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");

    let tg = method(&relay, "c2", "cards.get", json!({"platform": "tg"})).await["result"].clone();
    assert_eq!(tg["platform"], "tg", "{tg}");
    assert_eq!(
        tg["cards"]["groups"].as_array().map(Vec::len),
        Some(1),
        "{tg}"
    );
    assert_eq!(tg["cards"]["groups"][0]["id"], -100_888, "{tg}");
    assert!(tg["defaults"]["groups"]["-100888"].is_object(), "{tg}");
    assert!(tg["defaults"]["privates"]["31"].is_object(), "{tg}");
    assert_eq!(tg["cards"].get("others"), None, "{tg}");

    let qq = method(&relay, "c3", "cards.get", json!({"platform": "qq"})).await["result"].clone();
    assert_eq!(
        qq["cards"]["groups"],
        json!([{"id": 777, "set": {"on": true}}]),
        "{qq}"
    );
    assert_eq!(qq["cards"]["privates"], json!([]), "{qq}");

    let whole = method(&relay, "c4", "cards.get", json!({})).await["result"].clone();
    assert_eq!(
        whole["cards"]["groups"].as_array().map(Vec::len),
        Some(2),
        "{whole}"
    );
    assert!(
        whole["defaults"]["groups"]["tg:-100888"].is_object(),
        "{whole}"
    );
    assert!(whole["defaults"]["groups"]["777"].is_object(), "{whole}");
    bridge.stop().await.expect("停得下");
}
