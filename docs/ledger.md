## 登记簿

接入QQ 给模型看的每一份字都登记在这里：包文件夹 `package/data/` 里判官的说明、平台工具的说明、事实、工具结果。这一页 2026-10-16 从核心仓库的设计 `26-提示词.md` 第十节搬来（施工 S-6 中，设计 32 第九节第 3 条：包的字由包自己登记），核心那边留一行指路；量法、每一行照搬，路径换成这个仓库里的。

### 规矩

- 一份一行：相对仓库根的路径、进到哪、什么时候加进来、实测多少 token、为什么加（证据、哪一步），指纹是文件字节的 SHA-256 的前 8 位。
- 事实、工具结果加进来以后留在历史里，以后每次请求都带着，直到压缩；工具说明在 tools 数组里，每次请求都带；判官的字只在判官那一次辅助请求里，不进主对话。
- 仓库里没有分词器，门禁不查 token、只查指纹（`cargo xtask check` 的「文档」一项，`xtask/src/ledger.rs`）：字改了就对不上，重新量过、写上为什么改，再换指纹；`package/data/` 里加了给模型看的文件没登记的、登记了却没有文件的，同样拦。
- 只登记会发给模型的：`human/` 下给人看的字不算；`package/data/` 里的四份数据（`bridge.json`、`defaults.toml`、`moderation.txt`、`venues.d/50-defaults.toml`）不发给模型，也不算；清单 `package.toml`、后台页 `page/` 是给人看的，不在 `data/` 里，不查。
- 给模型看的字一律英文短句，克制、有账：非必要不加，加之前先实测证明不加不行、量出多少 token（核心设计 `00-设计理念.md` 第三节、`26-提示词.md` J3）。

### 量法

