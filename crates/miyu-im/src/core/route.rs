//! 消息进来、回话出去（`onebot.md` 第一条「怎么走」第 6 到 10 条，「群消息」「撤回」「好友请求」）：一个任务拿着跟核心的连接，
//! 一边收 NapCat 那头读出来的消息、撤回和请求，一边收核心推来的事件。一件件照先后办（套场所规则、找会话、发进去），办的时候来的推送由
//! [`Core`] 留着、办完再看。
//!
//! - 私聊（这里）：场所、平台上的人经群聊内核拼（`onebot::venue`、`onebot::person`），拼不出来的（照说不会）记一行、这条
//!   不送；带上场所的格（施工 O-22，`fields`），`show_ids` 照这个私聊套出来的场所规则（`applied`）。白名单成员的私聊照场所
//!   规则带人格、预设、工作区（施工 O-27，同群）。
//! - 群消息（`group`，施工 O-22）：一律旁听；正文里的 @ 写成名字（`names`）；场所规则管人格、预设、工作区、谁是管理的人、
//!   睡没睡、看不看得到号。撤回（`recall`，施工 O-22）记 `venue.recalled`。
//! - 群里叫她（施工 O-23，「群里怎么叫她」）：记下以后判（`called`：纯逻辑的判断在 `decide`，线路规程在 `discipline`，
//!   判断的样子在 `body`），该回的开一轮；群会话推来的事件收进投影（`projection`），她新说的话过出站链（`outbound`，施工
//!   O-25 上）发回群里（`speak`）。要问判官的（O-23 下）交给 `judges`，任务（`ask`）经并着发的调用口
//!   问，跟核心的那一头接着办别的；判官回来了照号收回来、算分、记判断（`judged`）。判官带这个群会话所用的人格的说明（O-23
//!   补，`persona`）。
//! - 起来就订阅（施工 O-32，`revive`）：每次连上核心、握手以后先经 `venue.sessions` 列出名下的场所会话和终端管理员的私聊，群的
//!   从头订阅，白名单成员的、终端管理员的私聊也从头订阅一次；补来的（`backlog`）认出发到哪，期限以内、没入队的她的话补发。
//! - 找会话（`session`）：「场所 → 会话编号」只记在内存里，每个场所桥起来以后第一次要用时问一次 `venue.session`（起来时订阅了的
//!   群不再问）；私聊的问到了订阅（不写 `after`），群的从头订阅（施工 O-23）。会话不在了（`session_not_found`、`session_stopped`）
//!   忘掉，再问一次、再交一次，只重来一次。私聊不是终端管理员、也不在白名单里的（`no_system_account`，或者会话的属主是桥
//!   自己，「施工时定的」第 49 条，施工 O-27）、群的规则写错的不接，同一个场所只记一行运行日志。
//! - 好友请求（施工 O-27，`request`）：白名单成员的另起任务同意，别的放着；群邀请只记一行。
//! - `/` 开头的先当斜杠命令交 `command.run`（`command`，O-19，第 7、8 条之间的「斜杠命令」），核心说不是命令的才照普通的话发；
//!   回执交 `receipt`，群里的过几秒撤回（施工 O-25 上）。
//! - 她在私聊里的回话：`message.assistant` 的文字块接起来，过出站链、拆段（`speak`，施工 O-25 上）。
//! - 出站队列（施工 O-25 中，「出站队列」）：她的话、提示、回执都先入队（记 `ext.im.venues.queued`）再照先后交给收进这个会话
//!   的那个机器人号现在的连接，等回应的那一步交给别的任务，结局照放进写队列的先后交回来记（`sending`；排着、过期的纯逻辑在
//!   `queue`）。她被禁言、解禁的通知记下（`muted`），禁言时这个群的出站排着；号连上了、定时醒了再看一遍排着的。她的话没发出去的
//!   经核心的 `session.note` 退信（施工 O-25 下，「退信」，在 `sending`）。
//! - 贴表情（施工 O-25 下，「贴表情」）：判下来要回、主触发是冲她来或续聊的，在她要回的那一条上贴，那一轮发出去第一段、结束了、
//!   到时候了摘（`reaction`）。
//! - 不说话（施工 O-26，「提供者和不说话」）：她的回复里有 `skip_reply` 的调用块，这一轮的字都不发（`quiet` 认，`speak` 不发）。
//! - 平台工具（一）（施工 O-31，「平台工具（一）」）：读的一头交来的撤回、禁言、戳一戳的 `tool.call`，做不做、对谁做照 `platform`
//!   定，`acting` 收投影、调 NapCat、答核心。
//! - 支线（施工 O-36，「群里怎么叫她」第 6、7 条）：判下来要回的照群聊内核分派（`lanes`），主线忙的分叉一条支线回；支线推来的
//!   事件收进主线的投影（`projection` 的 `lanes`），她在支线说的话照主线发。私聊在 `private`。
//! - 核心推来的 `extension.config`（施工 O-20）：`keys` 原样交给 `serve`，它换上手里的配置、端口变了另起一个任务换（`running::rebind`）
//!   （「施工时定的」第 45 条）：这里够不着监听。白名单成员 `im.whitelist` 这里记一份（施工 O-23）；换了的，私聊找过的
//!   会话都忘掉，下一条照新的白名单再找（施工 O-27）。

