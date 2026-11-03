## 施工单 I-128：records 命令的帮助和用法错

状态：做完了（2026-11-04；I-127 合了以后打部署包时发现）。

### 目的

I-127 加了 `miyu im records import <文件>`，帮助页 `help/records` 三种语言也写了。可是帮助认的命令名单是写死的（`texts/cli.rs`），没加上 `records`，所以出了两个毛病：
- `miyu im records --help` 说「未知命令 'records'」；
- 少写文件的 `miyu im records import` 也说未知命令，不说用法不对。

### 怎么走

1. `texts/cli.rs` 的名单加上 `records`。
2. 测试：
   - `stdio.rs` 的帮助、用法错各加一条：`records import --help` 印那一页，`records import` 少写文件是用法不对。
   - `cli_texts.rs` 的帮助页清单、命令行文案的开头各加上 `records`。

### 验收

- 名单不加 `records` 时 `stdio.rs` 那一条红，加上绿；`cli_texts` 照规矩查过 `records/` 的字。
- 本机门禁、三台 CI 全绿。
