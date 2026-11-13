## 施工单 AUD-1：没人用的依赖和装法

状态：做完了（2026-11-13），三台 CI 绿了由 im 的会话过目再合。

### 起因

项目主人 2026-11-13 让审计会话把几个仓库过一遍，找坏的、对不上的、白费的。这个仓库查出三样，都不改行为：

1. `crates/miyu-im/Cargo.toml` 的开发依赖里有十个核心的 crate 没人用：`miyu-ipc`、`miyu-store`、`miyu-log`、`miyu-kernel`、`miyu-tool`、`miyu-core`、`miyu-basesystem`、`miyu-endpoint`、`miyu-session`、`miyu-models`。起真核心跑的测试 I-183 上搬去了 `crates/miyu-im-app/tests/`，这个 crate 只剩单元测试，`src/` 里一处都没提到它们。留着的坏处：单编 `cargo test -p miyu-im` 要白编一遍核心。另有一行 `tokio-tungstenite` 和正式依赖一字不差，重复了。
2. `crates/miyu-im-onebot/Cargo.toml` 的 `jiff` 没人用：查聊天记录的时间段那段代码在宿主的 `toolset/common/history.rs`。
3. 装法写的还是改名以前的：README「从源码编」最后一行 `miyu pkg install ./miyu-im`（下一行自己就写着「一定要装名叫 `im` 的文件夹」），Release 里的包文件名写成 `miyu-im-<版本>-<系统>.miyu`（实际是 `im-…`）；AGENTS.md 同一句，另有三处测试路径还写 `crates/miyu-im-onebot/tests/`（现在在 `crates/miyu-im-app/tests/`）。

### 改了什么

1. `crates/miyu-im/Cargo.toml`：`[dev-dependencies]` 只留用到的四个（`miyu-client`、`miyu-config`、`miyu-http`、`tokio`），注释照现在的写。
2. `crates/miyu-im-onebot/Cargo.toml`：去掉 `jiff`。
3. `Cargo.lock`：两个包的依赖表跟着少这几行，别的不动（没有哪个包从锁文件里消失：`miyu-im-app` 的测试照旧用它们）。
4. README、AGENTS.md：装法、包文件名、测试路径照现在的写。

### 不做什么

- 不动 `tokio` 那一行的特性、`miyu-client` 的 `testkit`：没法只靠查字证明没人用。
- 图纸（`docs/blueprint/`）里别的过期路径不在这一步改，清单交给 im 的会话。

### 怎么验

- `cargo tree -p miyu-im -e normal,dev`：改前 266 个 crate，改后 222 个，少的 44 个里有 `miyu-core`、`miyu-endpoint`、`miyu-session`、`miyu-basesystem`、`miyu-knowledge`、`miyu-mcp`、`zstd-sys`。
- 单编 `cargo test -p miyu-im --lib --no-run`、`-p miyu-im-onebot --lib --no-run` 编得过（特性不靠工作区里别的包凑）。
- 三台 CI 的门禁照旧全过：`dependencies.rs` 只看正式依赖，开发依赖不算。