mod acting;
mod aims;
mod applied;
mod ask;
mod asked;
mod backfill;
mod backlog;
mod bindings;
mod body;
mod called;
mod cards;
mod command;
mod decide;
mod deferred;
mod discipline;
mod driven;
mod events;
mod exports;
mod fetching;
mod fields;
mod forwards;
mod group;
mod impostor;
mod judged;
mod judges;
mod lanes;
mod long;
mod message;
mod migrate;
mod muted;
mod names;
mod noted;
mod notes;
mod outbound;
mod outreach;
mod overview;
mod packages;
mod pages;
mod parts;
mod persona;
mod pictures;
mod policy;
mod primer;
mod private;
mod projection;
mod pushed;
mod queue;
mod quiet;
mod reaction;
mod recall;
mod receipt;
mod request;
mod revive;
mod sending;
mod session;
mod shown;
mod speak;
mod steering;
mod stickers;
mod strangers;
#[cfg(test)]
mod tests;
mod titled;
mod voicing;
mod waiting;

use message::{Message, reply_text};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use futures_util::stream::FuturesOrdered;
use miyu_chat::Venue;
use miyu_client::ids::VenueId;
use serde_json::{Map, Value};
use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};

use crate::TARGET;
use crate::changed::Changed;
use crate::chat::{Bots, Platform};
use crate::chat::{Event, Members, To};
use crate::core::hooks::Hooks;
use crate::core::{Core, Gone, reason};
use crate::counters::{Count, Counters};
use crate::plugins::Toggles;
use crate::recent::Recent;
use crate::rules::{Applied, Tools, Venues};
use crate::serve::Failure;
use crate::settings::{recall_receipts, recall_receipts_key, whitelist, whitelist_key};
use aims::Aims;
pub(crate) use ask::Slots;
use backlog::Backlog;
use bindings::Bindings;
use deferred::Deferred;
pub(crate) use fetching::Fetching;
pub(crate) use forwards::Forwarding;
use impostor::Noted;
use judges::Judges;
use outbound::Spoken;
use parts::sent;
pub(crate) use parts::{Asking, Recalling};
pub(crate) use persona::Personas;
use projection::Projection;
use queue::Queue;
use quiet::Quiet;
pub(crate) use reaction::Reactions;
use sending::{Answered, Item};
use steering::Steering;
use titled::Titles;
use voicing::Voicing;

/// 一个场所会话的回执、回话发到哪：收进它的机器人号、私聊的对方或者群；场所只拿来记运行日志。
#[derive(Debug, Clone)]
struct Peer {
    bot: i64,
    to: To,
    venue: VenueId,
}

