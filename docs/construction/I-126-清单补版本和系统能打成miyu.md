## 施工单 I-126：清单补版本和系统，能打成 .miyu

状态：做完了（2026-11-03；主人定 FHS 做完以后 im 和赞助记录进官方软件源，core1 派的。真发版要主人当面点头，core1 问）。

### 目的

官方软件源收的是各仓库最新 Release 里的 `.miyu`。im 和赞助记录的两份清单原来都没写 `version`，核心的 `pkg pack` 打不成；原来的发包流程打的是 `tar.gz`、`zip`，软件源不收。

### 怎么走

1. 两份清单（`miyu-im/package.toml`、`miyu-im-sponsor/package.toml`）补 `version = "0.1.0"`（同表情包、embed 的第一版），两个包同一个版本，打标签 `v<版本>` 发版。
2. 补 `targets`：包里带编好的程序，一种系统一个包文件。写三台 CI 测过的 `linux-x86_64`、`macos-aarch64`、`windows-x86_64`；`linux-aarch64` 等 CI 测上了再加。
3. `.github/workflows/release.yml` 换成官方软件源的打包模板（`miyu-agent/packages` 的 `pack.yml`，照 embed 的写法）：两个包各一个任务，一种系统一个，打包前编程序、拼包文件夹、查结构；推标签时 `.miyu` 传进 Release，手动跑的放进产物。
4. README「装」一节改成 `miyu pkg install miyu-im`，或者下载 `.miyu` 装。

### 不在这一步

- 打标签、真发版、把仓库加进软件源的 `official.toml`：主人点头以后，core1 安排。

### 验收

- 本机照发包流程拼好两个包文件夹，用核心 FH-3 那一版的 `pkg pack --target linux-x86_64` 打得出 `miyu-im-0.1.0-linux-x86_64.miyu`、`miyu-im-sponsor-0.1.0-linux-x86_64.miyu`。
- 在分支上手动跑一次发包流程，三种系统、两个包都打得出。
- 本机门禁、三台 CI 全绿。
