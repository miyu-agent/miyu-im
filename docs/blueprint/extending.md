## 给接入QQ 写扩展包

状态：图纸，随施工 O-42 起草（2026-10-17），O-43 上写了第二条的找法和 `chat.event`，O-43 中写了 `chat.inbound`，O-43 下写了 `chat.outbound`，O-43 末写了 `chat.score`，O-52 `chat.inbound` 多带 `media`，O-56 多 `chat.send`。项目主人 2026-10-17 定：接入QQ 的新功能以后能做成单独的包装，例如 `miyu pkg install miyu-onebot-qq-admin`。扩展口分三步：给别的包调的方法（O-42，第一条）、包之间的依赖（核心做，做完补进第一条的清单）、插进桥里面（进站、出站、QQ 事件、判断打分，O-43，第二条）。

### 是什么

写给扩展包作者的约定：一个扩展包（核心的 `packages.md`，有自己的程序）怎么让 QQ 做事、怎么知道一个会话是哪个群。只写对外的样子；桥这一边怎么做在 `onebot.md` 第一条「给别的包调的方法」。

### 一、调接入QQ 的方法

**清单**：

```toml
[process]
capabilities = ["packages.call"]   # 调别的包的方法，开的时候人批（核心 X-18）
```

经核心的 `package.call {"package": "onebot", "method", "params"}` 调；核心转给接入QQ，回什么交回什么（核心的 `protocol.md`「`package.call`」）。没批 `packages.call` 的核心回 `needs_approval`；接入QQ 没在跑回 `program_not_running`；接入QQ 拒了的回 `method_failed`，`data.message` 是下面写的原因。

**平台**（施工 I-1，设计 01 第四节第 3 条）：挂接点和方法从 `qq.*` 改名成 `chat.*`，准备接别的平台。接入通讯平台交给你的参数（每一个挂接点）、`chat.venue` 的回答都带 `"platform"`（现在只有 `"qq"`）；你调 `chat.action`、`chat.venue`、`chat.send` 时可以写 `"platform"`，不写的当 `"qq"`，写了别的平台、不是字的拒 `bad_params`。

**`chat.action`**：让 QQ 做一件事，例如踢人、禁言、查群成员、发消息。

| 参数 | 写法 |
|---|---|
| `action` | OneBot 动作名（NapCat 认的那些）：小写字母、数字、`_`、`.`，1 到 64 个 |
| `params` | 动作的参数，对象；不写是 `{}` |
| `bot` | 经哪个机器人号；不写是连着的号里最小的那一个 |

回答是 NapCat 的回答原样：`{"status": "ok" | "failed", "retcode", "data", "message", "wording"}`。NapCat 说没成也是回答，看 `status`、`retcode`。拒绝：`bad_params`（写错）、`not_connected`（那个号没连着）、`unanswered`（NapCat 没回）。

用 `chat.action` 发的消息，AI 不知道是自己说的：不过接入QQ 的出站链（清理、去重）、不记进会话。要以 AI 的名义说话的，用下面的 `chat.send`。

**`chat.send`**（施工 O-56）：以 AI 的名义发一条，同 AI 自己用 `chat_send` 发。

| 参数 | 写法 |
|---|---|
| `user`、`group` | 好友的 QQ 号、群号，写正好一个 |
| `text` | 要说的话，去掉首尾空白不空，最多 4000 个字符 |
| `bot` | 经哪个机器人号；不写是连着的号里最小的那一个 |

照那个场所的出站链过一遍（去末尾的句号这些），太长的拆成几段发；对方那个场所的会话找得到的，给它记一块事实，AI 下次在那里说话时知道是自己先找的人。回答 `{"sent": <几段>}`。拒绝：`bad_params`（写错）、`not_connected`（没连着）、`failed`（NapCat 说没成）、`unanswered`（NapCat 没回）。

**`chat.venue`**：`{"session": <会话编号>}` → `{"platform": "qq", "venue": "qq:group:<群号>" | "qq:private:<号>", "bot": <机器人号>}`；接入QQ 认不得的会话是 `{"venue": null}`（还没来过消息的群、回完封存了的支线）。你提供的工具被调时，核心的 `tool.call` 交来的是会话编号，用它查是哪个群。

### 二、插进接入QQ 里面

导出约定名字的方法，接入QQ 在那一处经核心调你（施工 O-43）。不用另外登记：接入QQ 照核心的软件包列表找，你的包装上、跑起来就挂上，停了、卸了就没了。几个包挂在同一处的照包的编号排先后。

**清单**：

```toml
[depends]
packages = ["onebot"]              # 没装接入QQ 的装不上（核心 F-11）

[export]
methods = ["chat.event"]             # 挂在哪几处
```

经 `package.methods` 登记同样的名字，核心转来的 `method.call` 带 `from: "onebot"`。