/// 拿着跟核心的连接的那一个。
pub(crate) struct Route {
    core: Core,
    /// 今天的几个数（施工 I-6 下，`crate::counters`）。
    counters: Arc<Counters>,
    /// 卡片、白名单变了叫一声（施工 I-7 上：im 的会话页照它再问）。
    cards_changed: Arc<Changed>,
    /// 最近判断（施工 I-7 上补，`crate::recent`）。
    recent: Arc<Recent>,
    /// 平台：拼场所、平台上的人、命令编号（施工 I-2 下：驱动给的名字）。
    platform: Platform,
    bots: Arc<Bots>,
    /// 场所规则和出厂数据（施工 O-21）：每一条消息照它套场所（施工 O-22）。
    rules: Venues,
    /// 群成员的名字（施工 O-22，「群消息」第 5 条）。
    members: Members,
    /// 场所 → 会话编号（第 7 条）。键是场所编号的原文：内核的 `VenueId` 不能做散列表的键。
    venues: HashMap<String, String>,
    /// 会话编号 → 回执、回话发到哪（第 10 条）。
    peers: HashMap<String, Peer>,
    /// 记过一行「不接」的场所（第 7 条、「群消息」第 3 条）：私聊不是终端管理员、也不在白名单里的，群的规则写错的。
    refused: HashSet<String>,
    /// 群会话编号 → 这个群的投影（施工 O-23，「群里怎么叫她」第 1、2 条）：订阅了的群才有。
    groups: HashMap<String, Projection>,
    /// 白名单成员的平台身份（`im.whitelist`，施工 O-23）：握手交来的，推来新的就换。
    whitelist: Vec<String>,
    /// 出站排着的（施工 O-25 中，「出站队列」）：门关着（她被禁言、号没连着）的照会话排着，过了期作废。
    waiting: Queue<Item>,
    /// 每个会话最近退过信的那一句（线、回合；I-26）：她一句话拆成的几段，后面接着没成的不再退（`sending/bounce.rs`）。
    bounced: HashMap<String, (String, u64)>,
    /// 在等 NapCat 回应的一段段（她的话、提示、回执）：一段一个任务，结果照放进写队列的先后交出来（NapCat 并着办动作，回的
    /// 先后不一定照发的先后；`venue.delivered` 要照发的先后记，核心照日志的先后画她说的话，「施工时定的」第 84 条），由这一头记
    /// 结局（施工 O-25 中，`sending`）。放下 `Route` 时不掐：任务只等回应，连接断了、到了时限自己就完。
    sending: FuturesOrdered<JoinHandle<Answered>>,
    /// 私聊会话编号 → 桥这一轮自己入队了的（施工 O-25 上，`outbound`：私聊的出站链照它去重；O-25 中改成入队时记）。
    spoken: HashMap<String, Spoken>,
    /// 另起的平台动作：群里的命令回执 NapCat 回了编号以后等几秒撤回（施工 O-25 上，`receipt`），贴了表情等着摘（施工 O-25 下，
    /// `reaction`）。不放进 [`Route::sending`]：照先后交的那一串会被它们卡住。
    chores: JoinSet<()>,
    /// 驱动自己的状态目录（施工 Q-2，`toolset::Call::store`）。
    store: PathBuf,
    /// 驱动的工具交回的任务（施工 T-4，`driven`）：办完先做交回的那件事、再答。
    jobs: JoinSet<driven::Finished>,
    /// 群里回执要撤回的命令（施工 O-47，`im.recall_receipts`；推来的换）。
    recalled: Vec<String>,
    /// `im.recall_receipts` 的默认值：推来 `null` 的回到它。
    recall_default: Vec<String>,
    /// 群里的命令回执发出去几秒后撤回（`bridge.json` 的 `receipt_recall_seconds`）。
    recall: Duration,
    /// 在判的和问判官的任务（施工 O-23 下）。
    judges: Judges,
    /// 贴着的表情（施工 O-25 下，`reaction`）。
    reactions: Reactions,
    /// 这一轮不说话了的（施工 O-26，`quiet`）：群里、私聊的都在这一张表里。
    quiet: Quiet,
    /// 桥的工具（施工 O-31）：平台工具答的话。
    tools: Arc<Tools>,
    /// 问到的「是不是终端管理员」（施工 O-31，`bindings`）。
    bindings: Bindings,
    /// 问过的群名、放进 system 的那一段（施工 O-49，`titled`）。
    titles: Titles,
    /// 记过的冒充提醒（施工 O-53，`impostor`）。
    impostors: Noted,
    /// 她指定下一句 @ 谁、引用哪条（施工 O-55、I-31，`aims`）。
    aims: Aims,
    /// 禁言时判了要回的，按群会话排着（施工 O-50，`deferred`）。
    deferred: HashMap<String, Vec<Deferred>>,
    /// 禁言以后多久以内来的排着（施工 O-50）。
    mute_window: Duration,
    /// 解禁以后最多回几条排着的（施工 O-50）。
    mute_replies: usize,
    /// 私聊会话编号 → 推来的最近一条人说的话引用的平台编号（没引用的是空的，施工 O-31，`acting`：私聊里叫她做的那条）。
    quotes: HashMap<String, Option<String>>,
    /// 会话编号 → 订阅补来的那一段（施工 O-32，`backlog`）：只在订阅到补完的这一会儿有，补完了补发、拿掉。
    backlogs: HashMap<String, Backlog>,
    /// 支线都占满时排着的（施工 I-16，`waiting`）。
    queued: waiting::Waiting,
    /// 取东西的几个数（施工 O-33，`fetching`、`pictures`）。
    fetching: Fetching,
    /// 私聊会话编号 → 这一轮的回合编号（施工 O-35：照推来的事件记；T-3 起驱动的工具照它数一轮调了几次）。
    turns: HashMap<String, Option<u64>>,
    /// （会话编号，工具名）→ 那一轮的回合编号、调了几次（施工 T-3，`driven`）。
    uses: HashMap<(String, String), (Option<u64>, usize)>,
    /// 展开合并转发的几个数（施工 O-34，`forwards`）。
    forwarding: Forwarding,
    /// 推来的配置变化交给 `serve`（施工 O-20）。
    configured: mpsc::UnboundedSender<Map<String, Value>>,
    /// 挂接点（施工 O-43 上，`packages`）：起来时订阅软件包列表，推来的照换。
    hooks: Arc<Hooks>,
    /// 问过进站挂接点、照先后等着判的（施工 O-43 中，`steering`）。
    steering: FuturesOrdered<JoinHandle<Steering>>,
    /// 问过出站挂接点、照先后等着过链的她的话（施工 O-43 下，`voicing`）。
    voicing: FuturesOrdered<JoinHandle<Voicing>>,
    /// 场所卡片的两页在看的（施工 O-58 下，`pages`）。
    pages: pages::Pages,
    /// 陌生人的私聊开过的回合（施工 O-60，`strangers`）：限流照它数。
    strangers: strangers::Strangers,
}

