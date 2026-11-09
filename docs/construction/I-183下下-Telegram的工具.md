## 施工单 I-183 下下：Telegram 的工具（说话、记录七件）

状态：做完了（2026-11-10；范围是项目主人当天定的：说话加记录一套）。

### 为什么

I-183 下中以后，Telegram 的群和私聊里一件工具都没有：不能「这一轮不说话」，不能引用指定的一条、@ 指定的人，不能查记录。设计 07 第 3.5 节第一版要「说话」那一块 Telegram 有的；翻记录、排行、看一眼没写做也没写不做，项目主人 11-10 定一起做（群卡片的「时不时看一眼」要靠 `tg_glance`）。贴表情、撤回、图和文件照排期在 I-185；群管理、群资料、戳一戳、群名片第一版不做。

### 怎么改

1. 宿主多一块 `toolset::common`：参数的读法、查聊天记录、发言排行、定下次看一眼、@ 人的参数和拼名字，从 OneBot 驱动原样挪来（行为不变，单元测试跟着挪），两个驱动同用；查记录 `user` 写错的那一句各平台自己给（QQ 照旧是「QQ number」）。
2. Telegram 驱动交一套工具（`tools.rs`，目录 `data/telegram/`）：
   - `tg_skip_reply`、`tg_send`（本机会话也给）、`tg_quote`、`tg_mention`（只给群）、`tg_chat_history`、`tg_activity_ranking`（只给群）、`tg_glance`（开着看一眼的场所）。
   - `tg_quote` 照 im 的聊天记录核对那一条在这个场所里（Telegram 没有「取一条消息」）。
   - `tg_mention` 认缓存着名字的、在这个群里说过话的人（Telegram 不给机器人列群成员），没见过的答「not seen in this group」。
   - 连接表里一个令牌都没有的不登记：没接 Telegram 的人，本机会话的工具面上不多一件 `tg_send`。
3. 出厂清单 `[features]`：说话五件进「聊天动作」，查记录、排行进「查询信息」。各驱动的单元测试只核对自己前缀的那几件都归一个功能。
4. 数据：七份说明（除 `tg_send` 的「friend」改「person」都和 QQ 的一字不差），二十句结果（四句改了平台名或说法，十六句和 QQ 的字节一样），三种语言的显示名；登记簿二十七行；Telegram 的工具面另有一份预算（实测加一成）。
5. 测试夹具：Telegram 的起法从 `telegram.rs` 挪进 `support/telegram.rs`，多一种写场所规则的起法。

### 坑

- 清单写了 `[features]` 的包，登记的工具要归某一个功能，不然核心**整批不收**（`bad_tool`，`problem: feature`）：连 QQ 的工具也一起登记不上。第一趟集成测试她调 `tg_quote` 得到「没有这件工具」，运行日志里 `tools not provided reason=bad_tool` 才看出来。现在两个驱动各有单元测试核对，去掉清单那两行测试报红。
- magpie 网关上的模型名现在是 `opencode-go/deepseek-v4.1-flash`（以前的 `clinepass/…` 答 404）。

### 验收

- 集成测试 `telegram_tools`（真核心、真程序、本机假 Bot API，模型照剧本）：
  - 群里 Alice 随口说一句只记下。
  - 叫她：`tg_quote` 那一条、`tg_mention` Alice，下一句带 `reply_parameters` 和 `text_mention`。
  - 记录里没有的编号答 not-found；没说过话的人答 not-members，下一句不带。
  - `tg_chat_history` 查到 Alice 那一条。
  - 那个群的工具面上有 `tg_*`、没有 `qq_*`。
- 单元测试：功能表核对（两个驱动），没令牌不登记，宿主挪来的参数、记录、排行、看一眼、@ 人。
- 工具面：七件 677 个 token（QQ 二十一件 1626，没变）；预算测试两份。
- 本机门禁、三台 CI 全绿。
