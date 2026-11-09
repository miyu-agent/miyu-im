## 施工单 I-185 下上：Telegram 撤回、贴表情

状态：做完了（2026-11-10；设计 07 第 3.5 节第一版「说话」那一块：贴表情、撤回自己的。`tg_send_file` 等 LV-4，叫 I-185 下下）。

### 为什么

Telegram 的群和私聊里 AI 还不能撤回、不能贴表情。

### 怎么改

1. 两件工具（`tools/acts.rs`，照 QQ 的 `qq_recall`、`qq_react`）：
   - `tg_recall {msg?}`：撤她给的编号那一条；没给的撤叫她做的那条引用的那一条，都没有的答 `no-quote`。
   - `tg_react {msg, emoji}`：私聊、群都给，一条消息机器人只能贴一个；Telegram 不认的表情照它的原话答。
   - 她给的编号先照 im 的聊天记录核对在这个场所里（Telegram 的编号是一个聊天里自己数的，也不给机器人取消息），没有的答 `not-found`、不去调。
2. 驱动的连接：
   - 撤回照 `deleteMessage`。
   - 贴、摘照 `setMessageReaction`（摘交空的）。
   - 能力表多 `recall`、`react`。
3. 判过要回的那一条上宿主自己贴的表情按平台给（`Driver::reaction`）：
   - 原来全局一个，照 `bridge.json` 的 `reaction_emoji`，是 QQ 的表情编号 `289`。
   - Telegram 开了贴表情以后宿主也照 289 贴，Telegram 不认。
   - 现在驱动给了的照驱动的：Telegram 是 👀，同 289「睁眼」。没给的照旧，QQ 不变。
   - 会话浮窗里能挑的表情照驱动的列表：Telegram 列了常用的十个。
4. 数据：
   - 两份说明。`tg_recall` 和 QQ 一字不差；`tg_react` 的「this group」改「this chat」，私聊也能贴。
   - 三句结果和 QQ 的字节一样；中英日显示名、网页活动记录的名字；功能表「聊天动作」多两件。
   - 登记簿五行。
5. Telegram 的工具面九件合计 799 个 token、1746 字节，超了上一步照七件定的 1522。项目主人 11-10 定 Telegram 的预算和 QQ 的一样（4312 字节）：一个场所会话只看得到一个平台的那一套。

### 坑

- 第一趟集成测试红在「贴的不是那一条」：宿主自己在叫她的那条上贴了 `289`。这是打开 `react` 能力带出来的，照第 3 条修了。测试里留了一条断言守它：去掉驱动给的 👀，测试报红、看到的是 289。

### 验收

- 集成测试 `telegram_acts`（真核心、本机假 Bot API，模型照剧本）：
  - 贴 👍 照 `setMessageReaction` 贴在 50 上，撤 50 照 `deleteMessage`。
  - 没给编号也没引用的答 no-quote，记录里没有的答 not-found，这两样都没去调。
  - 宿主在判过要回的 51 上贴的是 👀。
- 别的贴表情测试（`reactions::*`，QQ）照旧过；本机门禁、三台 CI 全绿。
