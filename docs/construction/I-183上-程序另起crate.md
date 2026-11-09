## 施工单 I-183 上：程序 miyu-im 另起一个 crate

状态：做完了（2026-11-09；接 Telegram 真驱动的第一步，设计 07 第 4.3 节、第八节 8.3）。

### 为什么

程序 `miyu-im` 一直在 OneBot 驱动的 crate 里（`main.rs` 只有一行交驱动，施工 I-84）。Telegram 驱动另成 `miyu-im-telegram` 以后，程序要同时挂两个驱动：留在 OneBot 的 crate 里，OneBot 就得依赖 Telegram 的驱动，驱动之间缠上了。I-84 当时定「另起程序 crate 随第二个真驱动」，现在到了。

### 怎么改

1. 新 crate `crates/miyu-im-app`：程序 `miyu-im`（`src/main.rs`，照旧一行交 OneBot 一个驱动）、测试用的程序 `miyu-im-test-hook`。
2. 起真程序的集成测试（`tests/`，`env!("CARGO_BIN_EXE_miyu-im")` 只认同一个包的程序）整个挪过去，内容不改；它们原来借 OneBot crate 的正式依赖（群聊内核、`futures-util`、`tracing`、`jiff`）补进新 crate 的开发依赖，版本照旧。
3. `miyu-im-onebot` 只剩驱动；开发依赖只留单元测试用的两个（`toml`、停住的钟要的 `tokio` `test-util`）。
4. 依赖方向的测试多一条：程序只依赖宿主和驱动。发版照 `-p miyu-im-app` 编程序，打出来的包不变。
5. 蓝图 `onebot.md`、`chat.md` 里的路径、设计 07 第 8.3 节的施工表照改（I-183 拆成上、中上、中下、下）。

### 验收

- 纯搬家：行为、程序名、包不变；本机门禁七项全绿（集成测试照旧四百多个全过）、三台 CI 全绿。
