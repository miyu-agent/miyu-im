## 给接入QQ 写插件：让你的软件包在 QQ 里生效

写给想在 QQ 里给 Miyu 加功能的第三方作者。你写的是一个普通的 Miyu 软件包（扩展、技能、脚本都行），装好以后她在 QQ 私聊、QQ 群里也能用。

这一页只讲「在 QQ 里」和在终端、网页里不一样的地方。包怎么写、清单怎么写、扩展怎么说协议，看核心仓库的作者说明：[写一个 Miyu 软件包](https://github.com/miyu-agent/miyu/blob/main/docs/authors/README.md)。每个方法的参数、回应、错误码在核心的[协议参考](https://github.com/miyu-agent/miyu/blob/main/docs/reference/README.md)（场所会话那几个在 [`venues.md`](https://github.com/miyu-agent/miyu/blob/main/docs/reference/venues.md)）。调接入QQ 的方法、插进接入QQ 里面的挂接点，细节在本仓库的 [`docs/blueprint/extending.md`](../blueprint/extending.md)。

现成的例子：表情包包 [`miyu-memes`](https://github.com/miyu-agent/miyu-memes)（工具、注入提示、发图）、渲染包 [`miyu-render`](https://github.com/miyu-agent/miyu-render)（给接入QQ 调的方法）。

### 一、先知道的几件事

1. **QQ 里的对话也是会话**。每个群一个会话，每个私聊一个会话，叫「场所会话」。她在场所会话里说话、调工具，和在终端里一样；只是对面是 QQ 上的人，回话由接入QQ 发回 QQ。
2. **场所会话归系统账号**，不归管理员。它们由接入QQ 造、接入QQ 驱动，管理员在终端里看不到它们的工具确认框：场所会话里没人能确认。
3. **谁能叫她**：
   - 私聊：只有终端管理员和白名单成员（`onebot.whitelist`）。私聊的每一条她都回。
   - 群：只有白名单里的群（后台页的场所卡片），「其他群」出厂关着。群里的话她都听着，回不回由接入QQ 判（@ 她、引用她、触发词、判官打分）。
4. **你的工具在场所会话里有没有，看你登记时写的 `venues`**。接入QQ 不替你挑，核心照 `venues` 把工具放进对应的会话。

### 二、让工具在 QQ 里出现

扩展经 `provide` 登记工具时，`venues` 写这件工具给哪几种会话（核心作者说明[「写扩展」的登记工具](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.md#登记工具provide)）：

| `venues` 里写 | 出现在 |
|---|---|
| `local` | 终端、网页里管理员自己的会话 |
| `private` | QQ 私聊 |
| `group` | QQ 群 |

```json
{"name": "meme_send", "description": "Send a sticker as a message of its own.",
 "input_schema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]},
 "access": "read", "venues": ["local", "private", "group"]}
```

**访问类别（`access`）在场所会话里的规矩**：场所会话没人能确认，所以：

- `read`、`write`：照常放行。`write` 在只读开着时被核心拦下。
- `network`、`outbound`：要人确认的，场所会话里一律被拒。不要拿它们登记要在群里用的工具。
- `venue`：在平台上留下后果的动作（撤回、禁言、戳一戳）。场所会话里放行、不问人；本机会话里照 `outbound` 判（完全放开直接调，工作区级别问人）。
- 「这一轮的回话」不算出去：表情包的 `meme_send` 用 `read`，因为它只是她这一轮回话的一部分，由接入QQ 照回话发。

**谁在叫她**：`tool.call` 带着 `by`、`owner`（核心作者说明[「写扩展」的被调](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.md#被调toolcall)）：

- `by` 是她这时在回应的那一条消息是谁发的，例如 `{"kind": "external", "venue": "qq:group:123", "id": "qq:456", "role": "manager"}`。群里每来一条新消息、她看到了，`by` 就换成那一条的人。
- `owner` 是核心认不认这个人是主人本人。
- 登记成 `venue` 的工具，**核心不替你挡人**：谁能用（只给主人？只给群管理的人？）要你自己照 `by.role`、`owner` 挡，挡下的照工具结果回一句、`error` 写真。

**是哪个群**：`tool.call` 交来的是会话编号。要知道是哪个群、经哪个机器人号，调接入QQ 的 `chat.venue`（见第六节）。

### 三、往对话里加提示

声明 `context.inject` 的扩展（核心作者说明[「写扩展」的更多本事](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.md#更多本事)），每一轮开始时核心问你一次 `user.append {"session", "venue", "text"?}`。`venue` 是 `local`、`private`、`group`，你照它决定在 QQ 里说不说、说什么。答 `{"content": [{"kind", "text"}]}`，至多 3 块，排在触发这一轮的那句话后面，记进日志、之后原样回放。

- 300 毫秒内答不上的当这一轮没交。
- 外壳、转义你自己做：`text` 原样进上下文。
- 只陈述「是什么」，不写命令；写英文短句。表情包包的提醒是 `<meme-hint>This reply comes with a sticker.</meme-hint>`，照概率给。
- 每一轮都发的字是每个群、每轮都要付的 token，克制。

### 四、发图

两条路：

1. **工具结果带 `sent`**（表情包这样）：先经 `blob.put` 把图传上去，工具回答里多一格 `sent`：

   ```json
   {"blocks": [{"type": "text", "text": "Sent."}],
    "sent": [{"blob": "<blob.put 交回的>", "media_type": "image/gif", "width": 240, "height": 240, "alt": "A cat waving.", "sticker": true}]}
   ```

   至多 4 张。`sent` 不发给模型（她看到的只有 `blocks`），接入QQ 照它发进 QQ：一张一条图片段，和她的话照她调的先后排进出站队列。`sticker` 是真的发成 QQ 表情（OneBot 图片段带 `sub_type=1`）。被禁言、排着过期了发不出去的，接入QQ 给她记一块 `unshown` 事实，她知道没发成。`path` 写了也不收：用 `blob.put`。

2. **她调核心的 `show`**：给人看工作区里的一个文件。图片发成图片，别的传成 QQ 文件。你的工具只要把文件写进工作区、交回路径，发不发由她定（渲染包的 `render_image` 就这样）。

### 五、群里和私聊的区别

| | 私聊 | 群 |
|---|---|---|
| 谁说得上话 | 终端管理员、白名单成员 | 白名单里的群里的所有人 |
| 她回哪些 | 每一条 | 接入QQ 判过的（@、引用、触发词、判官打分） |
| `by` | 对方 | 她这时在回应的那一条的发言人，会换 |
| 能不能碰你的工具 | 照 `venues` 有没有 `private` | 照 `venues` 有没有 `group` |
| 会话属于 | 系统账号 | 系统账号 |

群里人多、陌生人多：要动东西的工具一定照 `by`、`owner` 挡，别以为「她叫我做的」就是主人叫的。

### 六、系统账号与权限

- **工作区**：场所会话只碰得到自己的工作区（核心的文件工具查真实的位置）。你的扩展进程**不在这个限制里**：它是你的程序，读写什么核心管不着。所以别把 `tool.call` 里的路径直接打开：照真实的位置（解开链接以后）认，要在 `cwd`（这一轮的工作目录）里面；写文件先写新建的临时文件再改名，别顺着预先放好的链接写出去。渲染包的 `render_image` 是样板（`src/tools/render_image.rs`、`src/tools/saved.rs`）。
- **不可信的字**：群里的人能写任何字，昵称、群名片也是。你要把这些字放进给模型看的提示里的，转义、套外壳，别让人伪造一行「系统说」。
- **blob**：你 `blob.put` 的图存在管理员名下；接入QQ 读得到你经工具结果 `sent` 交出来的、经方法回应交给它的（核心在补这条规矩）。
- **核心拒的**：扩展调 `extension.*`、`package.install` 这类只给本机的人用的方法会被拒（`local_only`）。

### 七、和接入QQ 打交道

你的扩展能调接入QQ，接入QQ 也能调你。都经核心的 `package.call`（核心作者说明[「写扩展」的更多本事](https://github.com/miyu-agent/miyu/blob/main/docs/authors/extensions.md#更多本事)），清单声明 `packages.call`，人批过才行。

**你调接入QQ**（细节在 [`extending.md` 第一条](../blueprint/extending.md#一调接入qq-的方法)）：

- 挂接点、方法 2026-10-19 起叫 `chat.*`（原来是 `qq.*`）：接入QQ 交给你的参数、`chat.venue` 的回答都带 `platform`（现在只有 `qq`）；你调方法时可以写 `platform`，不写当 `qq`，写了别的平台的拒 `bad_params`。
- `chat.send {user | group, text}`：以她的名义发一条，过出站链，对方那个会话记一块事实，她知道是自己先找的人。
- `chat.action {action, params?, bot?}`：替你调一个 OneBot 动作（踢人、查群成员、取图……），NapCat 的回答原样交回。发出去的话她不知道是自己说的。
- `chat.venue {session}`：会话编号是哪个群、哪个机器人号。

**接入QQ 调你**：

- **挂接点**（细节在 [`extending.md` 第二条](../blueprint/extending.md#二插进接入qq-里面)）：导出约定名字的方法，装上就挂上。`chat.event`（QQ 的通知、请求原样交你）、`chat.inbound`（群消息判以前问你：照常、只记下、直接回）、`chat.outbound`（她的话发出去以前问你：换掉、不发、照原样）、`chat.score`（判官打完分问你加减分）。`chat.event` 不等你的回答，别的三处最多等 1.5 秒，等不到的照没意见算。
- **约定的服务**：接入QQ 认得的包编号、方法名，装上就用。现在有一个：`miyu-render` 的 `markdown {markdown}` → `{pages: [{blob, width, height}]}`，接入QQ 把超过阈值的长回话交给它画成图（每张场所卡片的「长文」一格）。你写一个同编号、同方法的包就能换掉画法；调不通、超时（15 秒）的接入QQ 照拆段发。

### 八、在 QQ 里试

1. 接入QQ 先跑起来：装 NapCat、登录机器人号，`miyu onebot start`，`miyu onebot web` 打开后台页把令牌填进 NapCat，终端管理员和白名单成员在后台页里设好。`miyu onebot status` 看连没连上。
2. 装你的包：`miyu pkg install ./你的包`，答「打开」并批能力（核心作者说明[「装、试、查」](https://github.com/miyu-agent/miyu/blob/main/docs/authors/README.md#装试查)）。场所会话的工具面跟着包变：已经开着的会话下一轮就换上。
3. 用终端管理员的 QQ 号私聊机器人号，让她调你的工具。群里试：后台页把一个群加进白名单、打开，@ 她。
4. 看发生了什么：
   - `miyu onebot logs -f`：接入QQ 的运行日志（判断、入队、发没发出去、挂接点答没答）。
   - `<数据根>/state/logs/<你的包>.stderr`：你的程序的标准错误。
   - `miyu onebot venue show qq:group:<群号>`：这个群每一项设置的值和来处。
5. 预设关着你的包时，场所会话里没有你的工具：看后台页的场所卡片用的是哪个人格、预设。

### 九、清单

```toml
[package]
protocol = [1, 1]
name = { en = "Hello", zh = "你好" }

[command]
name = "hello"
program = "hello"
about = { en = "Hello package", zh = "你好" }

[process]
args = ["serve"]
start = "always"
capabilities = ["tools", "context.inject", "packages.call"]   # 用到哪几样写哪几样

# 挂在接入QQ 的挂接点上的才写：
[depends]
packages = ["onebot"]

[export]
methods = ["chat.inbound"]
```
