## 施工单 I-186：核心没在跑时 `miyu im start` 拉得起核心

状态：做完了（2026-11-09；core1 做验收便携版时撞见）。

### 为什么

核心没在跑时 `miyu im start` 报「无法连接核心：拉不起核心：No such file or directory」。im 拉起核心只找自己真实位置旁边的 `miyu`（`program.rs` 的 `beside`），可 im 装在数据根的 `opt/miyu-im/bin/` 下，旁边从来没有主程序，正式安装也一样。平时核心常驻，撞不上。

### 怎么改

照核心转交子命令、拉起扩展时交的 `MIYU_PROGRAM`（主程序的真实位置，核心 G-15）找：先认它；没交、那个文件不在了的退回自己旁边的；旁边也没有的照名字从 `PATH` 找（`program.rs` 的 `main_program`、`locate`）。拉起核心只有 `core()` 一处，`start`、`stop`、`web` 都经它。

### 验收

- 单元测试 `program::tests`：交来的在就照它；不在了照旁边；旁边也没有照名字。
- 集成测试 `main_program`（Unix）：没有核心在跑的数据根，`MIYU_PROGRAM` 指一个记参数的脚本，`start` 照它跑了 `core`；退回只找旁边、PATH 的写法时报红（没照它拉起）。
- 本机门禁、三台 CI 全绿。

### 坑

- 退回旧写法验红时，PATH 里找到了这台机器上装的真 `miyu`，在测试的临时数据根里起了一下核心（起不来、自己退了）。测试照设计设了 `MIYU_PROGRAM`，修好的写法不会走到 PATH。
