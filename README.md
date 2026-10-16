# 接入QQ

Miyu 的一个软件包：经 NapCat（OneBot v11 的反向 WebSocket）把 QQ 接到 Miyu 上，私聊、群里都能和 AI 说话。它不随 Miyu 的发布包一起装，要用的自己装。

核心在 [`miyu-agent/miyu`](https://github.com/miyu-agent/miyu)；这个仓库照 `Cargo.toml` 里的 git 依赖钉住核心的一个提交（核心仓库的设计 `docs/designs/32-仓库拆分.md` 第九节）。

## 里面有什么

| 目录 | 是什么 |
|---|---|
| `crates/miyu-onebot/` | 桥：程序 `miyu-onebot`，核心拉起它、经标准输入输出说协议；NapCat 反连进来，消息送进场所会话，她的回复发回 QQ |
| `crates/miyu-chat/` | 群聊内核：通讯平台里和平台无关的那一层（场所规则、进站链、主动回复判断、判官、出站链），纯逻辑；别的平台要用照 git 依赖它 |
| `package/` | 包文件夹：清单 `package.toml`、后台页 `page/`、桥自己读的出厂数据和给模型看的字 `data/` |
| `docs/blueprint/` | 图纸：`onebot.md`（桥）、`chat.md`（群聊内核） |
| `docs/ledger.md` | 登记簿：`package/data/` 里给模型看的每一份字 |
| `xtask/` | 门禁：`cargo xtask check` |

## 装

1. 先装好 Miyu。
2. 从 [Releases](https://github.com/miyu-agent/miyu-onebot/releases) 下载自己平台的包：`miyu-onebot-<版本>-<平台>.tar.gz`（Windows 是 `.zip`），解开得到一个 `onebot/` 文件夹。
3. 装上：

   ```sh
   miyu pkg install ./onebot
   ```

   它要的能力（代平台上的人发话、联网、读写会话……）照自己装的包走人批，装的时候核心会问。
4. 在 Miyu 的接入页里打开「接入QQ」，设好端口和令牌；NapCat 那边的反向 WebSocket 填 `ws://127.0.0.1:<端口>/ws`、同一个令牌。

## 从源码编

要 Rust（版本照 `rust-toolchain.toml`，`rustup` 会自己装）和 git（cargo 照 git 依赖取核心）。

```sh
cargo build --release -p miyu-onebot
```

拼成能装的包文件夹（Windows 上程序带 `.exe`）：

```sh
mkdir -p onebot/bin
cp -R package/. onebot/
cp target/release/miyu-onebot onebot/bin/
miyu pkg install ./onebot
```

## 门禁

```sh
cargo xtask check
```

依次跑格式、clippy（`-D warnings`）、文档、登记簿对账、纯逻辑（群聊内核不碰 I/O、时钟）、行数（每个 `.rs` 不超过 500 行）、许可证、测试，最后打一张结果表。三个平台的 CI 跑的是同一条命令。

## 许可证

GPL-3.0-or-later，见 `LICENSE`。
