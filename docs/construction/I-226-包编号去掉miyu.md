## 施工单 I-226：包编号去掉 `miyu-`

状态：做完了（2026-11-12）。

### 起因

项目主人 11-12 定新规矩（core1 转来）：包编号不带 `miyu-`、不带接入方式；仓库名、程序名照旧带 `miyu-`。没发版：不留旧编号别名。

### 定了的（项目主人 11-12 当面答的四条）

1. im 会话改 im、赞助记录，外加渲染包、表情包、表情包内容包（各自仓库另起）；迁移包归 core1。
2. 机器上的旧数据（核心照包编号分的状态目录）部署时挪一次，不写兼容代码。
3. 赞助记录的键跟着成 `im-sponsor.*`（不写 `prefix` 保旧的）。
4. im 内置插件的编号一起去掉 `miyu-`。

### 改了什么

1. 包文件夹改名（包编号就是文件夹的名字，核心设计 48）：`miyu-im/` → `im/`、`miyu-im-sponsor/` → `im-sponsor/`；`include_str!`、测试、`release.yml`（`package`、`folder`、`dist/`）、`.gitignore`、第三方许可证清单里的路径跟着改。
2. 包编号的常量：`miyu_im::PACKAGE` 是 `im`（握手的 `head.kind`、软件源的 `contents` 照它），赞助记录的 `PACKAGE` 是 `im-sponsor`（配置键、事件 `ext.im-sponsor.*`、插件编号照它）；赞助记录清单 `[content.miyu-im]` → `[content.im]`。
3. im 的清单去掉 `prefix = "im"`：编号就是 `im`，不写也一样。
4. 内置插件编号：`miyu-im-identity`、`miyu-im-qq-admin`、`miyu-im-qq-lookup`、`miyu-im-glance`、`miyu-im-memes` → `im-*`（卡片上存着的插件开关照新编号认）。
5. 给人看的字：「核心里没有 im 软件包」；「找不到 miyu-im 程序」照旧（说的是程序）。
6. 引用别的包的地方：渲染包 `render`、表情包 `memes`（测试、注释、作者说明）。
7. 不动：仓库名、crate 名、程序名 `miyu-im`、`miyu-im-sponsor`、运行日志 `miyu-im.log`、浏览器里记的几个键（`miyu-im-log-mode` 这种不是包编号）、已经写下的施工单和设计。

### 部署时挪旧数据

核心按包编号给状态目录（`<数据根>` 下的状态、配置那几层），新的 im 照 `im`、`im-sponsor` 找。换包前停核心，把 `miyu-im`、`miyu-im-sponsor` 两个状态目录改名成 `im`、`im-sponsor`，再装新包。命令写进主人跑的脚本，不进代码。

### 怎么验

- 整套测试（`cargo test --workspace`）全过；本机门禁七项。
