//! 测试用的别的包的程序（施工 O-43 上，`docs/blueprint/extending.md` 第二条）：集成测试把它链到测试程序旁边、写一份清单，当成
//! 另一个扩展包让核心拉起，导出挂接点的方法，照参数答接入通讯平台 经核心转来的调用。不随发版带出去（发版只打 `miyu-im`）。
//! 照核心的测试扩展（核心仓库 `crates/miyu-endpoint/src/bin/miyu-test-extension.rs`）的样子写，只认 `method.call`。
//!
//! 参数，照先后：
//!
//! - `record:<路径>`：收到的每一次 `method.call` 往这个文件追加一行：`{"method", "params", "from"}`。
//! - `answer:<方法>:<JSON>`：这个方法照这段 JSON 答（`result`）。
//! - `slow:<方法>`：这个方法收了不答。
//! - `sticker:<路径>`：把这个图经 `blob.put` 传上去，登记一件工具 `sticker`（施工 O-51：表情包包那样，私聊、群里用），她调了
//!   答一段字和 `sent` 里一张表情包（核心 X-19）。要清单里批过 `tools`。
//! - `sticker-end`：`sticker` 的回答带 `end_turn`（施工 O-57：她最后一步带着工具调用、这一轮到此结束，表情包包的 `meme_send` 那样）。
//! - `render:<路径>`：扮渲染包（施工 O-59 上）：把这个图经 `blob.put` 传上去，`markdown` 答两页、都是它。清单要导出 `markdown`。
//!
//! 起来先握手（不带凭据，核心亲手给的管道），再经 `package.methods` 登记写到的方法（有 `sticker:` 的另经 `provide` 登记那件
//! 工具），然后一直读标准输入，读到头退出。

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

/// 一个方法怎么答。
enum Answer {
    /// 照这段答。
    With(Value),
    /// 不答。
    Silent,
}

fn main() {
    let mut record: Option<File> = None;
    let mut answers: BTreeMap<String, Answer> = BTreeMap::new();
    let mut sticker: Option<String> = None;
    let mut render: Option<String> = None;
    let mut end = false;
    for arg in std::env::args().skip(1) {
        if let Some(path) = arg.strip_prefix("record:") {
            record = Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .expect("开得了记的文件"),
            );
        } else if let Some(rest) = arg.strip_prefix("answer:") {
            let (method, answer) = rest.split_once(':').expect("写成 answer:<方法>:<JSON>");
            let answer = serde_json::from_str(answer).expect("答的是 JSON");
            answers.insert(method.to_string(), Answer::With(answer));
        } else if let Some(method) = arg.strip_prefix("slow:") {
            answers.insert(method.to_string(), Answer::Silent);
        } else if let Some(path) = arg.strip_prefix("sticker:") {
            sticker = Some(path.to_string());
        } else if let Some(path) = arg.strip_prefix("render:") {
            render = Some(path.to_string());
        } else if arg == "sticker-end" {
            end = true;
        }
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let hello = json!({"protocol": [1, 1], "head": {"kind": "test-hook", "version": "0"}});
    ask(&mut input, "hello", hello);
    // 扮渲染包（施工 O-59 上）：图先传上去，`markdown` 照它答两页。
    if let Some(path) = render {
        let put = ask(&mut input, "blob.put", json!({"path": path}));
        let page = json!({"blob": put["result"]["blob"], "width": 1, "height": 1});
        answers.insert(
            "markdown".to_string(),
            Answer::With(json!({"pages": [page, page]})),
        );
    }
    let methods: Vec<Value> = answers.keys().map(|name| json!({"name": name})).collect();
    ask(&mut input, "package.methods", json!({"methods": methods}));
    // 发出去的那张图（施工 O-51）：传上去，交回的 `blob`、媒体类型、宽高照原样放进 `sent`。
    let sent = sticker.map(|path| {
        let put = ask(&mut input, "blob.put", json!({"path": path}));
        let result = &put["result"];
        let tool = json!({
            "name": "sticker",
            "description": "Send a sticker.",
            "input_schema": {"type": "object", "properties": {}},
            "access": "read",
            "venues": ["private", "group"],
        });
        ask(&mut input, "provide", json!({"tools": [tool]}));
        json!([{
            "blob": result["blob"],
            "media_type": result["media_type"],
            "width": result["width"],
            "height": result["height"],
            "alt": "A cat waving.",
            "sticker": true,
        }])
    });
    loop {
        let line = read(&mut input);
        if line.is_empty() {
            return;
        }
        serve(&line, &answers, &mut record, (sent.as_ref(), end));
    }
}

/// 发一条请求，读到它的回应为止，交回回应；中间核心说的别的（推送、反向的调用）不办。
fn ask(input: &mut impl BufRead, method: &str, params: Value) -> Value {
    let id = format!("hook-{method}");
    send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
    loop {
        let line = read(input);
        assert!(!line.is_empty(), "等 {method} 的回应时核心关了管道");
        let message: Value = serde_json::from_str(&line).expect("核心说的是 JSON");
        if message["id"] == id.as_str() && message.get("method").is_none() {
            return message;
        }
    }
}

/// 核心说的一行：是 `method.call` 的记下、照写的答；是工具 `sticker` 的 `tool.call` 的答一段字、`sent` 是传上去的那张（施工 O-51）。
fn serve(
    line: &str,
    answers: &BTreeMap<String, Answer>,
    record: &mut Option<File>,
    (sent, end): (Option<&Value>, bool),
) {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return;
    };
    if message["method"] == "tool.call"
        && let Some(sent) = sent
    {
        let mut result = json!({"blocks": [{"type": "text", "text": "Sent."}], "sent": sent});
        if end {
            result["end_turn"] = json!(true);
        }
        send(&json!({"jsonrpc": "2.0", "id": message["id"], "result": result}));
        return;
    }
    if message["method"] != "method.call" {
        return;
    }
    let params = &message["params"];
    if let Some(file) = record {
        let kept =
            json!({"method": params["method"], "params": params["params"], "from": params["from"]});
        writeln!(file, "{kept}").expect("写得进");
        file.flush().expect("写得出");
    }
    let method = params["method"].as_str().unwrap_or_default();
    match answers.get(method) {
        Some(Answer::With(result)) => {
            send(&json!({"jsonrpc": "2.0", "id": message["id"], "result": result}));
        }
        Some(Answer::Silent) => {}
        None => {
            let error = json!({"code": -32601, "message": "unknown_method"});
            send(&json!({"jsonrpc": "2.0", "id": message["id"], "error": error}));
        }
    }
}

/// 读一行；读到头的是空的。
fn read(input: &mut impl BufRead) -> String {
    let mut line = String::new();
    input.read_line(&mut line).expect("读得了标准输入");
    line
}

/// 写一行。
fn send(message: &Value) {
    let mut out = io::stdout().lock();
    writeln!(out, "{message}").expect("写得出");
    out.flush().expect("写得出");
}
