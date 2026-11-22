<p align="center">
  <a href="README.md">English</a> | 简体中文
</p>

# 通讯平台（im）

> <sub>本文档由 Anthropic 的 Claude Fable 5.2 生成。</sub>

这是 Miyu 的一个软件包，它可以把 AI 接入通讯平台，让你在私聊和群聊里都能和 AI 对话。目前支持 QQ（通过 NapCat 这类 OneBot v11 实现的反向 WebSocket 连接）和 Telegram（通过官方的机器人接口）。这个软件包不随 Miyu 的发行包一起安装，需要使用的话要自己安装。

核心的代码在 [`miyu-agent/miyu`](https://github.com/miyu-agent/miyu) 仓库里。这个仓库通过 `Cargo.toml` 里的 git 依赖，固定使用核心的某一个提交。

## 里面有什么

下表列出了这个仓库里的各个目录和它们的用途。

| 目录 | 是什么 |
|---|---|
| `crates/miyu-im/` | 通讯平台的宿主：负责场所会话、群聊、插件挂接点和给模型使用的工具，和具体的平台无关，也不依赖驱动 |
| `crates/miyu-im-app/` | 程序 `miyu-im`：它把各个平台的驱动交给宿主。核心启动这个程序，并通过标准输入输出和它交换协议消息。启动真实程序的集成测试也在这里 |
| `crates/miyu-im-onebot/` | OneBot v11 的驱动：NapCat 反向连接到这个驱动，驱动把消息送进场所会话，再把 AI 的回复发回 QQ |
| `crates/miyu-im-telegram/` | Telegram 的驱动：它使用官方的机器人接口，通过长轮询接收消息 |
| `crates/miyu-im-sponsor/` | 赞助记录插件，程序名是 `miyu-im-sponsor` |
| `crates/miyu-chat/` | 群聊内核：它是通讯平台里和具体平台无关的那一层（场所规则、进站链、主动回复判断、判官、出站链），只包含纯逻辑；其他平台需要使用时，可以通过 git 依赖引用它 |
| `im/` | 包文件夹（包编号 `im`）：里面有清单 `package.toml`、通讯平台自己的页面 `site/`，以及 `data/`（OneBot 桥自己读取的出厂数据和给模型看的文字） |
| `im-sponsor/` | 赞助记录插件的包文件夹（包编号 `im-sponsor`） |
| `docs/blueprint/` | 设计图纸：`onebot.md`（OneBot 桥）、`chat.md`（群聊内核） |
| `docs/ledger.md` | 登记簿：登记了 `im/data/` 里每一份给模型看的文字 |
| `xtask/` | 门禁：`cargo xtask check` |

## 安装

这一节介绍安装这个软件包的方法。先安装 Miyu，然后运行：

```sh
miyu pkg install im
```

赞助记录插件是另一个软件包 `im-sponsor`，安装方法相同。如何接入 QQ 和 Telegram、设置哪些人能和 AI 对话、设置群里的规则，可以看 [通讯平台](https://github.com/miyu-agent/miyu/blob/main/docs/zh/guide/heads/im.md)。

## 从源码编译

这一节介绍如何自己编译这个软件包。编译需要 Rust（版本由 `rust-toolchain.toml` 指定，`rustup` 会自动安装）和 git（cargo 通过 git 依赖获取核心的代码）。运行：

```sh
cargo build --release -p miyu-im-app --bin miyu-im
```

把编译出的程序放进包文件夹 `im/` 下的 `bin/` 目录（在 Windows 上文件名带 `.exe`；`im/bin/` 不纳入 git 管理），然后安装这个文件夹：

```sh
mkdir -p im/bin
cp target/release/miyu-im im/bin/
miyu pkg install ./im
```

软件包的编号由文件夹的名字决定，所以一定要安装名为 `im` 的文件夹（这个软件包的配置项的键是 `im.*`，赞助记录插件的是 `im-sponsor.*`）。如果程序不在 `bin/` 目录里，核心会把这个软件包当作没有安装。

## 门禁

门禁是一组提交代码之前必须通过的检查。运行：

```sh
cargo xtask check
```

这条命令会依次运行格式检查、`clippy`（`-D warnings`）、文档检查、登记簿对账、纯逻辑检查（群聊内核不能接触 I/O 和时钟）、行数检查（每个 `.rs` 文件不超过 500 行）、许可证检查和测试，最后输出一张结果表。三个平台的 CI 运行的都是同一条命令。

## 许可证

这个仓库使用 AGPL-3.0-or-later 许可证，见 `LICENSE`。通讯平台的网页后台在侧边栏底部放了源码链接，因此通过网络使用它的人也能拿到源码。

不是这个仓库自己编写的内容，仍然使用它们原来的许可证，AGPL 只适用于这个仓库自己编写的部分：各个依赖使用各自的 `license`（门禁里的“许可证”一项会检查它们能否合在一起发布）；收录的第三方资源使用各自的许可证，并随附许可证原文。

| 第三方资源 | 许可证 | 原文 |
|---|---|---|
| Lucide 图标（后台页的 `im/site/shell/lucide-all.json`、`lucide.js`） | ISC | `im/site/shell/LICENSE-lucide.txt` |

编译进程序的第三方包和上面这些资源，它们的版权声明和许可证原文都在 `THIRD-PARTY-LICENSES.txt` 里（这个文件由 `cargo xtask notices` 生成，门禁会检查它是不是最新的）。发行包里的每个包文件夹都带有这个文件和 `LICENSE`。