impl Route {
    /// 拿着连接 `core`，回话照 `bots` 找连接，场所规则照 `rules`，群成员的名字记进 `members`，问判官照全局的名额 `slots` 排队（施工 O-23 下）、判官带的人格原文照 `personas` 记（施工 O-23 补），群里的命令回执
    /// `recall` 以后撤回（施工 O-25 上），出站排着的过了 `expire` 作废（施工 O-25 中），表情照 `reactions` 贴、摘（施工 O-25 下），
    /// 推来的配置变化交给 `configured`。白名单成员照握手交来的配置（`core.config`）。问到的「是不是终端管理员」记多久、群名隔多久再问照 `asking`（施工
    /// O-31）。取图、视频、文件照 `fetching`（施工 O-33），合并转发照 `forwarding` 展开（施工 O-34）。
    pub(crate) fn new(
        core: Core,
        (platform, bots): (Platform, Arc<Bots>),
        rules: Venues,
        members: Members,
        (slots, personas, recall, expire, reactions, asking, fetching, forwarding): (
            Slots,
            Personas,
            Recalling,
            Duration,
            Reactions,
            Asking,
            Fetching,
            Forwarding,
        ),
        configured: mpsc::UnboundedSender<Map<String, Value>>,
        (hooks, counters, cards_changed, recent, store): (
            Arc<Hooks>,
            Arc<Counters>,
            Arc<Changed>,
            Arc<Recent>,
            PathBuf,
        ),
    ) -> Route {
        let whitelist = whitelist(&core.config[whitelist_key()]);
        let Recalling {
            after: recall,
            default: recall_default,
        } = recall;
        let recalled = recall_receipts(&core.config[recall_receipts_key()], &recall_default);
        let judges = Judges::new(
            core.caller(),
            rules.judge_texts(),
            slots,
            (personas, Arc::clone(&hooks)),
        );
        let tools = rules.tools();
        hooks.set_toggles(Toggles::from_cards(rules.cards()));
        Route {
            core,
            store,
            counters,
            cards_changed,
            recent,
            platform,
            bots,
            rules,
            members,
            venues: HashMap::new(),
            peers: HashMap::new(),
            refused: HashSet::new(),
            groups: HashMap::new(),
            whitelist,
            waiting: Queue::new(expire),
            bounced: HashMap::new(),
            sending: FuturesOrdered::new(),
            spoken: HashMap::new(),
            chores: JoinSet::new(),
            jobs: JoinSet::new(),
            recall,
            recalled,
            recall_default,
            judges,
            reactions,
            quiet: Quiet::new(tools.silence()),
            tools,
            bindings: Bindings::new(asking.binding),
            titles: Titles::new(asking.group_info),
            impostors: Noted::default(),
            aims: Aims::default(),
            deferred: HashMap::new(),
            mute_window: asking.mute_window,
            mute_replies: asking.mute_replies,
            quotes: HashMap::new(),
            backlogs: HashMap::new(),
            queued: waiting::Waiting::default(),
            fetching,
            turns: HashMap::new(),
            uses: HashMap::new(),
            forwarding,
            configured,
            hooks,
            steering: FuturesOrdered::new(),
            voicing: FuturesOrdered::new(),
            pages: pages::Pages::default(),
            strangers: strangers::Strangers::default(),
        }
    }

