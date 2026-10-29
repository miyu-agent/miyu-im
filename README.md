# 接入通讯平台

Miyu 的一个软件包：把 AI 接入通讯平台，私聊、群里都能和 AI 说话。现在接的是 QQ（经 NapCat 这类 OneBot v11 实现的反向 WebSocket）。它不随 Miyu 的发布包一起装，要用的自己装。

核心在 [`miyu-agent/miyu`](https://github.com/miyu-agent/miyu)；这个仓库照 `Cargo.toml` 里的 git 依赖钉住核心的一个提交（核心仓库的设计 `docs/designs/32-仓库拆分.md` 第九节）。

## 里面有什么

| 目录 | 是什么 |
|---|---|
| `crates/miyu-im/` | 接入通讯平台的宿主：场所会话、群聊、插件挂接点、给模型的工具，和平台无关；不依赖驱动 |
| `crates/miyu-im-onebot/` | OneBot v11 的驱动和程序 `miyu-im`：核心拉起它、经标准输入输出说协议；NapCat 反连进来，消息送进场所会话，她的回复发回 QQ |
| `crates/miyu-chat/` | 群聊内核：通讯平台里和平台无关的那一层（场所规则、进站链、主动回复判断、判官、出站链），纯逻辑；别的平台要用照 git 依赖它 |
| `miyu-im/` | 包文件夹（包编号 `miyu-im`）：清单 `package.toml`、im 自己的页面 `site/`、网页软件框里的后台页 `page/`、桥自己读的出厂数据和给模型看的字 `data/` |
| `docs/blueprint/` | 图纸：`onebot.md`（桥）、`chat.md`（群聊内核） |
| `docs/ledger.md` | 登记簿：`miyu-im/data/` 里给模型看的每一份字 |
| `xtask/` | 门禁：`cargo xtask check` |

## 装

1. 先装好 Miyu。
2. 从 [Releases](https://github.com/miyu-agent/miyu-im/releases) 下载自己平台的包：`miyu-im-<版本>-<平台>.tar.gz`（Windows 是 `.zip`），解开得到一个 `miyu-im/` 文件夹。
3. 装上：

   ```sh
   miyu pkg install ./miyu-im
   ```

   它要的能力（代平台上的人发话、联网、读写会话……）照自己装的包走人批，装的时候核心会问。
4. 运行 `miyu im web` 打开接入通讯平台的页面，点「新建连接」照着走：在 NapCat 那边新建反向 WebSocket，填页面上给的地址，再把 NapCat 生成的 Token 粘贴回页面。

## 从源码编

要 Rust（版本照 `rust-toolchain.toml`，`rustup` 会自己装）和 git（cargo 照 git 依赖取核心）。

```sh
cargo build --release -p miyu-im-onebot
```

程序放进包文件夹 `miyu-im/` 的 `bin/` 里（Windows 上带 `.exe`；`miyu-im/bin/` 不进 git），再装这个文件夹：

```sh
mkdir -p miyu-im/bin
cp target/release/miyu-im miyu-im/bin/
miyu pkg install ./miyu-im
```

包的编号照文件夹的名字定：一定要装名叫 `miyu-im` 的文件夹（清单写了 `prefix = "im"`，配置项的键照旧是 `im.*`）。程序不在 `bin/` 里的，核心当这个包没装。

## 门禁

```sh
cargo xtask check
```

依次跑格式、clippy（`-D warnings`）、文档、登记簿对账、纯逻辑（群聊内核不碰 I/O、时钟）、行数（每个 `.rs` 不超过 500 行）、许可证、测试，最后打一张结果表。三个平台的 CI 跑的是同一条命令。

## 许可证

AGPL-3.0-or-later，见 `LICENSE`（2026-10-26 项目主人定许可证分层：核心和第一方程序用 AGPL-3.0-or-later）。im 的网页后台在侧边栏底下放了源码链接：别人经网络用到它，也拿得到源码。

不是这个仓库自己写的东西照它们原来的许可证，AGPL 只管这里自己写的部分：依赖照各自的 `license`（门禁的「许可证」一项查能不能合在一起发）；收的第三方资源照各自的许可证，原文随附。
