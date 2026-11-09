## 施工单 I-185 中：Telegram 发图、发文件

状态：做完了（2026-11-10；设计 07 第 3.5 节第一版「发图」。I-185 再拆一次：上是收图，中是发图、发文件，下是贴表情、撤回、`tg_send_file`，`tg_send_file` 等 LV-4 的「放开」）。

### 为什么

Telegram 驱动原来只发得出字。下面几样交过来都答「做不到」，什么都发不出去：
- AI 调核心的 `show` 给人看的图、文件；
- 长文画成的图；
- 发表情包。

### 怎么改

1. 驱动的连接答 `Outgoing` 的几样（`link.rs`、新的 `media.rs`）：
   - 图（`Image`）照 `sendPhoto` 传；表情包也照图发，`sendSticker` 只收 512 px 以内的 WebP、PNG。
   - 长文画成的图（`Picture`）照 `sendPhoto` 传，@ 写进图的说明，引用照 `reply_parameters`。
   - 文件（`Upload`）照 `sendDocument` 传，带文件名。
   - 合并转发 Telegram 没有，照旧答做不到，能力表不报。
2. 宿主交的是 `base64://…`，不是的照本机的路径读。
3. Bot API 的上限：图 10 MB、文件 50 MB。超了 10 MB 的图改照文件发，超了 50 MB 的答为什么。
4. 和发字同一条队，先后照调的；同一个聊天的间隔、429 等待照旧。传一份最多等 120 秒。
5. 请求体（`multipart/form-data`）自己拼，不为它多带依赖。
6. 能力表多 `upload`。
7. 测试夹具：
   - 假 Bot API 认 `multipart/form-data`：字的格照原样，文件那一格记文件名和 base64。
   - Telegram 的测试起法多一种「工具目录里有 `show`」。

### 验收

- 集成测试 `telegram_shown`（真核心、本机假 Bot API、工具目录里有 `show`）：主人私聊里她调两次 `show`。
  - 图照 `sendPhoto` 原样传到那个私聊。
  - 文本照 `sendDocument` 传、文件名对。
  - 运行日志两行 `file shown`。
  - 把发图改回做不到，这个测试报红。
- 单元测试：图照大小挑方法，超了的不发；base64、路径都读得出；请求体里的字、对象写成 JSON、文件名里的引号换行换掉、结尾分隔符。
- 本机门禁、三台 CI 全绿。
