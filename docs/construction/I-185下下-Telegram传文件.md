## 施工单 I-185 下下：Telegram 传文件（`tg_send_file`）

状态：做完了（2026-11-10；项目主人 11-09 定「发文件要做」，同 QQ 能发文件；路径照 LV-4 的规矩：主人本人叫的、完全放开的会话不限工作区）。

### 为什么

Telegram 里 AI 给人看的图、文件（`show`）I-185 中起发得出去，但不能像 QQ 的 `qq_send_file` 那样把工作区里的一份文件直接传进这个聊天。

### 怎么改

1. QQ 的 `qq_send_file` 原样挪进宿主（`toolset::common::send_file`，单元测试跟着挪），两个驱动同用：
   - 判路径：只认工作区里的普通文件，放开的会话不限；
   - 钉进 `outbox/`，链不了的拷一份；
   - 清老的；
   - 「大小照人的单位写」也挪过去（QQ 的群文件列表照旧用它）。
   QQ 的行为不变。
2. Telegram 驱动交 `tg_send_file`（私聊、群都给，`bridge.json` 的 `upload_seconds` 清 `outbox/`）。连接答「读得到本机的文件」（驱动和 im 在同一个进程），钉好的那一份照 I-185 中的 `sendDocument` 传（最大 50 MB），传完 im 删掉钉的那一份。
3. 数据：
   - 一份说明（和 QQ 的一字不差）。
   - 四句结果：`uploading`、`no-file` 和 QQ 的字节一样；`not-local`、`unsendable` 平台写 Telegram。
   - 显示名、网页活动记录的名字；功能表「聊天动作」多一件。
   - 登记簿五行。
   - Telegram 的工具面十件 851 个 token、1902 字节，在预算（4312）以内。

### 验收

- 集成测试 `telegram_send_file`（真核心、本机假 Bot API）：
  - 不是主人的私聊里她调两次：工作区里的照 `sendDocument` 传、文件名和内容对，传完 `outbox/` 空了；外面的答不是工作区里的文件，没去传。
  - 把连接「读得到本机的文件」改回假，这个测试报红。
- QQ 的 `send_file` 两条（群友、主人）照旧过；本机门禁、三台 CI 全绿。
