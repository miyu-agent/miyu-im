//! `miyu im web`（施工 I-6 上）：带交接票打开 im 自己的页面。真核心照开关拉起真桥，测试照命令的样子调 `web`，浏览器是记下网址的
//! 替身：网址是页面服务的端口，片段里的票换得到登录（交给页面服务的 `/login/ticket`）；im 没在跑的说一句、退出码 1。

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

/// 跑 `web`：交回退出码、标准输出、标准错误。
async fn ran(home: &Home, browser: &Recorder) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = web(&home.root, &zh(), browser, &mut out, &mut err).await;
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
async fn without_im_running_it_says_to_start_it() {
    let home = Home::new(&Script::new([]));
    let browser = Recorder {
        opens: true,
        seen: Mutex::new(Vec::new()),
    };
    let (code, out, err) = ran(&home, &browser).await;
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "miyu im: 接入通讯平台未运行\n提示：用 'miyu im start' 启动\n"
    );
    assert!(
        browser.seen.lock().expect("没 panic").is_empty(),
        "没开浏览器"
    );
}
