//! 赞助记录的程序（施工 S-1 上b）：测试扮核心，经管道拉起真的 `miyu-im-sponsor serve`，照协议握手、收 `provide`、发 `tool.call`、
//! 答它回头问 im 的 `package.call`；汇率照本机的假服务（`MIYU_SPONSOR_RATES_URL`）。管理员叫的那一轮记上美元、换成人民币；
//! 普通成员叫的记不上；后台页经 `method.call` 查得到合计（施工 S-1 下）；关了标准输入程序照常退出，账本落了盘。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};

/// 等一行最多多久。
const WAIT: Duration = Duration::from_secs(30);

/// 本机的假汇率服务：谁问都答 1 USD = 7.144 CNY。交回地址（后面接币种）。
fn rates() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("听得上");
    let port = listener.local_addr().expect("有端口").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut seen = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !seen.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => seen.extend_from_slice(&buffer[..read]),
                }
            }
            let body = r#"{"result":"success","rates":{"CNY":7.144,"USD":1}}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            drop(stream.write_all(reply.as_bytes()));
        }
    });
    format!("http://127.0.0.1:{port}/v6/latest/")
}

/// 拉起的程序：写它的标准输入，一行行读它的标准输出。
struct Sponsor {
    child: Child,
    input: Option<ChildStdin>,
    lines: Receiver<Value>,
}

impl Sponsor {
    /// 在 `dir`（包的状态目录）里拉起。
    fn start(dir: &std::path::Path) -> Sponsor {
        let mut child = Command::new(env!("CARGO_BIN_EXE_miyu-im-sponsor"))
            .arg("serve")
            .current_dir(dir)
            .env("MIYU_SPONSOR_RATES_URL", rates())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("拉得起");
        let output = child.stdout.take().expect("有标准输出");
        let (tell, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && tell.send(value).is_err()
                {
                    break;
                }
            }
        });
        let input = child.stdin.take();
        Sponsor {
            child,
            input,
            lines,
        }
    }

    /// 下一行。
    fn next(&self) -> Value {
        self.lines.recv_timeout(WAIT).expect("等得到它说话")
    }

    /// 写一行。
    fn send(&mut self, line: &Value) {
        let input = self.input.as_mut().expect("还开着");
        writeln!(input, "{line}").expect("写得进");
        input.flush().expect("冲得出");
    }

    /// 等它发来方法是 `method` 的请求，答 `result`：交回那一条请求。
    fn answers(&mut self, method: &str, result: &Value) -> Value {
        let asked = self.next();
        assert_eq!(asked["method"], method, "{asked}");
        self.send(&json!({"jsonrpc": "2.0", "id": asked["id"], "result": result}));
        asked
    }

    /// 发一次 `tool.call`（编号 `id`，谁叫的 `by`、是不是主人 `owner`，参数 `args`），答它问 im 的 `chat.venue`：交回工具的结果。
    fn calls(&mut self, id: &str, (by, owner): (Value, bool), args: Value) -> Value {
        let params = json!({"session": "s1", "call_id": id, "tool": "sponsor", "args": args, "cwd": ".", "by": by, "owner": owner});
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": "tool.call", "params": params}));
        let venue = json!({"platform": "qq", "venue": "qq:group:777", "bot": 10000, "origin": {
            "sender": {"id": "qq:9", "name": "老板"},
            "quote": {"msg": "5", "sender": {"id": "qq:40001", "name": "小明"}},
            "mentions": [],
        }});
        let asked = self.answers("package.call", &venue);
        assert_eq!(
            asked["params"],
            json!({"package": "miyu-im", "method": "chat.venue", "params": {"session": "s1"}})
        );
        let answer = self.next();
        assert_eq!(answer["id"], id, "{answer}");
        answer["result"].clone()
    }
}

#[test]
fn it_shakes_hands_provides_the_tool_and_records_only_for_an_admin() {
    let dir = std::env::temp_dir().join(format!("im-sponsor-serve-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).expect("建得了");
    let mut sponsor = Sponsor::start(&dir);
    let hello = sponsor.answers(
        "hello",
        &json!({"config": {"miyu-im-sponsor.currency": "CNY"}}),
    );
    assert_eq!(hello["params"]["protocol"], json!([1, 1]));
    let provided = sponsor.answers("provide", &json!({"tools": 1}));
    let tool = &provided["params"]["tools"][0];
    assert_eq!(
        (&tool["name"], &tool["venues"]),
        (&json!("sponsor"), &json!(["private", "group"]))
    );
    assert!(tool["description"].is_string() && tool["input_schema"]["type"] == "object");
    assert_eq!(
        tool["plugin"], "miyu-im-sponsor",
        "关着的场所没有它（核心 O-60）"
    );
    // 后台页的方法（施工 S-1 下）。
    let methods = sponsor.answers("package.methods", &json!({"methods": 7}));
    assert!(
        methods["params"]["methods"]
            .as_array()
            .is_some_and(|all| all.iter().any(|one| one["name"] == "totals")),
        "{methods}"
    );

    // 场所的管理的人叫的：没写号，照引用的小明记，美元照假服务的汇率换成人民币。
    let manager =
        json!({"kind": "external", "venue": "qq:group:777", "id": "qq:9", "role": "manager"});
    let added = sponsor.calls(
        "c1",
        (manager, false),
        json!({"action": "add", "amount": 10, "currency": "USD"}),
    );
    assert_eq!(added["error"], false, "{added}");
    assert_eq!(
        added["blocks"][0]["text"],
        "Recorded #1: 小明 (qq:40001) gave 10 USD (= 71.44 CNY at 7.144)."
    );
    // 普通成员叫的：不记。
    let member =
        json!({"kind": "external", "venue": "qq:group:777", "id": "qq:3", "role": "member"});
    let refused = sponsor.calls("c2", (member, false), json!({"action": "add", "amount": 5}));
    assert_eq!(refused["error"], true, "{refused}");

    // 后台页查合计：只有管理员记的那一笔，换成了人民币。
    sponsor.send(&json!({"jsonrpc": "2.0", "id": "m1", "method": "method.call", "params": {"method": "totals", "params": {}}}));
    let totals = sponsor.next();
    assert_eq!(totals["id"], "m1", "{totals}");
    assert_eq!(
        (&totals["result"]["all"], &totals["result"]["pending"]),
        (&json!(71.44), &json!(0)),
        "{totals}"
    );

    // 关了标准输入：照常退出，账本落了盘，只有那一笔。
    sponsor.input = None;
    let status = sponsor.child.wait().expect("等得到退出");
    assert!(status.success(), "{status}");
    let book: Value =
        serde_json::from_slice(&std::fs::read(dir.join("sponsors.json")).expect("落了盘"))
            .expect("是 JSON");
    assert_eq!(book["entries"].as_array().map(Vec::len), Some(1), "{book}");
    assert_eq!(book["entries"][0]["rate"], 7.144);
    std::fs::remove_dir_all(dir).expect("删得掉");
}