- 施工 O-11 的十三份（`package/data/judge/`）2026-10-07 核心的主会话照开发端点（`deepseek-v4.1-flash`）量：每份单独接在 `hi` 和一个换行后面量，带行尾换行，`violations.txt` 的 `{severity_min}` 填 7。整块核对：打分那一次的 system（不带人格：`system`、`reply`、`violations`、`answer` 连起来）648 个，正好是四份的和；user 那一条（两行假记录、四个标签）51 个，标签合起来 20。它们只在判官那一次辅助请求里（`model.call`，`purpose` 是 `judge`），不进主对话；`reply.txt` 367 个是大头，判官每次请求都带。
- 施工 O-25 下的一份（`package/data/facts/undelivered.txt`）2026-10-09 照开发端点（magpie 网关的 `clinepass/cline-pass/deepseek-v4.1-flash`）量：接在 `hi` 和一个换行后面量，带行尾换行，字段照出厂的样子填（`why` 是 `expired`，`detail` 空的，`text` 是 30 个字符的中文开头「大家说得都有道理。那就这么定了：周六上午十点在东门集合，别迟」）是 49；同一句中文单独量是 20，模板本身（`text` 空的）29。一段没发出去才有一块，进了历史留到压缩，平常一个字节不多。
- 施工 O-26 的三份（`package/data/tools/skip_reply.json`、`tool-results/skipped.txt`、`tool-results/unknown.txt`）2026-10-09 照开发端点（magpie 网关的 `clinepass/cline-pass/deepseek-v4.1-flash`）量：工具面照「带工具减不带」，十六件（基础系统十五件加它）一起时 `skip_reply` 的边际份量 70，整个 tools 数组 2676 → 2746；第一稿参数带一句说明（「一句话、只进日志」）是 86（322 字节），短一点的 81，名字看得出是理由，去掉（`onebot.md` 施工时定的第 143 条）。说明 249 字节，桥单有一份预算（`10-自带软件.md` 第九节）。两句每份单独接在 `hi` 和一个换行后面量，带行尾换行：`skipped.txt` 14，`unknown.txt`（`{name}` 填 `recall`）9。工具面上只有场所会话（私聊、群）有它，本机的会话一个字节没变。
- 施工 O-31 的十六份（`package/data/tools/recall.json`、`mute.json`、`poke.json`，`tool-results/` 里成了的四句、不做和没成的九句）2026-10-10 照开发端点（magpie 网关的 `clinepass/cline-pass/deepseek-v4.1-flash`）量：工具面照「带工具减不带」，十九件（基础系统十五件、`skip_reply` 和这三件）一起时边际份量 `recall` 36、`mute` 80、`poke` 46、`skip_reply` 70，整个 tools 数组 2746 → 2908（多 162）；`mute` 的参数第一稿带 `minimum`、`maximum` 和「0 unmutes.」是 87，只带 `maximum` 81，什么都不写 65（看不出怎么解禁），定成说明里写「0 unmutes; at most 2592000.」80（`onebot.md` 施工时定的第 186 条）。说明三份 576 字节，桥的四件合计 825 字节（`10-自带软件.md` 第九节）。十三句每份单独接在 `hi` 和一个换行后面量，带行尾换行，字段照典型值（`who` 填 `小明`，`duration` 填 `10m`，`detail` 填 `Recall failed`）。`recall`、`poke` 在私聊、群的工具面上，`mute` 只在群的；本机的会话一个字节没变。结果的几句只在她调了这几件时进历史。
- 施工 O-38 改的一份（`package/data/tool-results/skipped.txt`）2026-10-12 照开发端点（magpie 网关的 `clinepass/cline-pass/deepseek-v4.1-flash`）量，接在 `hi` 和一个换行后面、带行尾换行：原来那一句 14，定下的 `Skipped: nothing in this reply was sent.` 11；候选 `Skipped: this turn ended; nothing in this reply was sent.` 15、`Skipped. Turn ended; nothing from this reply was sent.` 14，紧接着就是下一轮的话，「这一轮结束了」不用说。它在 `skip_reply` 那一步的结果里，以后的轮次照历史原样回放；以前的会话日志里记的是原来那一句，前缀不变。
- 施工 O-33 桥的八份（`package/data/tools/fetch_media.json`，`tool-results/` 里 `fetch_media` 的七句）2026-10-11 照开发端点（magpie 网关的 `clinepass/cline-pass/deepseek-v4.1-flash`）量：工具面照「带工具减不带」，二十件（基础系统十五件、桥的五件）一起时 `fetch_media` 的边际份量 114，整个 tools 数组 2908 → 3022；第一稿说明多一句「the path comes back」、`index` 写成「counting from 1」是 122。七句结果每份接在 `hi` 和一个换行后面量，字段照登记簿那一格填：`saved` 33、`not-found` 12、`no-item` 16、`not-fetchable` 13、`unfetched` 9、`too-many` 12、`not-saved` 12。`failed`、`unreachable`、`unanswered` 和平台工具（一）共用，没改。
- 施工 O-36 的一份（`package/data/facts/another-line.txt`）2026-10-11 照开发端点（magpie 网关）在真实位置量：真核心拉起真桥、真模型（`kimi-k3`）跑一遍「两个人几乎同时 @ 她」，抓下支线第一次请求的原样字节（人格、示范对话、工具面、事实外壳、近况都是真的），只换那一块，`kimi-k3`、`deepseek-v4.1-flash` 各 15 次：候选 a（原句 `Another line is already replying to msg=504.` 带外壳）重复回 A 1/15、3/15，`skip` 0、1；候选 b（末尾加 `You are replying to msg=505.`）重复回 A 0/15、0/15，`skip` 0、0，回 B 15/15、15/15；不带外壳的原句（草稿 `lane-marker/` 那种，另量的对照）0/15、2/15，`skip` 1、0。定 b。接在 `hi` 和一个换行后面量，带行尾换行，`msg`、`aim` 填十位的 QQ 消息编号：b 是 33，a 是 21。只在有别的线还在回的那一轮开头有，一条一块。

