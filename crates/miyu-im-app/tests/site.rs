//! im 自己的页面服务（施工 I-5 上，设计 01 第九节第 1、3 条）：进程里跑的桥经本机套接字连真核心，页面服务听系统挑的端口。头照
//! 网页的样子要交接票，测试照浏览器的样子拿票换登录、开通道。

use serde_json::json;

use miyu_session::testkit::Script;

use crate::support::browser::{channel, get, heard, page_port, post, request, say};
use crate::support::head::Head;
use crate::support::spawning::free_port;
use crate::support::*;
use miyu_im::serve::Notice;

#[tokio::test(flavor = "multi_thread")]
async fn the_page_files_are_served_to_local_addresses_only() {
    let home = Home::new(&Script::new([]));
    let (serve, relay) = serve_pushing(home.root.clone(), settings());
    let bridge = start(serve).await;
    let port = page_port(&bridge, 1).await;
    let index = get(port, "/").await;
    assert_eq!(index.status, 200);
    assert_eq!(
        index.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(index.header("cache-control"), Some("no-store"));
    assert_eq!(index.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(
        index.header("content-security-policy"),
        Some("default-src 'self'; img-src 'self' blob: data:; frame-ancestors 'none'")
    );
    assert!(
        String::from_utf8_lossy(&index.body).starts_with("<!doctype html>"),
        "包文件夹 site/ 的 index.html"
    );
    let script = get(port, "/app.js").await;
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("content-type"),
        Some("text/javascript; charset=utf-8")
    );
    for missing in ["/nothing.js", "/../package.toml", "/.hidden", "/a/b.js"] {
        assert_eq!(get(port, missing).await.status, 404, "{missing}");
    }
    for host in ["localhost", "[::1]"] {
        let host = format!("{host}:{port}");
        let answer = request(port, ("GET", "/", &host), &[], b"").await;
        assert_eq!(answer.status, 200, "{host}");
    }
    for host in [
        format!("evil.example:{port}"),
        format!("127.0.0.1:{}", port.wrapping_add(1)),
        "127.0.0.1".to_string(),
    ] {
        let answer = request(port, ("GET", "/", &host), &[], b"").await;
        assert_eq!(answer.status, 421, "{host}");
    }
    let host = format!("127.0.0.1:{port}");
    let crossed = request(
        port,
        ("POST", "/login/ticket", &host),
        &["Origin: http://evil.example"],
        b"{}",
    )
    .await;
    assert_eq!(crossed.status, 403, "别的网站跨站调");
    let put = request(port, ("PUT", "/", &host), &[], b"").await;
    assert_eq!(put.status, 405);
    let status = page_status(&relay, "web").await;
    assert_eq!(
        status["web"], port,
        "后台页的 status 带页面服务的端口：{status}"
    );
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_ticket_logs_the_page_in_once() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let mut head = Head::connect(&home.root).await;
    let ticket = head.call("login.ticket", json!({})).await["result"]["ticket"].clone();
    assert!(ticket.is_string(), "头要得到票：{ticket}");
    let origin = format!("Origin: http://127.0.0.1:{port}");
    let logged = post(
        port,
        "/login/ticket",
        &[&origin],
        &json!({"ticket": ticket}),
    )
    .await;
    assert_eq!(
        logged.status,
        200,
        "{:?}",
        String::from_utf8_lossy(&logged.body)
    );
    let answer = logged.json();
    assert_eq!(
        answer.as_object().map(|o| o.len()),
        Some(1),
        "只交回会话号：{answer}"
    );
    let session = answer["session"].as_str().expect("交回会话号").to_string();
    assert_eq!(session.len(), 64, "{session}");
    let again = post(port, "/login/ticket", &[], &json!({"ticket": ticket})).await;
    assert_eq!(again.status, 401, "票只能用一次");
    assert_eq!(again.json(), json!({"reason": "bad_ticket"}));
    let made_up = post(
        port,
        "/login/ticket",
        &[],
        &json!({"ticket": "0".repeat(64)}),
    )
    .await;
    assert_eq!(made_up.status, 401);
    for body in [json!({}), json!({"ticket": 1}), json!("ticket")] {
        let bad = post(port, "/login/ticket", &[], &body).await;
        assert_eq!(bad.status, 400, "{body}");
        assert_eq!(bad.json(), json!({"reason": "bad_params"}));
    }
    let host = format!("127.0.0.1:{port}");
    let big = vec![b' '; 5000];
    let large = request(port, ("POST", "/login/ticket", &host), &[], &big).await;
    assert_eq!(large.status, 413, "正文最多 4096 字节");
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_page_opened_on_this_machine_logs_in_without_a_password() {
    // 施工 I-167补（核心 LP-1）：本机直连、带自定义头的页面不要密码，im 向核心要一张本机页面的票换登录；缺自定义头、带了反向代理头的
    // 一律不认，回 403 not_local，不去要票。
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let origin = format!("Origin: http://127.0.0.1:{port}");
    let mark = "X-Miyu-Page: local";
    let logged = post(port, "/login/local", &[&origin, mark], &json!({})).await;
    let answer = logged.json();
    assert_eq!(logged.status, 200, "{answer}");
    assert_eq!(
        answer["session"].as_str().map(str::len),
        Some(64),
        "交回会话号：{answer}"
    );
    for headers in [
        vec![origin.as_str()],
        vec![origin.as_str(), mark, "X-Forwarded-For: 192.168.1.9"],
        vec![origin.as_str(), mark, "Forwarded: for=192.168.1.9"],
        vec![mark, "Origin: http://evil.example"],
    ] {
        let refused = post(port, "/login/local", &headers, &json!({})).await;
        assert!(
            refused.status == 403 || refused.status == 421,
            "{headers:?} {}",
            refused.status
        );
    }
    let missing = post(port, "/login/local", &[&origin], &json!({})).await;
    assert_eq!(missing.json(), json!({"reason": "not_local"}));
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_channel_answers_a_logged_in_page_until_it_logs_out() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let mut head = Head::connect(&home.root).await;
    let ticket = head.call("login.ticket", json!({})).await["result"]["ticket"].clone();
    let session = post(port, "/login/ticket", &[], &json!({"ticket": ticket}))
        .await
        .json()["session"]
        .as_str()
        .expect("登录了")
        .to_string();
    let origin = format!("http://127.0.0.1:{port}");
    let mut open = channel(port, Some(&origin)).await;
    say(&mut open, &json!({"session": session})).await;
    assert_eq!(heard(&mut open).await, Ok(json!({"ready": true})));
    say(&mut open, &json!({"id": 1, "method": "connections"})).await;
    let answered = heard(&mut open).await.expect("答了");
    assert!(answered["result"]["connections"].is_array(), "{answered}");
    say(&mut open, &json!({"id": "x", "method": "nothing"})).await;
    assert_eq!(
        heard(&mut open).await,
        Ok(json!({"id": "x", "error": {"reason": "unknown_method"}}))
    );
    let wrong = post(port, "/logout", &["Authorization: Bearer nope"], &json!({})).await;
    assert_eq!(wrong.status, 204, "没有这一份也是 204");
    say(&mut open, &json!({"id": 2, "method": "connections"})).await;
    assert!(heard(&mut open).await.is_ok(), "登出别的会话号不碍事");
    let bearer = format!("Authorization: Bearer {session}");
    let out = post(port, "/logout", &[&bearer], &json!({})).await;
    assert_eq!(out.status, 204);
    say(&mut open, &json!({"id": 3, "method": "connections"})).await;
    assert_eq!(heard(&mut open).await, Err(4401), "登出以后的请求回关闭帧");
    let mut stale = channel(port, None).await;
    say(&mut stale, &json!({"session": session})).await;
    assert_eq!(heard(&mut stale).await, Err(4401), "登出了的会话号不认");
    let mut nameless = channel(port, None).await;
    say(&mut nameless, &json!({"hello": 1})).await;
    assert_eq!(heard(&mut nameless).await, Err(4401), "第一帧没有会话号");
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_page_port_replaces_the_old_one_and_a_taken_one_keeps_it() {
    let home = Home::new(&Script::new([]));
    let (serve, relay) = serve_pushing(home.root.clone(), settings());
    let bridge = start(serve).await;
    let first = page_port(&bridge, 1).await;
    let wanted = free_port();
    relay.config(json!({"im.web.port": wanted}));
    let second = page_port(&bridge, 2).await;
    assert_eq!(second, wanted, "照推来的端口开");
    assert_eq!(get(second, "/").await.status, 200);
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", first))
            .await
            .is_err(),
        "旧的端口换掉了"
    );
    let taken = std::net::TcpListener::bind("127.0.0.1:0").expect("挑得到");
    let taken_port = taken.local_addr().expect("有地址").port();
    relay.config(json!({"im.web.port": taken_port}));
    within("说端口被占了", async {
        loop {
            let said = bridge.notices.lock().expect("没 panic").clone();
            if said.contains(&Notice::PageInUse { port: taken_port }) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert_eq!(get(second, "/").await.status, 200, "开不了新的，旧的照旧");
    let status = page_status(&relay, "web-taken").await;
    assert_eq!(status["web"], second, "{status}");
    drop(taken);
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn opening_to_the_lan_keeps_the_port_and_takes_ip_hosts() {
    // 施工 I-167：设置页「向局域网开放」写 im.web.listen = 0.0.0.0，同一个端口换成听所有地址（旧的先放下，不然开不上），写 IP 的
    // Host 也收；关了照旧只认本机的写法。
    let home = Home::new(&Script::new([]));
    let (serve, relay) = serve_pushing(home.root.clone(), settings());
    let bridge = start(serve).await;
    page_port(&bridge, 1).await;
    // 测具出厂的端口是 0（系统挑）：先定一个，换地址时才是同一个端口。
    let port = free_port();
    relay.config(json!({"im.web.port": port}));
    assert_eq!(page_port(&bridge, 2).await, port);
    let lan_host = format!("192.168.1.9:{port}");
    let refused = request(port, ("GET", "/", &lan_host), &[], b"").await;
    assert_eq!(refused.status, 421, "只听本机时不收写 IP 的 Host");
    relay.config(json!({"im.web.listen": "0.0.0.0"}));
    assert_eq!(page_port(&bridge, 3).await, port, "同一个端口");
    assert_eq!(get(port, "/").await.status, 200, "本机照样打得开");
    let taken = request(port, ("GET", "/", &lan_host), &[], b"").await;
    assert_eq!(taken.status, 200, "向局域网开放后收写 IP 的 Host");
    let named = request(
        port,
        ("GET", "/", &format!("evil.example:{port}")),
        &[],
        b"",
    )
    .await;
    assert_eq!(named.status, 421, "域名照旧不收");
    relay.config(json!({"im.web.listen": "127.0.0.1"}));
    assert_eq!(page_port(&bridge, 4).await, port);
    let again = request(port, ("GET", "/", &lan_host), &[], b"").await;
    assert_eq!(again.status, 421, "关了照旧只认本机");
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_channel_lists_connections_and_says_when_they_change() {
    // 施工 I-6 上：连着的机器人（页面的连接卡），NapCat 连上、问到实现都推一帧 `changed`，页面照它再问。
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let mut head = Head::connect(&home.root).await;
    let ticket = head.call("login.ticket", json!({})).await["result"]["ticket"].clone();
    let session = post(port, "/login/ticket", &[], &json!({"ticket": ticket}))
        .await
        .json()["session"]
        .as_str()
        .expect("登录了")
        .to_string();
    let mut open = channel(port, None).await;
    say(&mut open, &json!({"session": session})).await;
    assert!(heard(&mut open).await.is_ok());
    say(&mut open, &json!({"id": 1, "method": "connections"})).await;
    assert_eq!(
        heard(&mut open).await,
        Ok(json!({"id": 1, "result": {"connections": []}}))
    );
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("在 1970 年以后")
        .as_millis();
    let mut napcat = crate::support::napcat::admin_napcat(bridge.port).await;
    napcat.version().await;
    let listed = within("连接卡带上实现", async {
        let mut asked = 1;
        let mut pushed = 0;
        loop {
            let frame = heard(&mut open).await.expect("通道开着");
            if frame == json!({"event": "changed"}) {
                // 变了：再问一遍。
                pushed += 1;
                asked += 1;
                say(&mut open, &json!({"id": asked, "method": "connections"})).await;
                continue;
            }
            let connections = frame["result"]["connections"].clone();
            if connections[0]["implementation"].is_string() {
                assert!(pushed > 0, "是推了变了才问到的");
                return connections;
            }
        }
    })
    .await;
    let since = listed[0]["since"].as_u64().expect("连上的时刻");
    // 平台做得到的几样（施工 I-85 上）：OneBot 除了命令菜单（施工 I-183 下上，QQ 没有）什么都做得到。
    let caps: Vec<&str> = miyu_im::chat::CAPS
        .iter()
        .filter(|cap| **cap != miyu_im::chat::Cap::Menu)
        .map(|cap| cap.name())
        .collect();
    assert!(u128::from(since) + 1000 >= before, "{listed}");
    assert_eq!(
        listed,
        json!([{"id": format!("qq:{BOT}"), "platform": "qq", "bot": BOT.to_string(), "connected": true, "since": since, "caps": caps, "implementation": "NapCat.Onebot", "version": "4.8.0", "avatar": format!("/avatar/{BOT}"), "listen": bridge.port}])
    );
    bridge.stop().await.expect("停得下");
}