    /// 先订阅名下的场所会话（施工 O-32，`revive`），再一直办，直到办不下去：核心关了管道、写不出去（核心请它退出、核心不在了，
    /// 第 11 条），交回空的，桥好好停下；发回话
    /// 的任务、问判官的任务（施工 O-23 下）、撤回执、贴表情的任务（施工 O-25 上、下）崩了，交回 [`Failure::Crashed`]（「施工时定的」
    /// 第 14 条），桥照它退出。`inbound` 关了就只看核心：桥在停、读 NapCat 的那一头都放下了。有东西排着的，另睡到最早的过期、
    /// 禁言到期的时刻，醒了再看一遍（施工 O-25 中，「出站队列」第 8 条）。
    pub(crate) async fn run(mut self, mut inbound: mpsc::Receiver<Event>) -> Option<Failure> {
        // 起来就订阅（施工 O-32，「群里怎么叫她」第 1 条）：NapCat 那头来的先排着，订阅完了再办。
        if self.move_defaults().await.is_err()
            || self.revive().await.is_err()
            || self.follow_packages().await.is_err()
            || self.reload_plugins().await.is_err()
        {
            return None;
        }
        let mut open = true;
        loop {
            let wake = self.wake();
            let handled = tokio::select! {
                event = inbound.recv(), if open => match event {
                    Some(event) => self.event(event).await.map_err(|Gone| None),
                    None => {
                        open = false;
                        Ok(())
                    }
                },
                pushed = self.core.next() => match pushed {
                    Some(pushed) => self.pushed(pushed).await.map_err(|Gone| None),
                    None => Err(None),
                },
                Some(joined) = self.sending.next(), if !self.sending.is_empty() => match sent(joined) {
                    Ok(answered) => self.answered(answered).await.map_err(|Gone| None),
                    Err(failure) => Err(Some(failure)),
                },
                Some(joined) = self.chores.join_next(), if !self.chores.is_empty() => {
                    sent(joined).map_err(Some)
                }
                Some(joined) = self.jobs.join_next(), if !self.jobs.is_empty() => {
                    sent(joined).map(|finished| self.finished(finished)).map_err(Some)
                }
                () = tokio::time::sleep(wake.unwrap_or_default()), if wake.is_some() => {
                    // 到点的禁言先补记解禁（施工 O-50），再看排着的。
                    match self.expire_mutes().await {
                        Ok(()) => self.pump().await.map_err(|Gone| None),
                        Err(Gone) => Err(None),
                    }
                }
                Some(joined) = self.voicing.next(), if !self.voicing.is_empty() => match joined {
                    Ok(voicing) => self.voiced(voicing).await.map_err(|Gone| None),
                    Err(error) => {
                        tracing::error!(target: TARGET, error = %error, "hook task crashed");
                        Err(Some(Failure::Crashed(error.to_string())))
                    }
                },
                Some(joined) = self.steering.next(), if !self.steering.is_empty() => match joined {
                    Ok(steering) => self.steered(steering).await.map_err(|Gone| None),
                    Err(error) => {
                        tracing::error!(target: TARGET, error = %error, "hook task crashed");
                        Err(Some(Failure::Crashed(error.to_string())))
                    }
                },
                Some(joined) = self.judges.running.join_next(), if !self.judges.running.is_empty() => match joined {
                    Ok(asked) => self.judged(asked).await.map_err(|Gone| None),
                    Err(error) => {
                        tracing::error!(target: TARGET, error = %error, "judge task crashed");
                        Err(Some(Failure::Crashed(error.to_string())))
                    }
                },
            };
            if let Err(ended) = handled {
                return ended;
            }
        }
    }

