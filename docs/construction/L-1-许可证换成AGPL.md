## 施工单 L-1：许可证换成 AGPL-3.0-or-later

状态：做完了。

### 目的

项目主人 2026-10-26 定许可证分层（核心主会话转来）：核心和第一方程序用 AGPL-3.0-or-later，开发包、作者模板、协议参考用 Apache-2.0。im 是第一方程序，从 GPL-3.0-or-later 换成 AGPL-3.0-or-later。

### 做什么

- `LICENSE` 换成 FSF 的 AGPL-3.0 原文（https://www.gnu.org/licenses/agpl-3.0.txt，34523 字节，sha256 `0d96a4ff` 开头，下载以后核对过）。
- 工作区 `Cargo.toml` 的 `license`（各 crate 照 `license.workspace`），README 的许可证一节，`AGENTS.md` 那一条。
- 许可证门禁（`xtask/src/licenses.rs`）：仓库自己是 AGPL-3.0-or-later，能合在一起发的里面多它；照 git 钉住的核心的 crate 在核心换许可证以前还是 GPL-3.0-or-later，照旧认（两份第 13 条互相允许合在一起）。
- im 的网页后台在网络上听着（经网络用到它的人也要拿得到源码，AGPL 第 13 条）：侧边栏底下多一个「源码」链接，指向仓库，三种语言。

### 测试

- `xtask` 的许可证测试照新说法改；门禁「许可证」一项全过。
- `tests/page.rs`：脚本里新用到的字三种语言都写了。

### 验收

本机门禁 `cargo xtask check` 全过，CI 三台全绿。打开 im 的网页后台，侧边栏底下有「源码」，点开是仓库。
