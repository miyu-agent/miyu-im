## 施工单 I-86：包改名 miyu-im

状态：做完了（2026-10-30；核心设计 48「包的命名」，核心 PN-1、PN-4）。

### 目的

项目主人 10-29 定官方包一律 `miyu-` 打头，im 改叫 `miyu-im`；配置键、会话事件保持短名。核心 PN-1 让清单能写 `[package] prefix`：配置键是 `<前缀>.<名字>`，扩展记的事件是 `ext.<前缀>.*`。im 原来一个常量 `PACKAGE = "im"` 什么都管，要拆开。

### 图纸改哪几节

- 设计 07：第 7.4 节改成照核心设计 48 第五节用仓库外的工具搬（产品里不做迁移）；第 7.5 节加 I-86 这一行，I-81 不再等核心的改编号方法。
- 蓝图 `onebot.md`、`chat.md`、`extending.md`、README 里的状态目录、模块目录、日志文件、`package.call` 的包编号跟着改。

### 怎么走

1. 两个常量（`lib.rs`）：
   - `PACKAGE = "miyu-im"`：`extension.*`、`package.call`、连核心报的头、`state/packages/<编号>/`、`modules/<编号>/`、运行日志 `state/logs/miyu-im.log`（核心接的标准错误本来就照编号写 `miyu-im.stderr`）、判官旁路请求的全编号（核心照编号拼成 `miyu-im/judge`）、插件包带给它的内容（`[content.miyu-im]`）。
   - `PREFIX = "im"`：配置键（`im.qq.token`、`im.whitelist` 这些）、事件（`ext.im.*`）、页面存的密钥名（`im-<平台>-<名字>`，线上的密钥不用搬）。
2. 仓库里的包文件夹改名：`im/` → `miyu-im/`，`sponsor/` → `miyu-im-sponsor/`（施工 O-44：核心照文件夹的名字定包编号，从源码
   `miyu pkg install ./<文件夹>` 装出来的编号要对；赞助记录原来的 `sponsor/` 从源码装出来编号是 `sponsor`，一起改对）。代码里的
   `include_str!`、测试、门禁的登记簿（`docs/ledger.md`）、README 的路径跟着改。清单 `miyu-im/package.toml` 的 `[package]` 加
   `prefix = "im"`。
3. 赞助记录：编号照旧 `miyu-im-sponsor`；清单 `[content.im]` 改 `[content.miyu-im]`（目录照旧 `im/`）；问场所照 `miyu-im`。
4. 发包：拼成 `miyu-im/`、`miyu-im-sponsor/` 文件夹（包文件夹的名字就是包编号）；装法 `miyu pkg install ./miyu-im`。
5. 核心钉到 dcc464a36（含 PN-1、PN-4）。
6. 测试：拼资源目录、状态目录、模块目录、日志文件、`package.call`、扩展列表、旁路请求编号照新编号。

### 不做什么

- 旧数据不做兼容（核心设计 48 第一节第 6 条）：项目主人部署里的 `state/packages/im/`、`modules/im/` 由仓库外的脚本搬（设计 07 第 7.4 节），判官旁路请求的模型档位配置（`im/judge` 那一项）由核心的工具搬。
- 表情包（`miyu-memes`）的 `[content.im]` 和它调 im 的两处，在 im 合进 main 以后另改那个仓库。

### 验收

- 现有测试全过（照新编号改了夹具）；`settings::the_package_folder_is_named_after_the_package` 守着仓库的包文件夹叫 `miyu-im`。
- 本机门禁、三台 CI 全绿。
