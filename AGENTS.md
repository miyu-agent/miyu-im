# 给在这个仓库里干活的 AI

你是顶级的 Rust 工程师，也是顶级的 AI loop、AI Agent、AI Harness 设计师。

## 这个仓库是什么

Miyu 的「接入QQ」软件包：桥 `crates/miyu-onebot`（程序 `miyu-onebot`）、群聊内核 `crates/miyu-chat`、包文件夹 `package/`。核心在 `miyu-agent/miyu`，这里照 `Cargo.toml` 里的 git 依赖钉住核心的一个提交（核心仓库的设计 `docs/designs/32-仓库拆分.md` 第九节）。它不进核心的发布包：单独发、用的人自己 `miyu pkg install ./onebot`。

## 先读什么

- `docs/blueprint/onebot.md`（桥）、`docs/blueprint/chat.md`（群聊内核）：现在长什么样，事无巨细，由哪些测试守着。改一个部件之前，先读它那一节。
- `docs/ledger.md`：登记簿，`package/data/` 里给模型看的每一份字。
- 核心仓库的设计文档（`docs/designs/`）：平台接入是设计 18，给模型看的字是设计 26，软件包是设计 31。O 线以前的施工单在核心仓库的 `docs/construction/` 里，是历史；以后的 O 线施工单写在这个仓库的 `docs/construction/`。

## 施工的规矩

1. 一步一张施工单，先商量、拍板，再动手。
2. 一步一个工作树：从 main 开分支 `step/<编号>-<名字>`，检出到 `.worktrees/<编号>-<名字>/`。本机门禁（`cargo xtask check`）全过再整理成正式提交，快进合进 main、推送；三平台 CI 在 main 上跑，红了当场修。碰路径、文件系统、子进程、Windows 的，先推 `step/` 分支过三平台 CI 再合。
3. 每一步做完，仓库是绿的：编译、测试、门禁全过。
4. 先写证明它对的测试。修 bug 先写能复现它的测试，修之前它必须是红的。
5. 照图纸施工：设计要改，先改 `docs/blueprint/` 那一页；合并时代码和图纸要对得上。
6. 写文档只写现在的样子和定了什么，后面注明日期、谁定的。

## 和核心的边界

- 桥、群聊内核只经门面 `miyu-client` 用核心的代码（设计 32 第九节第 1 条）；群聊内核另外直接用 `miyu-kernel`、`miyu-config` 的类型。缺的告诉核心的主会话加进门面，不直接依赖核心里别的 crate（`crates/miyu-onebot/tests/dependencies.rs` 守着）。测试可以用核心的别的 crate（开发依赖），钉同一个提交。
- 协议、事件的形状以核心仓库的 `docs/blueprint/protocol.md`、`docs/designs/samples/` 为准；改到协议的事先找核心。
- 升级核心：改根目录 `Cargo.toml` 的 `[workspace.dependencies]` 里钉住的提交（核心的几个 crate 都在那里，一个 `rev`），刷 `Cargo.lock`，过三台 CI。
- 桥的文件从包文件夹读（核心拉起它时交 `MIYU_PACKAGE_DIR`），不读核心资源目录里的 `software/onebot/`。

## 代码的规矩

- 先读懂，再动手；照周围的写法。
- 加第三方依赖之前先看许可证：仓库是 GPL-3.0-or-later，依赖要能和它合在一起发，门禁的「许可证」一项查。
- 一个模块一种职责；文件小，每个 `.rs` 不超过 500 行，测试也算。
- 不吞错误：要么处理，要么如实上报（不写 `let _ =`、`.ok()` 扔错误）。
- 无硬编码：数值、文字、名单是数据，住在 `package/data/` 里。
- 给模型看的字一律英文短句；给人看的字照三种语言（`package/data/human/`），不用「她」说 AI。
- 给模型看的字克制、有账：非必要不加，加之前先实测证明不加不行、量出多少 token；加了就登记在 `docs/ledger.md`，门禁照文件算指纹、和登记簿一一对上。
- 注释用中文，名字用英文；公开的东西都写 `///`，模块开头写 `//!`，会出错的写 `# Errors`，会 panic 的写 `# Panics`。

## 三个平台

- Linux、macOS（Apple 芯片）、Windows 上都要成立。平台差异关在边界上；路径、换行、编码都不写死。

## 测试

- 测试一律把 `MIYU_HOME` 指到临时目录，绝不碰真实数据。
- 要出厂资源的测试照钉住的那份核心检出（门面 `testkit` 的 `WORKSPACE_ROOT`）拼一个临时资源目录，把这个仓库的 `package/` 放成它的 `packages/onebot/`（`crates/miyu-onebot/tests/support/assembled.rs`）；直接拉桥的设 `MIYU_PACKAGE_DIR` 指 `package/`。
- 每个 crate 的集成测试并成一个程序 `tests/all.rs`，一个文件一个模块；装日志订阅者的自己一个程序，在 `Cargo.toml` 另列 `[[test]]`。门禁查漏掉的文件。
- 量尺类的测试标 `#[ignore]`；断言结果，不断言耗时。

## 协作

- 用中文回复。每次回复末尾报总体进度和把握，用百分比，附上依据。
- 先定位问题、找到根因，再提方案，标出推荐项，拍板以后再动手。不确定的点问项目主人，附推荐。
- 做完给出清楚的、能照着做的验收流程。项目主人验收通过才合进 main。
