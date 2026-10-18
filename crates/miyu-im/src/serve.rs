//! `miyu-im serve`（`onebot.md` 第一条「怎么走」第 1、2、11 条）：在核心亲手给的管道上握手，再开监听，把几样接起来，跑到
//! 停的信号或者核心关了管道。
//!
//! 1. 在 [`Pipe`] 上握手（`core`）：程序里是标准输入输出，核心拉起桥时接好的（施工 O-18，`extensions.md`），不带凭据。等不到
//!    回应：[`Failure::NotSpawned`]；被拒、管道关了：[`Failure::Core`]。握手回了语言先告诉调的一方（之后说的都照它，端口被占
//!    那一句也是，施工 O-20，「施工时定的」第 42 条）；回应里的 `config` 里驱动的那几项（施工 I-3：`Driver::settings`）挑出来交给驱动
//!    （没交的照清单的默认值），别的照宿主的读。握手以后读一次系统的场所规则，问题记运行日志（施工 O-21，[`Venues`]），交给跟核心的那一头，每一条
//!    消息照它套场所（施工 O-22）。限流满了发进群里的提示照握手回的语言说（施工 O-23，「群里怎么叫她」第 7 条）；那种语言的
//!    字读不懂的照系统的语言（`main.rs` 那一头也是照原来的说，「施工时定的」第 80 条）。握手以后、开监听以前登记桥的工具
//!    （施工 O-26，`core/provider.rs`）：写出去就接着起来，等回应、记运行日志的那一段放进下面第 3 条的任务组。开好监听、桥手里
//!    的状态（`crate::running`）造好以后，把它交给答后台页方法的那一头、登记方法（施工 O-28 上，`core/methods.rs`）：同样写
//!    出去就接着起来。
//! 2. 叫驱动起来（[`Driver::start`]，施工 I-3）：OneBot 开好端口、说听上了；端口被占了：[`Failure::PortInUse`]，桥起不来。
//! 3. 驱动跑着的那一段（接连接、换端口）、跟核心的那一头一个任务（`core/route.rs`）、写状态文件的一个任务
//!    （`crate::status_file`，施工 O-18），都在一组里：停下时一起停。核心推来的配置（`extension.config`，施工 O-20）由跟核心的
//!    那一头交过来：换上桥手里的那一份（`crate::current`），驱动的那几项变了驱动自己换（施工 I-3）。
//! 4. 核心关了管道（标准输入读到头：核心请它退出，或者核心不在了）：好好停下，交回 `Ok`，退出码 0（第 11 条，「施工时定的」
//!    第 5、21 条：崩了由核心退避重启，桥里不另写一套重连）。跟核心的那一头、发回话的任务崩了：[`Failure::Crashed`]，退出
//!    （第 14 条）。NapCat 断了不退。

use std::sync::Arc;
use std::time::Instant;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Notify, mpsc};
use tokio::task::JoinSet;

use miyu_chat::Problem;
use miyu_client::places::DataRoot;

use crate::TARGET;
use crate::chat::Bots;
use crate::chat::Members;
use crate::core::hooks::Hooks;
use crate::core::methods::{Methods, register};
use crate::core::route::{
    Asking, Fetching, Forwarding, Personas, Reactions, Recalling, Route, Slots,
};
use crate::core::{Core, provide};
use crate::current::Current;
use crate::driver::{Driver, Gate, Reported};
use crate::rules::{Configured, Factory, Venues};
use crate::running::Running;
use crate::settings::Defaults;
use crate::status_file;
use crate::tuning::Tuning;

/// 跟核心说协议的管道：读的一头、写的一头（施工 O-18）。程序里是核心拉起桥时接好的标准输入输出；测试里是内存里的管道。
pub struct Pipe {
    /// 核心说的：一行一条。读到头是核心请它退出，或者核心不在了。
    pub read: Box<dyn AsyncRead + Send + Unpin>,
    /// 说给核心的：一行一条，只有协议。
    pub write: Box<dyn AsyncWrite + Send + Unpin>,
}

impl Pipe {
    /// 把读的一头 `read`、写的一头 `write` 接成一条管道。
    pub fn new(
        read: impl AsyncRead + Send + Unpin + 'static,
        write: impl AsyncWrite + Send + Unpin + 'static,
    ) -> Pipe {
        Pipe {
            read: Box::new(read),
            write: Box::new(write),
        }
    }
}

