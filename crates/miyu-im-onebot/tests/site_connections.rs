//! im 页面上的连接（施工 I-6 中）：进程里跑的桥连真核心，页面经通道问连接、状态，存连接设置；机器人的名字页面问时才问平台，
//! 头像由页面服务转；断开了的号留着。

use std::io::{Read, Write};

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::browser::{call, changed, get, logged_in, page_port};
use crate::support::head::Head;
use crate::support::napcat::{NICKNAME, admin_napcat};
use crate::support::*;

/// 一张小小的 PNG（只看开头的几个字节认）。
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

/// 本机起一个只会回 [`PNG`] 的 HTTP 服务：交回端口和它收到的路径。
fn picture_server() -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("挑得到");
    let port = listener.local_addr().expect("有地址").port();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let saw = std::sync::Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = [0u8; 2048];
            let read = stream.read(&mut head).unwrap_or(0);
            let line = String::from_utf8_lossy(&head[..read])
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            saw.lock()
                .expect("没 panic")
                .push(line.split(' ').nth(1).unwrap_or_default().to_string());
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                PNG.len()
            );
            if stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(PNG))
                .is_err()
            {
                // 那头不收了：下一个。
            }
        }
    });
    (port, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bot_gets_its_name_avatar_and_a_card_after_it_leaves() {
    let home = Home::new(&Script::new([]));
    let (pictures, seen) = picture_server();
    let mut serve = serve(home.root.clone(), settings());
    serve.tuning.avatar_user_url = format!("http://127.0.0.1:{pictures}/u/{{id}}.png");
    serve.tuning.avatar_group_url = format!("http://127.0.0.1:{pictures}/g/{{id}}.png");
    let bridge = start(serve).await;
    let port = page_port(&bridge, 1).await;
    let mut open = logged_in(&home.root, port).await;
    let mut napcat = admin_napcat(bridge.port).await;
    napcat.version().await;
    // 头一回问：还没有名字，桥另去问平台；问到了推变了，再问就带上。
    let first = call(&mut open, 1, "connections", json!({})).await;
    assert!(
        first["result"]["connections"][0].get("name").is_none(),
        "{first}"
    );
    napcat.login_info().await;
    let named = within("问到名字", async {
        let mut asked = 2;
        loop {
            changed(&mut open).await;
            let answer = call(&mut open, asked, "connections", json!({})).await;
            if answer["result"]["connections"][0]["name"] == NICKNAME {
                return answer["result"]["connections"][0].clone();
            }
            asked += 1;
        }
    })
    .await;
    assert_eq!(named["connected"], true, "{named}");
    assert_eq!(named["avatar"], format!("/avatar/{BOT}"));
    // 头像由页面服务转：照模板换号去取，取过的记着。
    let avatar = get(port, &format!("/avatar/{BOT}")).await;
    assert_eq!(avatar.status, 200);
    assert_eq!(avatar.header("content-type"), Some("image/png"));
    // 头像让浏览器记十分钟（施工 I-7 上修：页面重画不闪），别的照旧不记。
    assert_eq!(avatar.header("cache-control"), Some("private, max-age=600"));
    assert_eq!(
        get(port, "/avatar/x").await.header("cache-control"),
        Some("no-store")
    );
    assert_eq!(avatar.body, PNG);
    assert_eq!(get(port, &format!("/avatar/{BOT}")).await.body, PNG);
    assert_eq!(
        *seen.lock().expect("没 panic"),
        [format!("/u/{BOT}.png")],
        "取过的不再取"
    );
    assert_eq!(
        get(port, "/avatar/12345").await.status,
        404,
        "不认识的号不取"
    );
    assert_eq!(get(port, "/avatar/x").await.status, 404);
    // 会话卡上的（施工 I-7 上）：群的、人的照各自的模板取，不查号。
    // 取过的记着：隔了别的再要也不再取。
    assert_eq!(get(port, "/avatar/user/20004").await.body, PNG);
    assert_eq!(get(port, "/avatar/group/777").await.body, PNG);
    assert_eq!(get(port, "/avatar/user/20004").await.body, PNG);
    assert_eq!(get(port, "/avatar/group/777").await.body, PNG);
    assert_eq!(
        *seen.lock().expect("没 panic"),
        [
            format!("/u/{BOT}.png"),
            "/u/20004.png".into(),
            "/g/777.png".into()
        ],
        "群、人的各取一次"
    );
    for wrong in [
        "/avatar/group/a:b",
        "/avatar/group/-1",
        "/avatar/group/0",
        "/avatar/room/1",
        "/avatar/user/",
    ] {
        assert_eq!(get(port, wrong).await.status, 404, "{wrong}");
    }
    // 断开了：卡留着，写断开的时刻。
    let closing = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("在 1970 年以后")
        .as_millis();
    napcat.close().await;
    let gone = within("断开的留着", async {
        let mut asked = 100;
        loop {
            changed(&mut open).await;
            let answer = call(&mut open, asked, "connections", json!({})).await;
            let one = answer["result"]["connections"][0].clone();
            if one["connected"] == false {
                return one;
            }
            asked += 1;
        }
    })
    .await;
    assert_eq!(gone["bot"], BOT.to_string(), "{gone}");
    let at = u128::from(gone["since"].as_u64().expect("断开的时刻"));
    assert!(at + 1000 >= closing, "写的是断开的时刻：{gone}");
    assert_eq!(gone["name"], NICKNAME, "名字照旧");
    assert!(gone.get("implementation").is_none(), "{gone}");
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_page_reads_the_status_and_saves_the_connection_settings() {
    // 核心照真核心那样把出厂的包的配置项拼进来：先把 `miyu-im` 链到测试程序旁边（同 `settings.rs`）。
    crate::support::spawning::linked();
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let mut open = logged_in(&home.root, port).await;
    let status = call(&mut open, 1, "status", json!({})).await["result"].clone();
    assert_eq!(status["path"], "/ws", "{status}");
    assert_eq!(status["platform"], "qq", "{status}");
    assert_eq!(status["listen"], bridge.port, "{status}");
    assert_eq!(status["token"], "set", "{status}");
    let wanted = crate::support::spawning::free_port();
    let saved = call(
        &mut open,
        2,
        "connection.save",
        json!({"values": {"listen": wanted.to_string(), "token": " fresh-token "}}),
    )
    .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let mut head = Head::connect(&home.root).await;
    let items = head.call("config.get", json!({})).await["result"]["items"].clone();
    assert_eq!(items["im.qq.listen"]["value"], wanted, "{items}");
    let secrets = head.call("secret.list", json!({})).await["result"]["secrets"].clone();
    let token: Vec<&Value> = secrets
        .as_array()
        .expect("是列表")
        .iter()
        .filter(|one| one["name"] == "im-qq-token")
        .collect();
    assert_eq!(token.len(), 1, "令牌存成密钥：{secrets}");
    assert_eq!(token[0]["set"], true, "{secrets}");
    assert_eq!(
        token[0]["used_by"],
        json!(["im.qq.token"]),
        "配置里写引用：{secrets}"
    );
    // 写错的：核心的原因照交；不是驱动的配置项、不是字的：bad_params。
    let wrong = call(
        &mut open,
        3,
        "connection.save",
        json!({"values": {"listen": "abc"}}),
    )
    .await;
    assert_eq!(wrong["error"]["reason"], "config_invalid", "{wrong}");
    for values in [
        json!({"whitelist": "x"}),
        json!({"listen": 8301}),
        json!({}),
    ] {
        let bad = call(&mut open, 4, "connection.save", json!({"values": values})).await;
        assert_eq!(bad["error"]["reason"], "bad_params", "{values}");
    }
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn the_page_adds_edits_and_removes_connections() {
    // 施工 I-81 下中（设计 07 第 7.7 节）：连接表写进三个藏起来的列表；原来设了 Token 的那一个是头一个连接，写表时原来的三项去掉。
    crate::support::spawning::linked();
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let mut open = logged_in(&home.root, port).await;
    let mut head = Head::connect(&home.root).await;
    let table = |items: &Value| {
        (
            items["im.qq.ports"]["value"].clone(),
            items["im.qq.hosts"]["value"].clone(),
            items["im.qq.tokens"]["value"].as_array().map(Vec::len),
        )
    };
    // 原来那一个：设了 Token。
    let saved = call(
        &mut open,
        1,
        "connection.save",
        json!({"values": {"token": "first-token"}}),
    )
    .await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let (second, third) = (40_002, 40_003);
    let added = call(
        &mut open,
        2,
        "connection.add",
        json!({"port": second, "token": " second-token "}),
    )
    .await;
    assert_eq!(added["result"], json!({"saved": true}), "{added}");
    let items = head.call("config.get", json!({})).await["result"]["items"].clone();
    assert_eq!(
        table(&items),
        (
            json!([8301, second]),
            json!(["0.0.0.0", "0.0.0.0"]),
            Some(2)
        ),
        "原来那一个在前：{items}"
    );
    assert!(
        items["im.qq.token"].get("value").is_none(),
        "原来的三项去掉了：{items}"
    );
    let secrets = head.call("secret.list", json!({})).await["result"]["secrets"].clone();
    assert!(
        secrets
            .as_array()
            .expect("是列表")
            .iter()
            .any(|one| one["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("im-qq-token-40002-"))),
        "新的 Token 存成密钥：{secrets}"
    );

    // 改：换端口、换地址；Token 空的不改。
    let edited = call(
        &mut open,
        3,
        "connection.edit",
        json!({"port": second, "values": {"listen": third.to_string(), "host": "127.0.0.1", "token": ""}}),
    )
    .await;
    assert_eq!(edited["result"], json!({"saved": true}), "{edited}");
    let items = head.call("config.get", json!({})).await["result"]["items"].clone();
    assert_eq!(
        table(&items),
        (
            json!([8301, third]),
            json!(["0.0.0.0", "127.0.0.1"]),
            Some(2)
        )
    );

    // 写错的：端口重了、Token 没写、找不到、端口超出范围。
    for (method, params) in [
        ("connection.add", json!({"port": third, "token": "x"})),
        ("connection.add", json!({"port": 40_004, "token": "  "})),
        ("connection.add", json!({"port": 80, "token": "x"})),
        (
            "connection.edit",
            json!({"port": 40_009, "values": {"host": "127.0.0.1"}}),
        ),
        (
            "connection.edit",
            json!({"port": third, "values": {"listen": 8301}}),
        ),
        ("connection.remove", json!({"port": 40_009})),
    ] {
        let wrong = call(&mut open, 4, method, params.clone()).await;
        assert_eq!(
            wrong["error"]["reason"], "bad_params",
            "{method} {params}：{wrong}"
        );
    }

    // 删：剩一个以后不让删。
    let removed = call(&mut open, 5, "connection.remove", json!({"port": 8301})).await;
    assert_eq!(removed["result"], json!({"saved": true}), "{removed}");
    let items = head.call("config.get", json!({})).await["result"]["items"].clone();
    assert_eq!(
        table(&items),
        (json!([third]), json!(["127.0.0.1"]), Some(1))
    );
    let last = call(&mut open, 6, "connection.remove", json!({"port": third})).await;
    assert_eq!(last["error"]["reason"], "last_connection", "{last}");
    bridge.stop().await.expect("停得下");
}
