## 施工单 AUD-3：戳一戳的编号、Telegram 收发四处

状态：做完了（2026-11-18），等项目主人验收。叠在 AUD-2 上。

### 起因

2026-11-18 全项目审计第四章 im 段 #107–#110，core1 转来：每条先二次核实，成立的照「改前红」修。

### 核实和改了什么

1. **#107 四点都成立**：
   - 戳一戳的编号认法是「比 32 位整数最大的大」，64 位编号的 OneBot 实现，正常消息都被当成戳一戳：引用不了、贴不了表情，后台动态把没有字的图、文件显示成「戳了戳 AI」。
   - 编号直接用毫秒数，同一毫秒两次戳会撞号，后一次被当成重复丢掉。
   - 出站链 `pass()` 漏了不引用戳一戳那一道（`relead()` 有）。
   - 后台动态的条件写成 `fields.who || fields.text`，戳一戳没有字也没有人名时退成「一条消息」。

   改法：编号改由 `chat::poke_number` 编，是负的毫秒数，同一毫秒的往下数，比 32 位整数最小的还小。NapCat 的编号是正的 32 位整数，64 位编号的实现给的也是正数，都落不进去。`is_poke` 照它认，`pass()` 补上，`activity.js` 照新的认、条件改成 `text`。
2. **#108 成立**：长轮询只在新编号比记着的大时才往前走。Telegram 一周没有新消息、自建的 Bot API 重建以后编号会重新起；照老编号问，Bot API 照队头重交同几条（`telegram-bot-api` 的 `TQueue::get` 遇到「from_id in the future」退回队头），永远不往前走。改成照这一批最大的那条接着收。
3. **#109**：
   - 第 1 点成立：图的说明（@ 人）用的是发字的 4096 上限，Telegram 的 `caption` 只收 1024。改照 1024。
   - 第 2 点不成立：切分本来就按 UTF-16 单位算（`split` 里逐字加 `len_utf16`）。
   - 第 3 点成立：@ 多到第一段装不下时，越过第一段的 `text_mention` 实体照样挂上，Telegram 整条拒收。装不下的不挂。
4. **#110 两点都成立**：
   - GIF 动图照 `sendPhoto` 发，Telegram 只留第一帧。改照 `sendAnimation` 发，至多 50 MB。
   - 群里匿名的管理员说话，`from` 是 GroupAnonymousBot（`is_bot`），整条被当成机器人丢掉。改照 `sender_chat` 认：号是那个群（或频道），名字照署名，没署名的照群名。别的机器人照旧不收。

### 不加给模型看的字

不加。

### 怎么验

- 单元：
  - `chat/event/tests.rs`：64 位、32 位正负的消息编号都不是戳一戳，连编一千个都认得出、不重。
  - `outbound/tests.rs` `a_poke_is_never_quoted`。
  - Telegram 的 `send/tests.rs` `mentions_past_the_first_part_carry_no_entity`、`media/tests.rs` `a_gif_goes_as_an_animation`、`receive/tests.rs` `an_anonymous_admin_speaks_as_the_group_and_other_bots_are_skipped`。
- `telegram.rs` `update_numbers_that_start_over_are_followed`：收到第 41 条以后又来第 5 条，接着照 6 问。
- 改前红：每处修复各自退回，对应的测试都红。门禁。