/// 起一个桥要的。
pub struct Serve {
    /// 数据根。
    pub root: DataRoot,
    /// 跟核心说协议的管道（施工 O-18）。握手的回应交来 NapCat 的端口、令牌（施工 O-20）。
    pub pipe: Pipe,
    /// 系统的语言：握手时报给核心，`ui.language` 是 `auto` 的照它定说话的语言。
    pub locale: Option<String>,
    /// 桥自己的数（`bridge.json`）。
    pub tuning: Tuning,
    /// 清单 `[settings]` 里 NapCat 端口的默认值：握手没交、推来 `null` 的照它（施工 O-20）。
    pub defaults: Defaults,
    /// 出厂的场所规则、出厂参数、违规词表：握手以前读好、查过（施工 O-21，[`Factory::load`]）。
    pub factory: Factory,
    /// 平台的接法（施工 I-2 下）：监听到的连接交给它。
    pub driver: Arc<dyn Driver>,
}

/// 说给人听的（「样子」）：怎么说照 `texts`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// 听上了：实际的端口（设的是 0 的，系统挑的那一个）。
    Listening {
        /// 实际听的端口。
        port: u16,
    },
    /// NapCat 连上了：机器人的号，连进来时没报的是空的。
    Connected {
        /// 机器人的号。
        bot: Option<i64>,
    },
    /// NapCat 断开了。
    Disconnected {
        /// 机器人的号：一直没认出来的是空的。
        bot: Option<i64>,
    },
    /// 令牌没设：NapCat 连进来会被拒，说怎么设。跟在 [`Notice::Listening`] 后面说。
    NoToken,
}

/// 起不来、跑着跑着停了的（「出错」，退出码都是 1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// 连不上核心、握手被拒、握手时管道关了：原因。
    Core(String),
    /// 握手等不到回应（`bridge.json` 的 `hello_seconds`）：不是核心拉起的，例如人在终端里跑的（施工 O-18）。
    NotSpawned,
    /// NapCat 的端口被占了。
    PortInUse(u16),
    /// 别的原因起不来（听不了、起不了运行时、读不了数据根或 `bridge.json`）：原话。
    Start(String),
    /// 跟核心的那一头崩了、发回话的任务崩了（是 bug）：原话。
    Crashed(String),
    /// 出厂的场所规则、出厂参数、违规词表有问题（是打包的错，施工 O-21）：每一条问题。
    Factory(Vec<Problem>),
}