**`chat.event`**（QQ 事件，施工 O-43 上）：NapCat 推来的每一件通知、请求（OneBot 的 `post_type` 是 `notice`、`request` 的，进群、退群、戳一戳、撤回、好友请求……）原样交给你：`{"platform": "qq", "bot": <机器人号>, "event": <OneBot 事件原样>}`。每件各自交、核心也各自转，先后不保证，要排先后的看事件里的 `time`。接入QQ 不等你的回答、不看它，回 `{}` 就行；它自己该办的照办（撤回记进会话、好友请求照白名单……）。要让 QQ 做事的，调第一条的 `chat.action`。

**`chat.inbound`**（进站，施工 O-43 中）：群里一条人说的话记下以后、接入QQ 判回不回以前问你：

```json
{"venue": "qq:group:<群号>", "bot": <机器人号>, "msg": "<平台的消息编号>", "sender": {"id": "qq:<号>", "name": "<名字>"},
 "text": "<正文，@ 已经写成名字>", "mentions_me": <@ 了 AI>, "quotes_me": <引用了 AI 的话>,
 "media": [{"index": 1, "kind": "image", "id": "<平台的编号>", "name"?: "<名字>", "size"?: <字节数>}]}
```

`media`（施工 O-52）是这条带的东西，照先后：`index` 从 1 数（同 AI 看到的 `#n`），`kind` 是 `image`、`sticker`（表情包、小黄脸、商城表情）、`video`、`file`、`voice`，`id` 是平台的编号，`name`、`size` 有的才写；没带的是空的列表。要图本身的，调第一条的 `chat.action`：`{"action": "get_image", "params": {"file": "<id>"}}`，NapCat 回本机的路径和网址（`data.file`、`data.url`）；视频、文件照 `get_file`。表情包包偷图就照这样做，接入QQ 不知道有表情包这回事。

回 `{"route": "pass"}`（照常）、`{"route": "record"}`（只记下，AI 不接这一条）、`{"route": "reply"}`（当成冲 AI 来的，直接回、不过判官）。

- 最多等 1.5 秒（`bridge.json` 的 `hook_millis`）：等不到、出错、回得不对的照 `pass`。等你的时候这一条和后面的消息都排着，答得快些。
- 几个包挂在这里的，`record` 盖 `reply` 盖 `pass`。
- 终端管理员的消息你也会收到，但 `record` 对它不算；`reply` 不越过限流、睡觉时间、关掉的群。
- 只问群消息，私聊不问。
- 要让 QQ 做事（踢人、撤回……）的，回答以外另调第一条的 `chat.action`。

**`chat.outbound`**（出站，施工 O-43 下）：AI 的一段话（群里、私聊）发出去以前问你：`{"platform", "venue", "bot", "text": "<这一段>"}`。回 `{"text": "<换成的>"}` 换掉它，`{"drop": true}` 不发，`{}` 照原样。

- 几个包挂在这里的照编号一个接一个：前一个换过的交给后一个；有一个说不发就不再问后面的。
- 最多等 1.5 秒；等不到、出错、写法不对的照原样发。等你的时候她后面的话排着，答得快些。
- 换过的照样过接入QQ 的出站链（清理 Markdown、去重、拆段）。不发的 AI 不知道：它以为自己说了。
- 命令回执、限流提示这类不是 AI 说的话，不问你。

**`chat.score`**（判断打分，施工 O-43 末）：群里一条消息要 AI 的判官打分时，判官答了以后问你：

```json
{"venue": "qq:group:<群号>", "msg": "<平台的消息编号>", "conditions": [{"kind": "direct", "bonus": 0.3}],
 "scores": [<相关>, <意愿>, <社交>, <时机>, <连贯>], "should_reply": <判官说该不该回>, "to_bot": <判官说是不是在跟 AI 说话>}
```

回 `{"bonus": <-1 到 1 的数>}`：加进总分（正的更想回，负的更不想回），门槛是 0 到 1 左右的数。几个包的加起来再夹到 -1 到 1。最多等 1.5 秒，等不到、出错的照 0。只查违规的、判官判不了的不问你；违规够重的照回，减分减不掉。

### 起草时定的

| # | 定了什么 | 为什么 | 没选 |
|---|---|---|---|
| 1 | `chat.action` 不挑动作 | 人批 `packages.call` 的时候就是在批这个包能让 QQ 做事；列一张能调的动作表要跟着 NapCat 改，挡不住真想做事的包 | 只开一张动作表 |
| 2 | 方法名带 `qq.` | 和第二条别的包导出、接入QQ 来调的约定方法放在同一个名字空间，作者一眼看出是接入QQ 的约定 | 不带前缀 |
| 3 | `chat.venue` 照接入QQ 手里的表答 | 你的工具被调的那个会话一定是接入QQ 认得的场所会话；去问核心，别的包又没有系统账号 | 请核心给别的包开 `venue.sessions` |
