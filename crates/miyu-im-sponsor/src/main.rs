//! miyu-im-sponsor：赞助记录（设计 03，施工 S-1 上b）。im 的插件，独立的软件包：核心照清单 `[process] args = ["serve"]` 拉起，
//! 标准输入输出上说协议；工作目录是核心给的这个包放状态的目录，账本放在那里（`sponsors.json`）。
//!
//! 起来先握手（核心交设置的最终值），经 `provide` 登记工具 `sponsor`（只给场所会话：群、私聊），然后一直读标准输入，读到头退出。
//! 每次 `tool.call` 先经 `package.call` 问 im 的 `chat.venue`：是哪个平台、哪个场所、叫她的那一条引用了谁、@ 了谁（S-1 上a）。
//! 后台页（`sponsor/page/`）经 `package.methods` 登记的方法读、改账本（施工 S-1 下，`methods.rs`）。

mod methods;
mod money;
mod rates;
mod settings;
mod store;
mod texts;
mod tool;
mod wire;

use std::io::{self, BufRead};
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

use jiff::tz::TimeZone;
use serde_json::{Value, json};

use rates::{Rates, Web};
use settings::{PACKAGE, Settings};
use store::Book;
use wire::Wire;

/// 账本的文件名（工作目录里）。
const BOOK: &str = "sponsors.json";

/// 工具名。
const TOOL: &str = "sponsor";

/// 跑起来要的。
struct State {
    wire: Arc<Wire>,
    book: Mutex<Book>,
    settings: Mutex<Settings>,
    rates: Box<dyn Rates>,
}

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("serve") {
        return serve();
    }
    eprintln!("usage: miyu-im-sponsor serve");
    ExitCode::from(2)
}

/// 跑起来：读账本、握手、登记工具，然后照核心发来的办，标准输入读到头就退出。
fn serve() -> ExitCode {
    let book = match Book::open(Path::new(BOOK)) {
        Ok(book) => book,
        Err(error) => {
            // 退出码 1：账本读不进来，重启也没用，核心不重试；不覆盖它。
            eprintln!("cannot open {BOOK}: {error}");
            return ExitCode::from(1);
        }
    };
    let wire = Arc::new(Wire::new(Box::new(io::stdout())));
    let state = Arc::new(State {
        wire: Arc::clone(&wire),
        book: Mutex::new(book),
        settings: Mutex::new(Settings::default()),
        rates: Box::new(Web::new()),
    });
    let reading = {
        let state = Arc::clone(&state);
        std::thread::spawn(move || read(&state))
    };
    let hello = json!({"protocol": [1, 1], "head": {"kind": PACKAGE, "version": env!("CARGO_PKG_VERSION")}});
    match wire.call("hello", &hello) {
        Ok(answer) => {
            if let Some(config) = answer["config"].as_object() {
                state
                    .settings
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .apply(config);
            }
        }
        Err(error) => {
            eprintln!("hello refused: {error}");
            return ExitCode::from(2);
        }
    }
    if let Err(error) = wire.call("provide", &provided()) {
        eprintln!("provide refused: {error}");
        return ExitCode::from(2);
    }
    // 后台页调的方法（施工 S-1 下）。
    if let Err(error) = wire.call("package.methods", &methods::register()) {
        eprintln!("package.methods refused: {error}");
        return ExitCode::from(2);
    }
    drop(reading.join());
    ExitCode::SUCCESS
}

/// `provide` 的参数：一件 `sponsor`，只给场所会话（设计 03 第二节第 2 条）。
fn provided() -> Value {
    let spec: Value = serde_json::from_str(texts::TOOL).unwrap_or_default();
    json!({"tools": [{
        "name": TOOL,
        "description": spec["description"],
        "input_schema": spec["parameters"],
        "access": "write",
        "venues": ["private", "group"],
        "display_name": {"zh": "赞助记录", "en": "Sponsorships", "ja": "スポンサー記録"},
        "subject": "action",
    }]})
}

/// 读核心发来的：回应交给等它的，请求各开一个线程办（办的时候要回头问 im），通知当场办。
fn read(state: &Arc<State>) {
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match (
            message.get("method").and_then(Value::as_str),
            message.get("id"),
        ) {
            (Some(method), Some(id)) => {
                let state = Arc::clone(state);
                let (method, id, params) =
                    (method.to_string(), id.clone(), message["params"].clone());
                std::thread::spawn(move || {
                    let answer = request(&state, &method, &params);
                    state.wire.answer(&id, answer);
                });
            }
            (Some("extension.config"), None) => {
                if let Some(keys) = message["params"]["keys"].as_object() {
                    state
                        .settings
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .apply(keys);
                }
            }
            (None, Some(id)) => {
                let answer = match message.get("error") {
                    Some(error) => Err(error.clone()),
                    None => Ok(message["result"].clone()),
                };
                state.wire.deliver(id.as_str().unwrap_or_default(), answer);
            }
            (Some(_) | None, None) => {}
        }
    }
}

/// 办核心发来的一条请求。
fn request(state: &State, method: &str, params: &Value) -> wire::Answer {
    match method {
        "tool.call" if params["tool"] == TOOL => Ok(called(state, params)),
        "method.call" => {
            let now = jiff::Timestamp::now().as_millisecond();
            methods::call(state, params, (now, &TimeZone::system()))
        }
        _ => Err(wire::unknown_method()),
    }
}

/// 一次 `tool.call`：先问 im 是哪个场所、叫她的那一条（问不到的照 `null`），再照工具办。
fn called(state: &State, params: &Value) -> Value {
    let asked =
        json!({"package": "im", "method": "chat.venue", "params": {"session": params["session"]}});
    let venue = state
        .wire
        .call("package.call", &asked)
        .unwrap_or(Value::Null);
    let context = tool::Context {
        book: &state.book,
        settings: state
            .settings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
        rates: state.rates.as_ref(),
        now: jiff::Timestamp::now().as_millisecond(),
        zone: TimeZone::system(),
    };
    tool::call(params, &venue, &context)
}
