//! NapCat 反连进来的那一下（施工 O-8，`onebot.md` 第一条「怎么走」第 2 条）：令牌三种出示法都认，不对的、没出示的 401；
//! 只认 `/onebot/v11/ws` 和 `/ws`，别的 404。端口被占了说是哪个端口（第 1 条）。听哪个地址照配置（施工 I-27）。

use miyu_im::serve::{Failure, Notice, run};
use miyu_im_onebot::settings::Settings;
use miyu_session::testkit::Script;

use crate::support::*;

#[tokio::test]
async fn the_token_is_taken_three_ways() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    for (path, auth) in [
        ("/onebot/v11/ws", Auth::Bearer(TOKEN)),
        ("/ws", Auth::Token(TOKEN)),
        ("/onebot/v11/ws", Auth::Query(TOKEN)),
    ] {
        let napcat = napcat(bridge.port, path, auth, Some(BOT)).await;
        assert!(napcat.is_ok(), "{path} {auth:?}");
        if let Ok(napcat) = napcat {
            napcat.close().await;
        }
    }
    within("说连上了", async {
        while !bridge
            .notices
            .lock()
            .expect("没 panic")
            .contains(&Notice::Connected { bot: Some(BOT) })
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await;
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn a_wrong_or_missing_token_is_refused() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    let near = &TOKEN[..TOKEN.len() - 1];
    let longer = format!("{TOKEN}x");
    for auth in [
        Auth::Bearer("wrong"),
        Auth::Bearer(near),
        Auth::Bearer(&longer),
        Auth::Bearer(""),
        Auth::Token("wrong"),
        Auth::Query("wrong"),
        Auth::Query(""),
        Auth::Nothing,
    ] {
        let refused = napcat(bridge.port, "/onebot/v11/ws", auth, Some(BOT)).await;
        assert_eq!(refused.err(), Some(401), "{auth:?}");
    }
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn other_paths_are_not_found() {
    let home = Home::new(&Script::new([]));
    let bridge = bridge(&home).await;
    for path in [
        "/",
        "/onebot/v11",
        "/onebot/v11/ws/",
        "/ws/",
        "/api",
        "/onebot/v11/wsx",
    ] {
        let refused = napcat(bridge.port, path, Auth::Bearer(TOKEN), Some(BOT)).await;
        assert_eq!(refused.err(), Some(404), "{path}");
    }
    bridge.stop().await.expect("停得下");
}

#[tokio::test]
async fn a_port_in_use_is_named() {
    let home = Home::new(&Script::new([]));
    let taken = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("挑得到");
    let port = taken.local_addr().expect("有地址").port();
    let serve = serve(home.root.clone(), Settings { port, ..settings() });
    let ran = within("起不来", run(serve, |_| {}, |_| {}, std::future::pending())).await;
    assert_eq!(ran, Err(Failure::PortInUse(port)));
}

/// 这台机器对外的一个地址（不是回环的）：照系统给的出口挑，UDP 只连不发。没有的（没联网的机器）是空的。
fn outward() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

#[tokio::test]
async fn the_listen_address_decides_who_can_reach_it() {
    // 施工 I-27：0.0.0.0 别的机器也连得进来（这里拿本机对外的地址代替别的机器），127.0.0.1 只收本机。
    let Some(outward) = outward() else {
        eprintln!("这台机器没有对外的地址，跳过");
        return;
    };
    for (host, reachable) in [("0.0.0.0", true), ("127.0.0.1", false)] {
        let home = Home::new(&Script::new([]));
        let host = host.parse().expect("是地址");
        let bridge = start(serve(home.root.clone(), Settings { host, ..settings() })).await;
        let listening = Notice::Listening {
            host: host.to_string(),
            port: bridge.port,
        };
        assert!(
            bridge
                .notices
                .lock()
                .expect("没 panic")
                .contains(&listening),
            "说在哪个地址等：{host}"
        );
        let reached = tokio::net::TcpStream::connect((outward, bridge.port))
            .await
            .is_ok();
        assert_eq!(reached, reachable, "听 {host} 时从 {outward} 连");
        bridge.stop().await.expect("停得下");
    }
}
