//! 实时日志从桥推到页面通道（设计 10，施工 I-160 中）：真核心照开关拉起真桥，页面开始看一个群，群里有人叫她，她想一想再回；
//! 通道上推来收到、判断、开轮、思考、正文、发出这几行，写着的思考先推、写完再定下来；见过的场所列得出；停了不再推。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::browser::{call, heard, logged_in};
use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3933;

/// 群里的人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 系统的场所规则：只有一条配群的空规则。
const RULES: &str = "[[rule]]\nmatch = { kind = \"group\" }\n";

/// 一帧推来的实时日志里的每一行：`kind`，和行本身。
fn items(frame: &Value) -> Vec<Value> {
    frame["batch"]["lines"]
        .as_array()
        .map(|lines| lines.iter().map(|line| line["item"].clone()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_watched_group_streams_what_she_does() {
    let script = Script::new([Play::Thinks {
        thinking: "先想一想",
        text: "在的。",
    }]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    let board = miyu_im::status_file::read(&home.root).expect("写了状态文件");
    let port = u16::try_from(board["web"].as_u64().expect("页面的端口")).expect("是端口");
    let mut open = logged_in(&home.root, port).await;
    let venue = format!("qq:group:{GROUP}");
    let started = call(&mut open, 1, "watch.start", json!({"venue": venue})).await;
    assert_eq!(
        started["result"]["batches"],
        json!([]),
        "桥刚起来，什么都没留"
    );
    // 没叫她、只记下的也推：收到的那一行是桥交进会话以后自己补的（核心不推回来）。
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    let mut quiet = Vec::new();
    within("先记下了一条", async {
        loop {
            let frame = heard(&mut open).await.expect("通道开着");
            quiet.extend(items(&frame));
            if quiet.iter().any(|item| item["kind"] == "decided") {
                return;
            }
        }
    })
    .await;
    assert!(
        quiet
            .iter()
            .any(|item| item["kind"] == "heard" && item["text"] == "大家好"),
        "{quiet:?}"
    );
    let decided = quiet
        .iter()
        .find(|item| item["kind"] == "decided")
        .expect("有");
    assert_eq!(
        (decided["who"].as_str(), decided["outcome"].as_str()),
        (Some("小林"), Some("record")),
        "判断照序号找到名字"
    );
    napcat.send(group_frame(
        GROUP,
        LIN,
        3,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    ));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");
    let mut seen = Vec::new();
    within("推到发出那一行", async {
        loop {
            let frame = heard(&mut open).await.expect("通道开着");
            if frame["event"] != "watch" {
                continue;
            }
            assert_eq!(frame["batch"]["venue"], venue, "只推看着的这个群：{frame}");
            let delivered = items(&frame).iter().any(|item| item["kind"] == "delivered");
            seen.extend(items(&frame));
            if delivered {
                return;
            }
        }
    })
    .await;
    let kinds: Vec<&str> = seen
        .iter()
        .filter_map(|item| item["kind"].as_str())
        .collect();
    for kind in ["heard", "decided", "started", "wrote", "delivered"] {
        assert!(kinds.contains(&kind), "少了 {kind}：{kinds:?}");
    }
    let heard_line = seen
        .iter()
        .find(|item| item["kind"] == "heard")
        .expect("有");
    assert_eq!(
        (heard_line["who"].as_str(), heard_line["text"].as_str()),
        (Some("小林"), Some("@米尤 在吗"))
    );
    let thought = seen
        .iter()
        .find(|item| {
            item["kind"] == "wrote" && item["block"]["kind"] == "thinking" && item["done"] == true
        })
        .expect("思考定下来了");
    assert_eq!(thought["text"], "先想一想");
    let writing = seen.iter().any(|item| {
        item["kind"] == "appended" || (item["kind"] == "wrote" && item["done"] == false)
    });
    assert!(writing, "写着的时候推过：{kinds:?}");
    // 留着的：再开始看一次，定下来的都在，写着的不在。
    let again = call(&mut open, 2, "watch.start", json!({"venue": venue})).await;
    let kept: Vec<Value> = again["result"]["batches"]
        .as_array()
        .expect("是列表")
        .iter()
        .flat_map(|batch| batch["lines"].as_array().cloned().unwrap_or_default())
        .map(|line| line["item"].clone())
        .collect();
    assert!(
        kept.iter().any(|item| item["kind"] == "delivered"),
        "{kept:?}"
    );
    assert!(
        kept.iter()
            .all(|item| item["kind"] != "appended" && item["done"] != false),
        "写着的不留：{kept:?}"
    );
    let venues = call(&mut open, 3, "watch.venues", json!({})).await;
    assert!(
        venues["result"]["venues"]
            .as_array()
            .expect("是列表")
            .iter()
            .any(|one| one["venue"] == venue),
        "{venues}"
    );
    // 停了以后群里再说话，不推。
    call(&mut open, 4, "watch.stop", json!({})).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        4,
        json!([plain("没人叫她")]),
        ("小林", "lin"),
    ));
    let quiet = tokio::time::timeout(std::time::Duration::from_millis(800), async {
        loop {
            let frame = heard(&mut open).await.expect("通道开着");
            if frame["event"] == "watch" {
                return frame;
            }
        }
    })
    .await;
    assert!(quiet.is_err(), "停了还推：{quiet:?}");
}

/// 命令行印出来的：照写的时候拼起来。
#[derive(Clone, Default)]
struct Printed(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Printed {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("没崩").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Printed {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().expect("没崩").clone()).expect("UTF-8")
    }
}

#[tokio::test]
async fn the_command_follows_a_group_by_its_number() {
    let script = Script::new([Play::Thinks {
        thinking: "先想一想",
        text: "在的。",
    }]);
    let (home, mut napcat, _) = started(&script, RULES, "", MEMBERS).await;
    // 先有动静，命令行才认得 `qq:号`。
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("大家好")]),
        ("小林", "lin"),
    ));
    let board = miyu_im::status_file::read(&home.root).expect("写了状态文件");
    let port = u16::try_from(board["web"].as_u64().expect("页面的端口")).expect("是端口");
    let mut open = logged_in(&home.root, port).await;
    within("见过这个群", async {
        let mut asked = 10;
        loop {
            let venues = call(&mut open, asked, "watch.venues", json!({})).await;
            if venues["result"]["venues"]
                .as_array()
                .is_some_and(|all| !all.is_empty())
            {
                return;
            }
            asked += 1;
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    let texts = miyu_im::texts::Texts::load(
        miyu_store::resources::ResourceRoot::at(resources()),
        package_in(resources()),
        "zh",
    )
    .expect("读得出来");
    let (out, err) = (Printed::default(), Printed::default());
    let (mut printing, mut erring) = (out.clone(), err.clone());
    let venue = format!("qq:{GROUP}");
    let following = miyu_im::watch::follow::follow(
        &home.root,
        Some(&venue),
        &texts,
        &mut printing,
        &mut erring,
    );
    let driving = async {
        within("开始看了", async {
            while !out.text().contains("正在看") {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
        napcat.send(group_frame(
            GROUP,
            LIN,
            2,
            json!([at(BOT), plain(" 在吗")]),
            ("小林", "lin"),
        ));
        assert_eq!(napcat.group_reply(GROUP).await, "在的");
        within("印到发出", async {
            while !out.text().contains("发出：在的") {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
    };
    tokio::select! {
        code = following => panic!("命令先退出了：{code}，{}", err.text()),
        () = driving => {}
    }
    let printed = out.text();
    assert!(printed.starts_with("正在看 "), "{printed}");
    for expected in [
        "小林：大家好",
        "小林：@米尤 在吗",
        "▶ 开始回复 小林",
        "想：先想一想",
        "说：在的。",
        "发出：在的",
    ] {
        assert!(
            printed.contains(expected),
            "少了「{expected}」：\n{printed}"
        );
    }
    assert_eq!(err.text(), "");
}
