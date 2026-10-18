## 施工单 I-2：驱动的接口、拆 crate

状态：施工中。

### 目的

设计 01 第二节、第三节、第一节第 26 条（项目主人 2026-10-19 定拆法 A）：宿主只经驱动的接口碰平台；代码拆成 `miyu-im`（宿主，和平台无关）和 `miyu-im-onebot`（OneBot 驱动）两个 crate，编进同一个程序，宿主不依赖驱动。程序名、命令名、包名、配置键不改（I-4）。

### 现在的样子（摸过的）

- 宿主（`core/`、`rules/`，约两万一千行）碰 OneBot 的地方有两类：`crate::onebot` 的类型和辅助函数（50 个文件），`listen::bots::Bots`（连着的号）。
- 真正发调用的只有 18 处，都是一个写法：`link.calls.call(&link.out, 动作, 参数)`，出站队列和取东西另用 `begin` 再等（照先后进写队列）。直接写 OneBot 动作名的 4 处（`cards.rs` 两处、`titled.rs`、`lookup.rs`）。
- 驱动那一头：`onebot/`（约一千四百行：认帧、认段、拼动作）、`listen/`（约五百行：反向 WebSocket、连着的号）、`media.rs`（约五百行：照 OneBot 取东西）。

### 怎么走（分三小步，每一步测试都全过）

1. **I-2 上（一）数据挪到宿主**：和平台无关、只是放在 `onebot` 里的类型挪到宿主新的 `chat` 模块：发给谁 `To`、前面带的引用和 @ `Lead`、一条消息 `Posted`、撤回 `Recall`、交给宿主的事 `Event`；认出来的段 `Segments`、`Piece`、`Media`、`MediaKind`、`Fetch`、`Forwarded`；群里的身份 `Rank` 和名字、身份的缓存 `Members`；东西在哪 `Source`；转发里的一条 `Node`；一次调用没成 `CallError`。认帧、认段、拼动作、读回应的函数留在驱动。驱动的模块照旧 `pub use` 它们，宿主改从 `crate::chat` 拿。纯搬家，编译器查。
2. **I-2 上（二）驱动的接口**：宿主只认两样。
   - `Link`（一个机器人号的一条连接）：`call(调用) → 回应`、`begin(调用) → 等着的一次`（照先后进写队列，出站队列要它）；`Call` 是驱动拼好的一次调用，宿主不看里面。
   - `Driver`：平台名、能力表；把宿主要做的事（发一段字、发图、长文的一页、合并转发、传文件、撤回、禁言、戳、贴表情、同意加好友、取一条消息、取东西、取转发、群成员、一个成员、群列表、好友列表、群资料、原样的动作）拼成 `Call`；把回应读成宿主认的结果（发出去那条的编号、成员、名字和身份、东西在哪、转发的节点、失败时说的为什么）。
   - 那 18 处和 4 处动作名改成经 `Driver`、`Link`；`Bots` 存 `Arc<dyn Link>`。OneBot 先在原地实现这两样。
3. **I-2 下：拆 crate**：`onebot/`、`listen/`、`media.rs` 和 OneBot 的 `Driver`、`Link` 搬进 `crates/miyu-im-onebot`；`crates/miyu-onebot` 改名 `crates/miyu-im`（库名 `miyu_im`，程序名照旧 `miyu-onebot` 到 I-4）；程序的 `main` 把 OneBot 驱动交给宿主。门禁加一条：宿主的 crate 不依赖驱动的 crate，宿主的源码里不出现 OneBot 的动作名、`napcat`。

### 施工时定的

- 接口用 trait 对象（`Arc<dyn Driver>`、`Arc<dyn Link>`），异步的方法交回装箱的 future：只有一个驱动，泛型参数会一路传到路由的每一层，改动面大、换不来什么。
- 类型名照旧（`To`、`Posted` 这些），只换地方：改名会让这一步的改动翻倍，名字以后要动另起一步。
- 号照旧是 `i64`：QQ、Telegram 的号都是整数；真接到字符串号的平台时再改。

### 不做什么

- 不改名（包、命令、配置键、程序名、仓库：I-4）；不分配置（I-3）；不改行为：每一步的端到端测试照旧全过。

### 验收

本机门禁、变异；不用真 QQ 验（行为不变）。
