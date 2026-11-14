# 接入通讯平台

Miyu 的一个软件包：把 AI 接入通讯平台，私聊、群里都能和 AI 说话。现在接的是 QQ（经 NapCat 这类 OneBot v11 实现的反向 WebSocket）和 Telegram（官方机器人接口）。它不随 Miyu 的发布包一起装，要用的自己装。

核心在 [`miyu-agent/miyu`](https://github.com/miyu-agent/miyu)；这个仓库照 `Cargo.toml` 里的 git 依赖钉住核心的一个提交（核心仓库的设计 `docs/designs/32-仓库拆分.md` 第九节）。

## 里面有什么

| 目录 | 是什么 |
|---|---|
| `crates/miyu-im/` | 接入通讯平台的宿主：场所会话、群聊、插件挂接点、给模型的工具，和平台无关；不依赖驱动 |
| `crates/miyu-im-app/` | 程序 `miyu-im`：把各个平台的驱动交给宿主；核心拉起它、经标准输入输出说协议。起真程序的集成测试也在这里 |
| `crates/miyu-im-onebot/` | OneBot v11 的驱动：NapCat 反连进来，消息送进场所会话，AI 的回复发回 QQ |
| `crates/miyu-im-telegram/` | Telegram 的驱动：官方机器人接口，长轮询收消息 |
| `crates/miyu-im-sponsor/` | 赞助记录插件，程序 `miyu-im-sponsor` |
| `crates/miyu-chat/` | 群聊内核：通讯平台里和平台无关的那一层（场所规则、进站链、主动回复判断、判官、出站链），纯逻辑；别的平台要用照 git 依赖它 |
| `im/` | 包文件夹（包编号 `im`）：清单 `package.toml`、im 自己的页面 `site/`、桥自己读的出厂数据和给模型看的字 `data/` |
| `im-sponsor/` | 赞助记录插件的包文件夹（包编号 `im-sponsor`） |
| `docs/blueprint/` | 图纸：`onebot.md`（桥）、`chat.md`（群聊内核） |
| `docs/ledger.md` | 登记簿：`im/data/` 里给模型看的每一份字 |
| `xtask/` | 门禁：`cargo xtask check` |

## 装

先装好 Miyu，再：

```sh
miyu pkg install im
```

赞助记录插件是另一个包 `im-sponsor`，装法一样。接 QQ、Telegram，管谁能叫 AI、群里的规矩，见 [接入通讯平台](https://github.com/miyu-agent/miyu/blob/main/docs/guide/heads/im.md)。

## 从源码编

要 Rust（版本照 `rust-toolchain.toml`，`rustup` 会自己装）和 git（cargo 照 git 依赖取核心）。

```sh
cargo build --release -p miyu-im-app --bin miyu-im
```

程序放进包文件夹 `im/` 的 `bin/` 里（Windows 上带 `.exe`；`im/bin/` 不进 git），再装这个文件夹：

```sh
mkdir -p im/bin
cp target/release/miyu-im im/bin/
miyu pkg install ./im
```

包的编号照文件夹的名字定：一定要装名叫 `im` 的文件夹（配置项的键是 `im.*`，赞助记录的是 `im-sponsor.*`）。程序不在 `bin/` 里的，核心当这个包没装。

## 门禁

```sh
cargo xtask check
```

依次跑格式、clippy（`-D warnings`）、文档、登记簿对账、纯逻辑（群聊内核不碰 I/O、时钟）、行数（每个 `.rs` 不超过 500 行）、许可证、测试，最后打一张结果表。三个平台的 CI 跑的是同一条命令。

## 许可证

AGPL-3.0-or-later，见 `LICENSE`（2026-10-26 项目主人定许可证分层：核心和第一方程序用 AGPL-3.0-or-later）。im 的网页后台在侧边栏底下放了源码链接：别人经网络用到它，也拿得到源码。

不是这个仓库自己写的东西照它们原来的许可证，AGPL 只管这里自己写的部分：依赖照各自的 `license`（门禁的「许可证」一项查能不能合在一起发）；收的第三方资源照各自的许可证，原文随附。

| 第三方资源 | 许可证 | 原文 |
|---|---|---|
| Lucide 图标（后台页 `im/site/shell/lucide-all.json`、`lucide.js`） | ISC | `im/site/shell/LICENSE-lucide.txt` |

编进程序的第三方包和上面这些资源，版权和许可证原文都在 `THIRD-PARTY-LICENSES.txt`（`cargo xtask notices` 生成，门禁查它是不是最新的），发行包里每个包文件夹都带它和 `LICENSE`。