    /// 交一条消息：`/` 开头的先当斜杠命令交（「斜杠命令」），办完了的不再发；别的照 `session.send` 发（第 8 条，「群消息」
    /// 第 6 条），交回会话编号和回应（接受的、拒绝的都原样：群的照它判，施工 O-23）。要带图的（施工 O-33）、要展开转发的（施工
    /// O-34）先找到会话（不接的不去取），取了图带上、展开的转发写进场所的格；私聊没有字、没带东西、图又一张都没取到的不送。命令办完了的、
    /// 不接的、找不到会话的、不送的是空的。
    async fn submit(&mut self, mut message: Message) -> Result<Option<(String, Value)>, Gone> {
        if command::looks_like(&message.text) && self.command(&message).await? {
            return Ok(None);
        }
        if !self.expanded(&mut message).await? {
            return Ok(None);
        }
        let mut attachments = Vec::new();
        if let Some(pictures) = message.pictures.as_ref().filter(|pictures| pictures.any()) {
            if !self.found(&message.place).await? {
                return Ok(None);
            }
            let venue = message.place.peer.venue.to_string();
            let shown;
            (attachments, shown) = self.pictures(pictures, &venue).await?;
            // 私聊里交了原图的那几样记 `shown`（施工 O-35）；群里冲她来的这一步不记。
            if message.place.private_chat() {
                fields::shown(&mut message.fields, &shown);
            }
        }
        let bare = message.text.trim().is_empty() && message.fields.get("media").is_none();
        if message.place.private_chat() && bare && attachments.is_empty() {
            tracing::debug!(target: TARGET, venue = %message.place.peer.venue, message = message.number, "nothing to send");
            return Ok(None);
        }
        let sent = self.deliver(&message, session::SEND, &attachments).await?;
        let Some((session, reply)) = sent else {
            return Ok(None);
        };
        let (venue, number) = (&message.place.peer.venue, message.number);
        match reason(&reply) {
            None => {
                let chars = message.text.chars().count();
                tracing::info!(target: TARGET, venue = %venue, message = number, chars, "message sent in");
                self.counters.bump(Count::Received, &venue.to_string());
            }
            Some(other) => {
                tracing::warn!(target: TARGET, venue = %venue, message = number, reason = other, "message refused");
            }
        }
        Ok(Some((session, reply)))
    }

    /// 这一刻套到场所 `venue` 上的场所规则（「场所规则和出厂数据」第 3、5 条）。
    fn applied(&mut self, venue: &Venue) -> Applied {
        self.rules.current(Instant::now()).at(venue)
    }
}