/// 跑起来，直到 `stop` 到了（停的信号）或者核心关了管道。握手回了语言交给 `shaken`（之后说的话、起不来的原因都照它说），
/// 说给人听的交给 `tell`。
///
/// # Errors
///
/// 握手不成（等不到、被拒、管道关了）、端口被占、听不了；跑着跑着跟核心的那一头、发回话的任务崩了。
pub async fn run(
    serve: Serve,
    shaken: impl FnOnce(&str) + Send,
    tell: impl Fn(Notice) + Send + Sync + 'static,
    stop: impl Future<Output = ()>,
) -> Result<(), Failure> {
    let tools = serve.factory.tools();
    let methods = Arc::new(Methods::new());
    let core = Core::connect(
        serve.pipe,
        serve.locale.as_deref(),
        serve.tuning.hello(),
        Arc::clone(&tools),
        Arc::clone(&methods),
    )
    .await?;
    shaken(&core.language);
    // 登记桥的工具（施工 O-26）：写出去就接着起来，之后的 `venue.session` 造的会话就有它们。
    let provided = provide(&core.caller(), &tools).await;
    // 场所规则和出厂数据（施工 O-21）：读一次系统的，问题记进运行日志。跟核心的那一头每一条消息照它套场所（施工 O-22）。
    let venues = Venues::new(
        serve.factory,
        &serve.root,
        Configured::handed(&core.config),
        serve.tuning.rules_check(),
        Instant::now(),
    );
    // 撤回回执的命令的默认值（施工 O-47）：跟核心的那一头照它读握手交来的、推来的。
    let recall_default = serve.defaults.recall_receipts.clone();
    // 驱动的配置项挑出来交给驱动（施工 I-3），群的几项照旧。
    let current = Arc::new(Current::new(
        &core.config,
        (serve.driver.settings(), serve.defaults.driver),
        Configured::handed(&core.config),
    ));
    let reported = Arc::new(Reported::default());
    let tell: Arc<dyn Fn(Notice) + Send + Sync> = Arc::new(tell);
    let bots = Arc::new(Bots::default());
    let platform = serve.driver.platform();
    // 挂接点（施工 O-43 上）：读 NapCat 的那一头交通知、请求，跟核心的那一头订阅软件包列表、记谁挂在哪。
    let hooks = Arc::new(Hooks::new(core.caller(), serve.tuning.hook(), platform));
    let (inbound, received) = mpsc::channel(serve.tuning.inbound_queue);
    let changed = Arc::new(Notify::new());
    let gate = Arc::new(Gate::new(
        (
            Arc::clone(&current),
            Arc::clone(&reported),
            serve.tuning.clone(),
        ),
        Arc::clone(&bots),
        inbound,
        tell,
        Arc::clone(&changed),
        Arc::clone(&hooks),
    ));
    // 驱动起来（施工 I-3：OneBot 开端口、说听上了；被占了的桥起不来）。
    let accepting = serve.driver.start(Arc::clone(&gate)).await?;
    let running = Arc::new(Running {
        tuning: serve.tuning.clone(),
        bots: Arc::clone(&bots),
        platform,
        current: Arc::clone(&current),
        reported,
    });
    // 后台页调的方法（施工 O-28 上）：桥手里的状态齐了，先交给答的那一头，再登记。
    methods.ready(Arc::clone(&running));
    let registered = register(&core.caller()).await;
    // 跟核心的那一头停了，交回为什么（空的是核心关了管道、发回话的任务崩了是那个原因）；接连接的、写状态文件的停了不要紧。
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        accepting.await;
        None
    });
    if let Some(provided) = provided {
        tasks.spawn(async move {
            provided.await;
            None
        });
    }
    if let Some(registered) = registered {
        tasks.spawn(async move {
            registered.await;
            None
        });
    }
    let (configured, mut configs) = mpsc::unbounded_channel();
    let members = Members::new(serve.tuning.member_names());
    let slots = Slots::new(serve.tuning.judge_concurrency, serve.tuning.judge_queue());
    let personas = Personas::new(serve.tuning.judge_persona());
    let recall = Recalling {
        after: serve.tuning.receipt_recall(),
        default: recall_default,
    };
    let reactions = Reactions::new(serve.tuning.reaction_emoji.clone(), serve.tuning.reaction());
    let parts = (
        slots,
        personas,
        recall,
        serve.tuning.queue_expire(),
        reactions,
        Asking {
            binding: serve.tuning.binding(),
            group_info: serve.tuning.group_info(),
            mute_window: serve.tuning.mute_window(),
            mute_replies: serve.tuning.mute_replies,
        },
        Fetching {
            images: serve.tuning.message_images,
            per_turn: serve.tuning.fetch_per_turn,
            wait: serve.tuning.fetch(),
            quick: serve.tuning.call_timeout(),
            avatars: (
                serve.tuning.avatar_user_url.clone(),
                serve.tuning.avatar_group_url.clone(),
            ),
            members: serve.tuning.members_shown,
        },
        Forwarding {
            per_message: serve.tuning.message_forwards,
            depth: serve.tuning.forward_depth,
            nodes: serve.tuning.forward_nodes,
            chars: serve.tuning.forward_chars,
        },
    );
    let route = Route::new(
        core,
        (platform, bots),
        venues,
        members,
        parts,
        configured,
        hooks,
    );
    let route = tasks.spawn(async move { route.run(received).await }).id();
    tasks.spawn(status_file::keep(
        status_file::path(&serve.root),
        Arc::clone(&running),
        Arc::clone(&changed),
    ));
    tokio::pin!(stop);
    loop {
        tokio::select! {
            // 核心推来的配置（施工 O-20）：换上桥手里的那一份；驱动的那几项变了驱动自己换（施工 I-3）。发的一头在跟核心的那一头
            // 里，它停了收到空的，这一支不再看。
            Some(keys) = configs.recv() => {
                // 没认出来的配置项变了（施工 O-41 补）：状态文件跟着写，`miyu im status` 照它说。
                if current.change(&keys) {
                    changed.notify_one();
                }
            },
            Some(ended) = tasks.join_next_with_id() => match ended {
                Ok((_, Some(failure))) => {
                    tracing::warn!(target: TARGET, failure = ?failure, "route ended");
                    return Err(failure);
                }
                Ok((id, None)) if id == route => {
                    tracing::info!(target: TARGET, "core closed, stopping");
                    return Ok(());
                }
                Ok((_, None)) => {}
                Err(error) if error.id() == route => {
                    tracing::error!(target: TARGET, error = %error, "route crashed");
                    return Err(Failure::Crashed(error.to_string()));
                }
                Err(error) => {
                    tracing::error!(target: TARGET, error = %error, "connection crashed");
                }
            },
            () = &mut stop => {
                tracing::info!(target: TARGET, "stopped");
                return Ok(());
            }
        }
    }
}
