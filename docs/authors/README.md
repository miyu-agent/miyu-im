# 给通讯平台写插件：让你的软件包在 QQ 里生效

> <sub>本文档由 Anthropic 的 Claude Fable 5.2 生成。</sub>

这一页写给想在 QQ 里给 Miyu 增加功能的第三方作者。你写的是一个普通的 Miyu 软件包（扩展、技能、脚本都可以），安装之后，AI 在 QQ 私聊和 QQ 群里也能使用它。

这一页只讲在 QQ 里和在终端界面、网页界面里不一样的地方。软件包怎么写、清单怎么写、扩展怎么使用协议，可以看核心仓库的作者说明：[写一个 Miyu 软件包](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/packages.md)。每个方法的参数、回应和错误码都在核心的[协议参考](https://github.com/miyu-agent/miyu/blob/main/docs/zh/reference/README.md)里（场所会话相关的几个方法在 [`venues.md`](https://github.com/miyu-agent/miyu/blob/main/docs/zh/reference/venues.md) 里）。如何调用通讯平台的方法、如何接入通讯平台内部的挂接点，细节在本仓库的 [`docs/blueprint/extending.md`](../blueprint/extending.md) 里。

可以参考的现成例子有表情包软件包 [`memes`](https://github.com/miyu-agent/miyu-memes)（工具、注入提示、发图）和渲染软件包 [`render`](https://github.com/miyu-agent/miyu-render)（提供给通讯平台调用的方法）。

## 一、需要先知道的几件事

在写插件之前，你需要先了解以下几点：

1. **QQ 里的对话也是会话**。每个群对应一个会话，每个私聊也对应一个会话，这类会话叫“场所会话”。AI 在场所会话里说话、调用工具，和在终端界面里一样；区别只是对面是 QQ 上的人，AI 的回复由通讯平台发回 QQ。
2. **场所会话属于系统账号**，不属于管理员。场所会话由通讯平台创建和驱动，管理员在终端界面里看不到这些会话的工具确认框，所以场所会话里没有人能确认工具调用。
3. **谁能叫 AI**：
   - 私聊：只有终端管理员和白名单成员（`im.whitelist`）可以。私聊里的每一条消息，AI 都会回复。
   - 群：只有白名单里的群（后台页里的场所卡片）可以，“其他群”出厂时是关闭的。AI 会听群里的所有消息，是否回复由通讯平台判断（@ AI、引用 AI 的消息、触发词、判官打分）。
4. **你的工具在场所会话里是否可用，取决于你登记工具时写的 `venues`**。通讯平台不会替你挑选工具，核心根据 `venues` 把工具放进对应的会话。

## 二、让工具在 QQ 里出现

扩展通过 `provide` 登记工具时，用 `venues` 写明这个工具给哪几种会话使用（见核心作者说明[「写扩展」的登记工具](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/extensions.md#登记工具provide)）：

| `venues` 里写的值 | 工具出现在 |
|---|---|
| `local` | 终端界面、网页界面里管理员自己的会话 |
| `private` | QQ 私聊 |
| `group` | QQ 群 |

```json
{"name": "meme_send", "description": "Send a sticker as a message of its own.",
 "input_schema": {"type": "object", "properties": {"id": {"type": "string"}}, "required": ["id"]},
 "access": "read", "venues": ["local", "private", "group"]}
```

**访问类别（`access`）在场所会话里的规则**：由于场所会话里没有人能确认，所以：

- `read`、`write`：正常放行。如果开启了只读，`write` 会被核心拦下。
- `network`、`outbound`：这两类需要有人确认，所以在场所会话里一律被拒绝。不要用它们登记需要在群里使用的工具。
- `venue`：用于在平台上留下后果的动作（撤回、禁言、戳一戳）。这类工具在场所会话里直接放行，不询问任何人；在本机会话里按 `outbound` 的规则判断（权限完全放开时直接调用，工作区级别时询问人）。
- “这一轮的回话”不算向外发送：表情包软件包的 `meme_send` 登记为 `read`，因为它发出的表情只是 AI 这一轮回话的一部分，由通讯平台随回话一起发送。

**谁在叫 AI**：`tool.call` 会带上 `by` 和 `owner`（见核心作者说明[「写扩展」的被调](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/extensions.md#被调toolcall)）：

- `by` 是 AI 这时正在回应的那一条消息的发送者，例如 `{"kind": "external", "venue": "qq:group:123", "id": "qq:456", "role": "manager"}`。在群里，每来一条新消息并且被 AI 看到，`by` 就会换成这条消息的发送者。
- `owner` 表示核心是否认定这个人是主人本人。
- 对于登记成 `venue` 的工具，**核心不会替你拦人**：谁能使用（只给主人？只给群里的管理员？）需要你自己根据 `by.role` 和 `owner` 判断并拦下；拦下时用工具结果回复一句说明，并把 `error` 设为真。

**是哪个群**：`tool.call` 交给你的是会话编号。如果要知道是哪个群、通过哪个机器人号，就调用通讯平台的 `chat.venue`（见第七节）。

## 三、往对话里加提示

扩展可以在每一轮对话里加一段提示。如果扩展声明了 `context.inject`（见核心作者说明[「写扩展」的更多本事](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/extensions.md#更多本事)），每一轮开始时，核心会调用一次 `user.append {"session", "venue", "text"?}` 询问你。`venue` 的值是 `local`、`private` 或 `group`，你可以根据它决定在 QQ 里要不要提示、提示什么。回答的格式是 `{"content": [{"kind", "text"}]}`，最多 3 块；这些内容排在触发这一轮的那句话后面，会记进日志，之后原样回放。

- 如果 300 毫秒内没有回答，就当作这一轮没有提交内容。
- 外壳和转义需要你自己处理：`text` 会原样进入上下文。
- 只陈述“是什么”，不写命令，并且用英文短句写。表情包软件包的提示是 `<meme-hint>This reply comes with a sticker.</meme-hint>`，按一定的概率提供。
- 每一轮都发送的文字，是每个群在每一轮都要付出的 Token，所以要克制。

## 四、发图

发图有两种方法：

1. **在工具结果里带上 `sent`**（表情包软件包使用的是这种方法）：先通过 `blob.put` 把图片上传，然后在工具的回答里多加一项 `sent`：

   ```json
   {"blocks": [{"type": "text", "text": "Sent."}],
    "sent": [{"blob": "<blob.put 交回的>", "media_type": "image/gif", "width": 240, "height": 240, "alt": "A cat waving.", "sticker": true}]}
   ```

   每次最多 4 张。`sent` 不会发给模型（AI 看到的只有 `blocks`），通讯平台根据它把图片发进 QQ：每张图片是一条图片段，和 AI 说的话一起，按 AI 调用的先后顺序排进出站队列。`sticker` 为真时，图片会发成真正的 QQ 表情（OneBot 图片段带 `sub_type=1`）。如果图片因为被禁言、排队过期而发不出去，通讯平台会给 AI 记一块 `unshown` 事实，让 AI 知道没有发送成功。即使写了 `path` 也不会被接受，需要使用 `blob.put`。

2. **AI 调用核心的 `show`**：把工作区里的一个文件给人看。图片会发成图片，其他文件会作为 QQ 文件上传。你的工具只需要把文件写进工作区并返回路径，是否发送由 AI 决定（渲染软件包的 `render_image` 就是这样做的）。

## 五、群里和私聊的区别

下表对比了私聊和群里的不同之处。

| | 私聊 | 群 |
|---|---|---|
| 谁能说话 | 终端管理员、白名单成员 | 白名单里的群的所有成员 |
| AI 回复哪些消息 | 每一条 | 通讯平台判断过的消息（@、引用、触发词、判官打分） |
| `by` | 对方 | AI 这时正在回应的那一条消息的发送者，会变化 |
| 能否使用你的工具 | 取决于 `venues` 里有没有 `private` | 取决于 `venues` 里有没有 `group` |
| 会话属于 | 系统账号 | 系统账号 |

群里人多，陌生人也多：需要改动东西的工具一定要根据 `by` 和 `owner` 拦人，不要以为“AI 叫我做的”就是主人叫的。

## 六、系统账号与权限

场所会话属于系统账号，下面几点和它的权限有关：

- **工作区**：场所会话只能访问自己的工作区（核心的文件工具检查的是真实的位置）。但是你的扩展进程**不受这个限制**：它是你的程序，它读写什么，核心都无法干预。所以不要直接打开 `tool.call` 里的路径，而要按真实的位置（解开链接之后）判断，并且要求它在 `cwd`（这一轮的工作目录）里面；写文件时，先写一个新建的临时文件再改名，不要顺着预先放好的链接写到外面去。渲染软件包的 `render_image` 是可以参照的样板（`src/tools/render_image.rs`、`src/tools/saved.rs`）。
- **不可信的文字**：群里的人可以写任何文字，昵称和群名片也一样。如果你要把这些文字放进给模型看的提示里，就要转义、套上外壳，不要让人伪造出一行“系统说”。
- **`blob`**：你通过 `blob.put` 上传的图片存在管理员名下；通讯平台能读取你通过工具结果的 `sent` 交出的图片，以及你通过方法回应交给它的图片（核心正在补上这条规则）。
- **核心拒绝的调用**：扩展调用 `extension.*`、`package.install` 这类只给本机用户使用的方法时，会被核心拒绝（`local_only`）。

## 七、和通讯平台交互

你的扩展可以调用通讯平台，通讯平台也可以调用你的扩展。你调用通讯平台时，使用它在总线上的名字 `chat`：`bus.call {"name": "chat", "method": "venue", "params"}`（`method` 写 `chat.` 后面的那一段，不写通讯平台的包编号）；通讯平台调用你时也走总线，使用你接入的名字 `chat.hook`（见核心作者说明[「写扩展」的更多本事](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/extensions.md#更多本事)）。这需要在清单里声明 `packages.call`，并且经过人批准才能使用。

**你调用通讯平台**（细节在 [`extending.md` 第一条](../blueprint/extending.md#一调接入通讯平台-的方法)）：

- 挂接点和方法都叫 `chat.*`：通讯平台交给你的参数和 `chat.venue` 的回答都带有 `platform`（目前只有 `qq`）；你调用方法时可以写 `platform`，不写时当作 `qq`，写了其他平台会被拒绝，错误是 `bad_params`。
- `chat.send {user | group, text}`：以 AI 的名义发送一条消息。这条消息会经过出站链，对方的会话会记一块事实，让 AI 知道是自己先找的对方。
- `chat.action {action, params?, bot?}`：替你调用一个 OneBot 动作（踢人、查询群成员、获取图片……），NapCat 的回答原样交还给你。通过这种方式发出去的话，AI 不知道是自己说的。
- `chat.venue {session}`：查询一个会话编号对应的是哪个群、哪个机器人号。

**通讯平台调用你**：

- **挂接点**（细节在 [`extending.md` 第二条](../blueprint/extending.md#二插进接入通讯平台-里面)）：在清单里声明 `plugs = ["chat.hook"]`，导出并登记这个名字下面约定的方法，软件包安装并运行之后就挂上了。这些挂接点是：`chat.hook.event`（把 QQ 的通知和请求原样交给你）、`chat.hook.inbound`（判断群消息之前询问你：正常处理、只记下、直接回复）、`chat.hook.outbound`（AI 的话发出去之前询问你：替换、不发送、保持原样）、`chat.hook.score`（判官打完分之后询问你要加分还是减分）。`chat.hook.event` 不等你的回答，其他三处最多等待 1.5 秒，等不到回答时当作没有意见。
- **约定的服务**：这是通讯平台认识的包编号和方法名，安装之后通讯平台就会使用。目前有一个：`render` 的 `markdown {markdown}` → `{pages: [{blob, width, height}]}`，通讯平台会把超过阈值的长回复交给它渲染成图片（每张场所卡片上的 `长文` 一项）。你写一个编号相同、方法相同的软件包，就能替换渲染的方式；如果调用失败或者超时（15 秒），通讯平台会改为把文字拆成几段发送。

## 八、在 QQ 里测试

下面是在 QQ 里测试你的软件包的步骤：

1. 先让通讯平台运行起来：安装 NapCat 并登录机器人号，运行 `miyu im start`，再运行 `miyu im web` 打开后台页，把令牌填进 NapCat，并在后台页里设置好终端管理员和白名单成员。运行 `miyu im status` 可以查看是否已经连上。
2. 安装你的软件包：运行 `miyu pkg install ./你的包`，回答“打开”并批准能力（见核心作者说明[「装、试、查」](https://github.com/miyu-agent/miyu/blob/main/docs/zh/dev/packages.md#装试查)）。场所会话的工具会随软件包变化：已经开着的会话在下一轮就会换上新的工具。
3. 用终端管理员的 QQ 号私聊机器人号，让 AI 调用你的工具。如果要在群里测试，就在后台页里把一个群加进白名单并打开，然后 @ AI。
4. 查看发生了什么：
   - `miyu im logs -f`：通讯平台的运行日志（判断、入队、是否发送成功、挂接点是否回答）。
   - 核心给你的程序分配的日志目录 `MIYU_LOG_DIR` 里的 `stderr`：你的程序的标准错误输出（核心提供的几个目录都要从环境变量读取，不要用 `MIYU_HOME` 自己拼路径）。
   - `miyu im venue show qq:group:<群号>`：这个群每一项设置的值和来源。
5. 如果预设关闭了你的软件包，场所会话里就没有你的工具：此时要查看后台页的场所卡片使用的是哪个人格和预设。

## 九、清单

下面是一个清单的例子：

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
capabilities = ["tools", "context.inject", "packages.call"]   # 用到哪几项就写哪几项

# 只有挂在通讯平台挂接点上的软件包才需要写：
[depends]
packages = ["im"]

[export]
plugs = ["chat.hook"]
methods = ["chat.hook.inbound"]
```
