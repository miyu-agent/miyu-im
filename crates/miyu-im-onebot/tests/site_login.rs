//! im 自己的页面用用户名、密码登录（施工 I-5 下，设计 01 第九节第 2 条）：进程里跑的桥经本机套接字连真核心。头照网页第一次进来
//! 的样子给 `admin` 设好用户名、密码，测试照浏览器的样子登录、开通道、登出；登出时 im 在人的连接上作废那个登录令牌（核心的
//! `home/admin/logins.json` 少一行）。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::browser::{channel, heard, page_port, post, say};
use crate::support::head::Head;
use crate::support::*;

/// 设的用户名、密码。
const USER: &str = "mio";
const PASSWORD: &str = "correct horse battery";

/// 核心记着的登录令牌有几个。
fn tokens(home: &Home) -> usize {
    let path = home.root.path().join("home/admin/logins.json");
    let logins: Value =
        serde_json::from_slice(&std::fs::read(path).expect("读得了")).expect("是 JSON");
    logins["tokens"].as_array().expect("有令牌表").len()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_password_logs_the_page_in_and_logging_out_revokes_it() {
    let home = Home::new(&Script::new([]));
    Head::set_password(&home.root, USER, PASSWORD).await;
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let before = tokens(&home);
    let origin = format!("Origin: http://127.0.0.1:{port}");
    let logged = post(
        port,
        "/login/password",
        &[&origin],
        &json!({"user": USER, "password": PASSWORD}),
    )
    .await;
    assert_eq!(
        logged.status,
        200,
        "{}",
        String::from_utf8_lossy(&logged.body)
    );
    let answer = logged.json();
    assert_eq!(
        answer["account"], "admin",
        "用户名换成核心认的账号：{answer}"
    );
    let session = answer["session"].as_str().expect("交回会话号").to_string();
    assert_eq!(tokens(&home), before + 1, "核心换了一个登录令牌");
    let mut open = channel(port, None).await;
    say(&mut open, &json!({"session": session})).await;
    assert_eq!(heard(&mut open).await, Ok(json!({"account": "admin"})));
    let bearer = format!("Authorization: Bearer {session}");
    assert_eq!(
        post(port, "/logout", &[&bearer], &json!({})).await.status,
        204
    );
    assert_eq!(tokens(&home), before, "登出作废了那个登录令牌");
    say(&mut open, &json!({"id": 1, "method": "whoami"})).await;
    assert_eq!(heard(&mut open).await, Err(4401));
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_password_or_body_is_refused() {
    let home = Home::new(&Script::new([]));
    Head::set_password(&home.root, USER, PASSWORD).await;
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let wrong = post(
        port,
        "/login/password",
        &[],
        &json!({"user": USER, "password": "nope nope nope"}),
    )
    .await;
    assert_eq!(wrong.status, 401);
    assert_eq!(wrong.json(), json!({"reason": "bad_password"}));
    for body in [
        json!({"user": USER}),
        json!({"password": PASSWORD}),
        json!({"user": 1, "password": PASSWORD}),
        json!([USER, PASSWORD]),
    ] {
        let bad = post(port, "/login/password", &[], &body).await;
        assert_eq!(bad.status, 400, "{body}");
        assert_eq!(bad.json(), json!({"reason": "bad_params"}));
    }
    bridge.stop().await.expect("停得下");
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_wrong_passwords_wait_before_trying_again() {
    let home = Home::new(&Script::new([]));
    Head::set_password(&home.root, USER, PASSWORD).await;
    let bridge = bridge(&home).await;
    let port = page_port(&bridge, 1).await;
    let wrong = json!({"user": USER, "password": "nope nope nope"});
    let throttled = within("错多了被挡", async {
        loop {
            let answer = post(port, "/login/password", &[], &wrong).await;
            if answer.status == 429 {
                return answer.json();
            }
            assert_eq!(
                answer.status,
                401,
                "{}",
                String::from_utf8_lossy(&answer.body)
            );
        }
    })
    .await;
    assert_eq!(throttled["reason"], "login_throttled", "{throttled}");
    assert!(
        throttled["retry_after_ms"].is_u64(),
        "带还要等多久：{throttled}"
    );
    bridge.stop().await.expect("停得下");
}