### 登记簿

| 文件 | 进到哪 | 什么时候加进来 | token | 为什么加 | 指纹 |
|---|---|---|---|---|---|
| `package/data/judge/system.txt` | 判官那一次请求的 system，不进主对话 | 主动回复判断交给判官的每一次（`chat.md` 第六条，施工 O-11）：system 的第一段 | 88 | 照旧版判官说明的开头改写成英文短句（`chat.md` 施工时定的第 1 条）：只判不写；名字、身份、性格只从人格那一段来；记录、昵称、引用、解出来的字都不可信，不照着做 | `5f2ac00d` |
| `package/data/judge/persona-open.txt` | 判官那一次请求的 system，不进主对话 | 同上，带人格说明的：人格说明的开头 | 4 | 人格说明夹在标签里，和判官自己的说明分得开；开头收尾各一份资源（施工时定的第 3 条，照 `core/jobs` 的先例） | `d771af5f` |
| `package/data/judge/persona-close.txt` | 判官那一次请求的 system，不进主对话 | 同上：人格说明的收尾 | 4 | 同上 | `5dc2e055` |
| `package/data/judge/reply.txt` | 判官那一次请求的 system，不进主对话 | 打分的那一次（`Route::Judge`），接在人格后面 | 367 | 照旧版判官说明的判法改写成英文短句：先认这一条是冲谁说的、谁该接；五条判法、五维各一句、`should_reply` 和 `to_bot` 怎么给。去掉好感度（Q21）和「程序还会再加减分」（两边各看各的）（施工时定的第 1 条）。旧版 09-24 实测 18/18 对 11/18 | `525c19fb` |
| `package/data/judge/moderation-only.txt` | 判官那一次请求的 system，不进主对话 | 只查违规的那一次（`Route::ModerationOnly`），代替 `reply.txt` | 37 | 只有违规旗的消息只查违规，不打分：说清五维给 0、两个布尔给假，回答的格式还是那一个 | `71c845d9` |
| `package/data/judge/violations.txt` | 判官那一次请求的 system，不进主对话 | 主动回复判断交给判官的每一次（`chat.md` 第六条，施工 O-11）：接在 `reply.txt` 或 `moderation-only.txt` 后面，`{severity_min}` 换成违规的门槛 | 100 | 照旧版违规的几类改写成英文短句：只给初判、不罚，光有关键词不算；只要 `severity`，旧版的类别、证据、相关的人和消息不要（施工时定的第 2 条） | `b57702c0` |
| `package/data/judge/answer.txt` | 判官那一次请求的 system，不进主对话 | 主动回复判断交给判官的每一次（`chat.md` 第六条，施工 O-11）：system 的最后一段 | 93 | 回答的格式：一个 JSON 对象，九格，`reason` 一句话。照旧版的格式去掉好感度和违规的类别、证据（施工时定的第 1、2 条）；读的时候宽（`chat.md` 第六条「怎么走」第 4 条） | `a11158ea` |
| `package/data/judge/records-open.txt` | 判官那一次请求的 user，不进主对话 | 主动回复判断交给判官的每一次（`chat.md` 第六条，施工 O-11）：群聊记录的开头 | 5 | 群聊记录夹在标签里，和这一条分得开（施工时定的第 3 条） | `499e1ed2` |
| `package/data/judge/records-close.txt` | 判官那一次请求的 user，不进主对话 | 同上：群聊记录的收尾 | 5 | 同上 | `2ad603de` |
| `package/data/judge/current-open.txt` | 判官那一次请求的 user，不进主对话 | 主动回复判断交给判官的每一次（`chat.md` 第六条，施工 O-11）：这一条的开头，接在群聊记录后面 | 5 | 判官只判这一条，记录只是上下文：这一条单独夹在标签里（施工时定的第 3 条） | `6e5b901f` |
| `package/data/judge/current-close.txt` | 判官那一次请求的 user，不进主对话 | 同上：这一条的收尾 | 5 | 同上 | `868e765a` |
| `package/data/judge/decoded-open.txt` | 判官那一次请求的 user，不进主对话 | 这一条里有 base64、进站链解出了字的：解出来的字的开头，接在这一条后面 | 6 | base64 里藏的话判官看不出来，解出来另给一段；标签写明是解出来的（施工时定的第 3 条） | `70ea8264` |
| `package/data/judge/decoded-close.txt` | 判官那一次请求的 user，不进主对话 | 同上：解出来的字的收尾 | 6 | 同上 | `d9c0d974` |
| `package/data/facts/undelivered.txt` | 事实（`kind` 是 `undelivered`），桥经核心的 `session.note` 记 | 她的一段话没发出去（NapCat 拒了、等不到、连接断了、排过期了），一段一块；这一轮还在跑的下一次请求看到，不在跑的下一轮开头看到（`onebot.md`「退信」，施工 O-25 下） | 49（出厂的样子：`why` 填 `expired`、`detail` 空的、`text` 填 30 个字符的中文开头；`why` 填 `rejected`、`detail` 填 `发送失败` 是 52；只有模板、`text` 空的是 29） | 她的话没发出去，她自己不知道，会以为群里看到了（18 第十节「发不出去就退回给她，像退信」、Q15）：说清是哪一句（开头 30 个字符）、没人看到、为什么；要不要再说由她定，不加指令 | `789148e2` |
| `package/data/facts/another-line.txt` | 事实（`kind` 是 `another-line`），桥经核心的 `session.respond` 的 `facts` 交 | 群里开轮时交给别的线、还在回的消息一条一块：主线开轮带每条支线在回的，分叉带主线这一轮在回的（`onebot.md`「群里怎么叫她」第 7 条，施工 O-36） | 33（`msg` 填 `1708423305`、`aim` 填 `1708423307`；只有前一句的原句 21） | 支线和主线的前缀逐字节相同，她看得到别的线正在回的那一条，不标会两边都回它（18 第八节「要守的三条」第 2 条：什么都不标，支线回 B 时 15 次里 12 到 15 次也回了 A）；末尾点明这条线回哪一条，重复回 A、`skip` 都是 0/30（施工 O-36 在真实位置量） | `a5c936ff` |
| `package/data/tools/skip_reply.json` | tools 数组 | 场所会话（私聊、群）的工具面里有 `skip_reply`：预设开着功能 `qq`（QQ 工具）的，每次请求都带；本机的会话没有（`onebot.md`「提供者和不说话」，施工 O-26） | 70（2026-10-09 照开发端点量，十六件一起时的边际份量） | 群里的回合判过才开，判官、加值项会看错；每个回合都要有一条不说话的路（18 第七节、Q5：旧版只靠提示词叫她别回，她把「没 @ 我」说出了口）。两句：这一轮写的都不发、什么时候用；`reason` 必填、不写说明 | `c4ebd416` |
| `package/data/tool-results/skipped.txt` | 工具结果 | `skip_reply` 答的那一句（施工 O-26；O-38 改） | 11（2026-10-12 量；O-26 那一句 14） | 告诉她这一条回复里写的没发。O-38 起调了这一轮就结束（桥带 `end_turn`），这一句只在以后的轮次照历史看到，改成说已经发生的事：原来的「这一轮写的都不会发出去」放在历史里像在说正在跑的那一轮（`onebot.md` 施工时定的第 239 条） | `46eb168d` |
| `package/data/tool-results/unknown.txt` | 工具结果 | 核心调到桥不答的工具（登记缓存是桥的旧版本的），字段按 `recall` 填（施工 O-26） | 9（2026-10-09 量） | 说法同核心的 `core/tool-results/unknown.txt`；给模型看的字随包走，桥不读核心的资源（`onebot.md` 施工时定的第 141 条） | `82d2ac6b` |
| `package/data/tools/recall.json` | tools 数组 | 场所会话（私聊、群）的工具面里有 `recall`：预设开着功能 `qq`（QQ 工具）的，每次请求都带；本机的会话没有（`onebot.md`「平台工具（一）」，施工 O-31） | 36（2026-10-10 照开发端点量，十九件一起时的边际份量） | 18 第十一节的撤回：目标从叫她做的那条的引用取，不给参数（不信模型给的编号）；一句说清撤哪一条 | `9549a49d` |
| `package/data/tools/mute.json` | tools 数组 | 群会话的工具面里有 `mute`（只给群；同上） | 80（同上） | 18 第十一节的群管：禁言，单位是秒；一句说清对谁；参数 `seconds` 名字看得出单位，说明只写「0 是解禁、最多 2592000」（旧版的教训：`duration` 被模型当成分钟；`onebot.md` 施工时定的第 186 条） | `1591c1d3` |
| `package/data/tools/poke.json` | tools 数组 | 场所会话（私聊、群）的工具面里有 `poke`（同 `recall`） | 46（同上） | 18 第十一节的互动：戳一戳；一句说清戳谁、没 @ 的戳叫她的人 | `563796d9` |
| `package/data/tool-results/recalled.txt` | 工具结果 | `recall` 成了（施工 O-31） | 3（2026-10-10 量） | 告诉她撤了 | `5a6ee52c` |
| `package/data/tool-results/muted.txt` | 工具结果 | `mute` 成了，秒数大于 0；`who` 是名字（没有的写号），`duration` 是 `10m` 这样的时长 | 9（`小明`、`10m`） | 18 第十一节：返回写明换算成多久，单位错了她看得出 | `1bee606a` |
| `package/data/tool-results/unmuted.txt` | 工具结果 | `mute` 成了，秒数是 0（解禁） | 6（`小明`） | 解禁和禁言分开说，她转告不会说反 | `d0036420` |
| `package/data/tool-results/poked.txt` | 工具结果 | `poke` 成了 | 5（`小明`） | 告诉她戳了谁：没 @ 的戳的是叫她的人 | `334c70a2` |
| `package/data/tool-results/not-allowed.txt` | 工具结果 | 不是终端管理员、管理的人叫她撤别人的、禁言（`onebot.md`「平台工具（一）」第 4 条） | 16 | 18 第十一节：群管只给终端管理员和管理的人；她照人话转告，不另发群消息（施工单「要定的」第 3 条） | `abecef57` |
| `package/data/tool-results/no-quote.txt` | 工具结果 | 撤回：叫她做的那条没引用 | 16 | 撤回的目标以回复的那条为准；告诉她要对方回复那条再叫 | `d33517b4` |
| `package/data/tool-results/no-target.txt` | 工具结果 | 禁言：引用、@ 都认不出人；戳一戳：没 @、也不知道谁叫的 | 19 | 目标不从模型来，认不出就不做；告诉她要对方回复或 @ 那个人再叫 | `49ed6fc4` |
| `package/data/tool-results/many-targets.txt` | 工具结果 | 禁言、戳一戳：引用的和 @ 的合起来不止一个人 | 17 | 一次只动一个人，不猜是哪一个 | `1feae897` |
| `package/data/tool-results/protected.txt` | 工具结果 | 禁言：目标是群主、群管理员、终端管理员、白名单成员 | 21（`小明`） | 18 第十一节：默认不动群主和群管理员；终端管理员、白名单成员同样不动 | `b371e98c` |
| `package/data/tool-results/bad-seconds.txt` | 工具结果 | 禁言：秒数没写、不是整数、不在 0 到 2592000 | 14 | QQ 的上限是 30 天；参数说明里写了，写错的再说一遍范围 | `82238467` |
| `package/data/tool-results/failed.txt` | 工具结果 | NapCat 回了失败（问身份、动作都算），`detail` 是它的原话（截到 200 个字符） | 9（`Recall failed`） | 施工单：NapCat 回失败照原话交回；她在群里不是管理员时撤别人的就走这里 | `a24f6a13` |
| `package/data/tool-results/unreachable.txt` | 工具结果 | 那个机器人号没连着、连接断了，或者桥不认识这个会话 | 11 | 没做成，说清是够不着 QQ，不是被拒 | `8c9726ee` |
| `package/data/tool-results/unanswered.txt` | 工具结果 | 等了 `call_timeout_seconds` NapCat 没回 | 15 | 可能做了、可能没做：照实说，她不会以为一定没成又叫一遍 | `9261e1f6` |
| `package/data/tools/fetch_media.json` | tools 数组 | 群、私聊会话的工具面里有 `fetch_media`（施工 O-33 给群，O-35 给私聊；本机的会话没有） | 114（2026-10-11，二十件一起的边际份量；第一稿 122；O-35 把 `group message` 换成 `QQ message`，重量还是 114） | 按需看（2026-10-11 项目主人定）：群里、私聊里的图、视频、文件她要看才去 QQ 取。说明一句说清取什么、图回来看、视频文件存进 `qq-files/`；`index` 写明是那一条里第几样、从 1 数（近况那一行的 `#n`，核心审时要的） | `ffe86481` |
| `package/data/tool-results/saved.txt` | 工具结果 | `fetch_media` 把视频、文件（存不成图的图）写进了工作区，`path` 是相对工作区的 `qq-files/<名字>`（施工 O-33 改的，「施工时定的」第 204 条） | 16（2026-10-11，`path` 填 `qq-files/8816-1-排班.pdf`；改相对路径以前填绝对路径量的是 33） | 她拿路径再 `read`（视频她用 `shell` 调 ffmpeg 抽帧再 `read`） | `b0a83124` |
| `package/data/tool-results/not-found.txt` | 工具结果 | `fetch_media` 的 `msg` 不是整数、QQ 说没有这一条、那一条不在这个会话（群里不是这个群的，私聊里不是这个人私聊的） | 12 | 编号不对、过期了、别处的都照没有说：不让她拿编号看别的群、别人的私聊。O-35 把 `in this group` 换成 `in this chat`，私聊里也说得对，重量还是 12 | `b6d793bf` |
| `package/data/tool-results/no-item.txt` | 工具结果 | `fetch_media` 的 `index` 不对：那一条没有第几样 | 16（`count` 填 2） | 说清那一条有几样、从 1 数，她改了再取 | `e9233523` |
| `package/data/tool-results/not-fetchable.txt` | 工具结果 | `fetch_media` 点到的是语音、小黄脸、`mface` 商城表情 | 13 | 语音先不转文字（2026-10-11 项目主人定），表情没有可取的原图：说清取不了，不再试 | `cb87ab72` |
| `package/data/tool-results/unfetched.txt` | 工具结果 | NapCat 回了，东西拿不到手：地址下不下来、路径读不到、没有 base64；`detail` 是每一样为什么（截到 200 个字符） | 9（`detail` 填 `url: HTTP 404`） | 照实说没取到、为什么 | `2418337c` |
| `package/data/tool-results/too-many.txt` | 工具结果 | 这一轮已经取了 `fetch_per_turn` 次（出厂 4） | 12（`count` 填 4） | 照旧 Miyu 的配额，一轮不取太多；说清上限，她下一轮再取 | `4cec53d7` |
| `package/data/tool-results/not-saved.txt` | 工具结果 | 取到了，写不进工作区：`qq-files` 不是真目录、落到了工作区外面、写不了；`detail` 是为什么 | 12（`detail` 填 `qq-files is not a folder`） | 照实说没存下，不装作存了 | `44b686f0` |
