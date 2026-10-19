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
        Some("default-src 'self'; frame-ancestors 'none'")
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
    assert_eq!(answer["account"], "admin", "{answer}");
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
    assert_eq!(heard(&mut open).await, Ok(json!({"account": "admin"})));
    say(&mut open, &json!({"id": 1, "method": "whoami"})).await;
    assert_eq!(
        heard(&mut open).await,
        Ok(json!({"id": 1, "result": {"account": "admin"}}))
    );
    say(&mut open, &json!({"id": "x", "method": "nothing"})).await;
    assert_eq!(
        heard(&mut open).await,
        Ok(json!({"id": "x", "error": {"reason": "unknown_method"}}))
    );
    let wrong = post(port, "/logout", &["Authorization: Bearer nope"], &json!({})).await;
    assert_eq!(wrong.status, 204, "没有这一份也是 204");
    say(&mut open, &json!({"id": 2, "method": "whoami"})).await;
    assert!(heard(&mut open).await.is_ok(), "登出别的会话号不碍事");
    let bearer = format!("Authorization: Bearer {session}");
    let out = post(port, "/logout", &[&bearer], &json!({})).await;
    assert_eq!(out.status, 204);
    say(&mut open, &json!({"id": 3, "method": "whoami"})).await;
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
    relay.config(json!({"im.web_port": wanted}));
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
    relay.config(json!({"im.web_port": taken_port}));
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
