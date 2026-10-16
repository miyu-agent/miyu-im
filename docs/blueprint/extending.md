## 给接入QQ 写扩展包

状态：图纸，随施工 O-42 起草（2026-10-17）。项目主人 2026-10-17 定：接入QQ 的新功能以后能做成单独的包装，例如 `miyu pkg install miyu-onebot-qq-admin`。扩展口分三步：给别的包调的方法（O-42，第一条）、包之间的依赖（核心做，做完补进第一条的清单）、插进桥里面（进站、出站、QQ 事件、判断打分，O-43，第二条）。

### 是什么

写给扩展包作者的约定：一个扩展包（核心的 `packages.md`，有自己的程序）怎么让 QQ 做事、怎么知道一个会话是哪个群。只写对外的样子；桥这一边怎么做在 `onebot.md` 第一条「给别的包调的方法」。

### 一、调接入QQ 的方法

**清单**：

```toml
[process]
capabilities = ["packages.call"]   # 调别的包的方法，开的时候人批（核心 X-18）
```

经核心的 `package.call {"package": "onebot", "method", "params"}` 调；核心转给接入QQ，回什么交回什么（核心的 `protocol.md`「`package.call`」）。没批 `packages.call` 的核心回 `needs_approval`；接入QQ 没在跑回 `program_not_running`；接入QQ 拒了的回 `method_failed`，`data.message` 是下面写的原因。

**`qq.action`**：让 QQ 做一件事，例如踢人、禁言、查群成员、发消息。

| 参数 | 写法 |
|---|---|
| `action` | OneBot 动作名（NapCat 认的那些）：小写字母、数字、`_`、`.`，1 到 64 个 |
| `params` | 动作的参数，对象；不写是 `{}` |
| `bot` | 经哪个机器人号；不写是连着的号里最小的那一个 |

回答是 NapCat 的回答原样：`{"status": "ok" | "failed", "retcode", "data", "message", "wording"}`。NapCat 说没成也是回答，看 `status`、`retcode`。拒绝：`bad_params`（写错）、`not_connected`（那个号没连着）、`unanswered`（NapCat 没回）。

用 `qq.action` 发的消息，AI 不知道是自己说的：不过接入QQ 的出站链（清理、去重）、不记进会话。要 AI 知道的，等第二条的出站挂接点。

**`qq.venue`**：`{"session": <会话编号>}` → `{"venue": "qq:group:<群号>" | "qq:private:<号>", "bot": <机器人号>}`；接入QQ 认不得的会话是 `{"venue": null}`（还没来过消息的群、回完封存了的支线）。你提供的工具被调时，核心的 `tool.call` 交来的是会话编号，用它查是哪个群。

### 二、插进接入QQ 里面

随施工 O-43：接入QQ 照软件包列表找导出了约定方法名的包（`qq.inbound`、`qq.outbound`、`qq.event`、`qq.score`），在那一处调它。要等核心的软件包列表带上每个包导出的方法名、包之间的依赖做完。

### 起草时定的

| # | 定了什么 | 为什么 | 没选 |
|---|---|---|---|
| 1 | `qq.action` 不挑动作 | 人批 `packages.call` 的时候就是在批这个包能让 QQ 做事；列一张能调的动作表要跟着 NapCat 改，挡不住真想做事的包 | 只开一张动作表 |
| 2 | 方法名带 `qq.` | 和第二条别的包导出、接入QQ 来调的约定方法放在同一个名字空间，作者一眼看出是接入QQ 的约定 | 不带前缀 |
| 3 | `qq.venue` 照接入QQ 手里的表答 | 你的工具被调的那个会话一定是接入QQ 认得的场所会话；去问核心，别的包又没有系统账号 | 请核心给别的包开 `venue.sessions` |
