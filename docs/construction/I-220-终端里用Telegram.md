## 施工单 I-220：终端里用 Telegram（设计 13 第三步）

状态：做完了（2026-11-11）。

### 改了什么

1. Telegram 的四件给终端（`venues` 加 `local`）：`tg_chat_history`（多 `group`、`private`，不写场所是查全部，`Reach::Either`）、`tg_send_file`（多 `group`、`private`）、`tg_token_usage`、`tg_activity_ranking`（各多 `group`）。路同 QQ（I-218）。
2. Telegram 的答的话多那六句（同 OneBot 的，一字不差，登记簿照抄）。
3. 场所参数认负的号（`route/elsewhere.rs` 的 `aimed`）：Telegram 的群号是负的（`-100…`），原来只认正的会把它当写错。
4. Telegram 那一套的工具说明合计在同一个预算（4840）以内，不加；四件的登记簿照量的改。

### 怎么验

- 单元测试：`aimed` 认负的群号。
- 集成测试 `telegram_tools::from_the_terminal_she_searches_a_telegram_group`：假 Bot API，群里 Alice 说了「今晚吃火锅」，终端里查那个群（群号 -1001）的记录，结果前头写「Group -1001:」、有那一条。
- 本机门禁七项。
