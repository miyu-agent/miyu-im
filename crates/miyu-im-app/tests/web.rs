//! `miyu im web`（施工 I-6 上）：带交接票打开 im 自己的页面。真核心照开关拉起真桥，测试照命令的样子调 `web`，浏览器是记下网址的
//! 替身：网址是页面服务的端口，片段里的票换得到登录（交给页面服务的 `/login/ticket`）；im 关着的先打开再开页面（施工 I-178），
//! 核心那边没有 im 的说一句、退出码 1。

use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};

use miyu_client::open::Browser;
use miyu_im::texts::Texts;
use miyu_im::web::web;
use miyu_session::testkit::Script;
use miyu_store::resources::ResourceRoot;

use crate::support::browser::post;
use crate::support::group::*;
use crate::support::spawning::{cli, ports_config_with, text};
use crate::support::*;

/// 记下交来的网址，照 `opens` 说打没打开。
struct Recorder {
    opens: bool,
    seen: Mutex<Vec<String>>,
}

impl Browser for Recorder {
    fn open(&self, url: &str) -> bool {
        self.seen.lock().expect("没 panic").push(url.to_string());
        self.opens
    }
}

/// 中文的字。
fn zh() -> Texts {
    Texts::load(ResourceRoot::at(resources()), package_in(resources()), "zh").expect("读得出来")
}

/// 跑 `web`：交回退出码、标准输出、标准错误。核心在测试里跑着，拉起核心的命令不该用上。
async fn ran(home: &Home, browser: &Recorder) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let launch: miyu_im::control::CoreCommand =
        std::sync::Arc::new(|| std::process::Command::new("miyu-im-test-no-core"));
    let code = web(&home.root, &launch, &zh(), browser, (&mut out, &mut err)).await;
    (
        code,
        String::from_utf8(out).expect("是字"),
        String::from_utf8(err).expect("是字"),
    )
}

#[tokio::test]
async fn the_page_opens_with_a_ticket_that_logs_in() {
    let (home, _napcat, _) = started(&Script::new([]), "", "", &[]).await;
    let browser = Recorder {
        opens: true,
        seen: Mutex::new(Vec::new()),
    };
    let (code, out, err) = ran(&home, &browser).await;
    assert_eq!(code, 0, "{err}");
    let url = browser.seen.lock().expect("没 panic")[0].clone();
    assert_eq!(out, format!("已在浏览器中打开：{url}\n"));
    let board = miyu_im::status_file::read(&home.root).expect("写了状态文件");
    let port = u16::try_from(board["web"].as_u64().expect("有页面的端口")).expect("是端口");
    let (base, fragment) = url.split_once("#miyu=").expect("片段带交接票");
    assert_eq!(base, format!("http://127.0.0.1:{port}/"));
    let handed: Value =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(fragment).expect("是 base64url"))
            .expect("是 JSON");
    assert_eq!(handed["v"], 1, "{handed}");
    assert_eq!(handed["lang"], "zh", "{handed}");
    let ticket = handed["ticket"].as_str().expect("有票");
    let logged = post(port, "/login/ticket", &[], &json!({"ticket": ticket})).await;
    assert_eq!(
        logged.status,
        200,
        "{}",
        String::from_utf8_lossy(&logged.body)
    );
    assert!(logged.json()["session"].is_string(), "换到了登录");
    // 浏览器打不开的：照样印网址，说一分钟内有效。
    let unopened = Recorder {
        opens: false,
        seen: Mutex::new(Vec::new()),
    };
    let (code, out, _) = ran(&home, &unopened).await;
    assert_eq!(code, 0);
    let url = unopened.seen.lock().expect("没 panic")[0].clone();
    assert_eq!(
        out,
        format!("在浏览器中打开以下地址（1 分钟内有效）：{url}\n")
    );
    stopped(home).await;
}

#[tokio::test]
async fn when_im_is_off_it_starts_it_then_opens_the_page() {
    // 施工 I-178（项目主人 11-09）：原来说一句「先 `miyu im start`」就退出；现在照 `start` 打开开关、等页面服务听上再开。
    let (home, _napcat, _) = started(&Script::new([]), "", "", &[]).await;
    let off = cli(&home.root, &["stop"]).await;
    assert_eq!(off.status.code(), Some(0), "{}", text(&off.stderr));
    let browser = Recorder {
        opens: true,
        seen: Mutex::new(Vec::new()),
    };
    let (code, out, err) = ran(&home, &browser).await;
    assert_eq!(code, 0, "{err}");
    let url = browser.seen.lock().expect("没 panic")[0].clone();
    assert_eq!(
        out,
        format!("已启动接入通讯平台\n已在浏览器中打开：{url}\n")
    );
    let board = miyu_im::status_file::read(&home.root).expect("写了状态文件");
    let port = board["web"].as_u64().expect("有页面的端口");
    assert!(
        url.starts_with(&format!("http://127.0.0.1:{port}/#miyu=")),
        "{url}"
    );
    stopped(home).await;
}

#[tokio::test]
async fn when_im_cannot_start_it_says_why() {
    // 施工 I-178：打开了开关，im 起来就停下（NapCat 的端口被占了）：照 `status` 说为什么，不开浏览器，退出码 1。
    let holding = std::net::TcpListener::bind("127.0.0.1:0").expect("挑得到");
    let taken = holding.local_addr().expect("有地址").port();
    let home = Home::spawning(&Script::new([]), &ports_config_with(taken, ""));
    let browser = Recorder {
        opens: true,
        seen: Mutex::new(Vec::new()),
    };
    let (code, out, err) = ran(&home, &browser).await;
    assert_eq!(code, 1, "{out}{err}");
    assert_eq!(out, "已启动接入通讯平台\n", "{err}");
    assert!(err.starts_with("接入通讯平台已停止"), "{err}");
    assert!(err.contains(&format!("端口 {taken} 已被占用")), "{err}");
    assert!(
        browser.seen.lock().expect("没 panic").is_empty(),
        "没开浏览器"
    );
    drop(holding);
    within("核心停下扩展", home.stop_extensions()).await;
}
