//! 测试用的别的包的程序（施工 O-43 上，`docs/blueprint/extending.md` 第二条）：集成测试把它链到测试程序旁边、写一份清单，当成
//! 另一个扩展包让核心拉起，导出挂接点的方法，照参数答接入QQ 经核心转来的调用。不随发版带出去（发版只打 `miyu-onebot`）。
//! 照核心的测试扩展（核心仓库 `crates/miyu-endpoint/src/bin/miyu-test-extension.rs`）的样子写，只认 `method.call`。
//!
//! 参数，照先后：
//!
//! - `record:<路径>`：收到的每一次 `method.call` 往这个文件追加一行：`{"method", "params", "from"}`。
//! - `answer:<方法>:<JSON>`：这个方法照这段 JSON 答（`result`）。
//! - `slow:<方法>`：这个方法收了不答。
//!
//! 起来先握手（不带凭据，核心亲手给的管道），再经 `package.methods` 登记写到的方法，然后一直读标准输入，读到头退出。

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
        }
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let hello = json!({"protocol": [1, 1], "head": {"kind": "test-hook", "version": "0"}});
    ask(&mut input, "hello", hello);
    let methods: Vec<Value> = answers.keys().map(|name| json!({"name": name})).collect();
    ask(&mut input, "package.methods", json!({"methods": methods}));
    loop {
        let line = read(&mut input);
        if line.is_empty() {
            return;
        }
        serve(&line, &answers, &mut record);
    }
}

/// 发一条请求，读到它的回应为止；中间核心说的别的（推送、反向的调用）照 [`serve`] 办，不记。
fn ask(input: &mut impl BufRead, method: &str, params: Value) {
    let id = format!("hook-{method}");
    send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
    loop {
        let line = read(input);
        assert!(!line.is_empty(), "等 {method} 的回应时核心关了管道");
        let message: Value = serde_json::from_str(&line).expect("核心说的是 JSON");
        if message["id"] == id.as_str() && message.get("method").is_none() {
            return;
        }
    }
}

/// 核心说的一行：是 `method.call` 的记下、照写的答。
fn serve(line: &str, answers: &BTreeMap<String, Answer>, record: &mut Option<File>) {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return;
    };
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
