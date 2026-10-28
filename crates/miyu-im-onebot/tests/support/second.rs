//! 假的第二个平台（施工 I-70 中上，`tests/drivers.rs`）：平台名 `tg` 的一个驱动，和 OneBot 一起交给 `Serve`。它不听端口：起来时
//! 记下宿主给的 `Gate`，报一格状态；测试经它交事件（[`Telegram::says`]），它连着的号用宿主的假连接（`miyu_im::chat::fake`），她
//! 发的字从那条队里取（[`Telegram::sent`]）。它交一套工具（`tg_echo`），说明、答的话写进一份抄出来的包文件夹（[`two_platforms`]）；
//! 每次调用记下平台、场所、配置、状态目录。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};
use tokio::sync::{Notify, mpsc};

use miyu_client::tooltext::Access;
use miyu_im::chat::fake::{Asked, Did, fake};
use miyu_im::chat::{Cap, Event, Platform, Posted, To};
use miyu_im::driver::{Driver, Gate, Started};
use miyu_im::package::Package;
use miyu_im::rules::Factory;
use miyu_im::serve::Serve;
use miyu_im::toolset::{Call, Decl, Planned, ToolSet};
use miyu_im::tuning::Tuning;
use miyu_session::Models;
use miyu_tool::Catalog;

use super::{ADMIN, Home, TIME, copy_tree, package, resources, tuning, within};

/// 假的第二个平台。
pub const TG: Platform = Platform::new("tg");

/// 它的工具：名字带平台前缀（核心 IM-3 照前缀认平台）。
pub const ECHO: &str = "tg_echo";

/// 它的配置项：在 `im.tg.` 那一组。
const MODE: &str = "mode";

/// 系统配置：两个平台上的 10001 都是终端管理员本人。
const CONFIG: &str = "[external.bindings]\n\"qq:10001\" = \"admin\"\n\"tg:10001\" = \"admin\"\n";

/// 一次调用里记下的：平台、场所、驱动的配置、状态目录。
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    /// `Call::platform` 的名字。
    pub platform: &'static str,
    /// 场所编号。
    pub venue: String,
    /// 驱动的配置。
    pub config: Map<String, Value>,
    /// 驱动自己的状态目录。
    pub store: PathBuf,
}

/// 假平台的工具：`tools` 那几件（`tg_echo`；撞名的测试用 OneBot 的名字），都答 `echoed`。
struct Echo {
    tools: &'static [Decl],
    calls: Arc<Mutex<Vec<Seen>>>,
}

/// `tg_echo`。
const ECHO_TOOLS: &[Decl] = &[Decl {
    name: ECHO,
    access: Access::Read,
    venues: &["private", "group"],
}];

/// 和 OneBot 撞名的一件。
const CLASHING: &[Decl] = &[Decl {
    name: "qq_poke",
    access: Access::Read,
    venues: &["private", "group"],
}];

impl ToolSet for Echo {
    fn dir(&self) -> &'static str {
        "tg"
    }

    fn tools(&self) -> &'static [Decl] {
        self.tools
    }

    fn answers(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &[("echoed", &[]), ("unreachable", &[])]
    }

    fn plan(&self, call: Call) -> Planned {
        self.calls.lock().expect("没 panic").push(Seen {
            platform: call.platform.name(),
            venue: call.venue.clone(),
            config: call.config.clone(),
            store: call.store.clone(),
        });
        Planned::Now(call.answers.say("echoed", &[], false))
    }
}

/// 假的第二个平台：起来时交来的 `Gate`、它交的工具记下的调用。
#[derive(Clone)]
pub struct Telegram {
    gate: Arc<Mutex<Option<Arc<Gate>>>>,
    started: Arc<Notify>,
    calls: Arc<Mutex<Vec<Seen>>>,
    tools: &'static [Decl],
}

impl Telegram {
    /// 交 `tg_echo` 的一个。
    pub fn echoing() -> Telegram {
        Telegram::with_tools(ECHO_TOOLS)
    }

    /// 交的工具和 OneBot 的撞名的一个。
    pub fn clashing() -> Telegram {
        Telegram::with_tools(CLASHING)
    }

    fn with_tools(tools: &'static [Decl]) -> Telegram {
        Telegram {
            gate: Arc::new(Mutex::new(None)),
            started: Arc::new(Notify::new()),
            calls: Arc::new(Mutex::new(Vec::new())),
            tools,
        }
    }

    /// 宿主给它的 `Gate`：等它起来。
    pub async fn gate(&self) -> Arc<Gate> {
        within("假平台起来", async {
            loop {
                let notified = self.started.notified();
                if let Some(gate) = self.gate.lock().expect("没 panic").clone() {
                    return gate;
                }
                notified.await;
            }
        })
        .await
    }

    /// 号 `bot` 连上：连接是宿主的假连接，交回它收到的要做的事。
    pub async fn connect(&self, bot: i64) -> mpsc::Receiver<Asked> {
        let gate = self.gate().await;
        let (link, asked) = fake();
        let replaced = gate.bots().insert(bot, link);
        assert!(replaced.is_none(), "头一次连");
        assert!(gate.inbound(Event::Connected { bot }).await, "宿主收");
        gate.changed();
        asked
    }

    /// 终端管理员（号 [`ADMIN`]）经号 `bot` 私聊说第 `message_id` 条 `text`，时刻是 [`TIME`]。
    pub async fn says(&self, bot: i64, message_id: i64, text: &str) {
        let posted = Posted {
            bot,
            user: ADMIN,
            message_id,
            time: TIME,
            name: None,
            rank: None,
            text: text.to_string(),
            segments: miyu_im::chat::Segments::default(),
        };
        assert!(
            self.gate().await.inbound(Event::Private(posted)).await,
            "宿主收"
        );
    }

    /// 她经假连接发的下一句：答它成了，交回发到哪、字。最多等六十秒。
    pub async fn sent(asked: &mut mpsc::Receiver<Asked>) -> (To, String) {
        let asked = within("假平台发", asked.recv()).await.expect("连接还在");
        let Did::Send { to, text } = asked.did.clone() else {
            panic!("该是发一句：{:?}", asked.did);
        };
        asked.answer(true);
        (to, text)
    }

    /// 它的工具记下的调用，照先后。
    pub fn calls(&self) -> Vec<Seen> {
        self.calls.lock().expect("没 panic").clone()
    }
}

impl Driver for Telegram {
    fn platform(&self) -> Platform {
        TG
    }

    fn caps(&self) -> &'static [Cap] {
        &[]
    }

    fn settings(&self) -> &'static [&'static str] {
        &[MODE]
    }

    fn toolset(&self, _: &Tuning) -> Option<Arc<dyn ToolSet>> {
        Some(Arc::new(Echo {
            tools: self.tools,
            calls: Arc::clone(&self.calls),
        }))
    }

    fn start(&self, gate: Arc<Gate>) -> Started {
        let (slot, started) = (Arc::clone(&self.gate), Arc::clone(&self.started));
        Box::pin(async move {
            // 报一格状态：状态文件里 `drivers.tg` 照它。
            let mode = gate.config().borrow().get(MODE).cloned();
            gate.report(Map::from_iter([("mode".to_string(), json!(mode))]));
            *slot.lock().expect("没 panic") = Some(gate);
            started.notify_waiters();
            let running: miyu_im::driver::Running = Box::pin(std::future::pending());
            Ok(running)
        })
    }
}

/// 抄一份这个仓库的包文件夹到 `dir`，写进假平台的工具说明、答的话：交回它。
pub fn package_with_tg(dir: &Path) -> Package {
    let copy = dir.join("im-with-tg");
    copy_tree(package().dir(), &copy);
    let data = copy.join("data").join("tg");
    let write = |file: PathBuf, text: &str| {
        std::fs::create_dir_all(file.parent().expect("有上一级")).expect("建得了目录");
        std::fs::write(file, text).expect("写得进");
    };
    write(
        data.join("tools").join(format!("{ECHO}.json")),
        r#"{"description": "Echo back.", "parameters": {"type": "object", "properties": {}}}"#,
    );
    write(
        data.join("tools").join("qq_poke.json"),
        r#"{"description": "Clash.", "parameters": {"type": "object", "properties": {}}}"#,
    );
    write(
        data.join("tool-results").join("echoed.txt"),
        "Echoed on tg.",
    );
    write(
        data.join("tool-results").join("unreachable.txt"),
        "tg is not reachable.",
    );
    Package::at(copy)
}

/// 出厂的工具加上假平台的：照包文件夹 `package` 读（同桥起来时，[`Factory::load`]）。
pub fn factory_with(package: &Package, tg: &Telegram) -> Result<Factory, Vec<miyu_chat::Problem>> {
    let onebot = miyu_im_onebot::OneBot;
    let tuning = tuning();
    let toolsets = [
        onebot.toolset(&tuning).map(|set| (onebot.platform(), set)),
        tg.toolset(&tuning).map(|set| (tg.platform(), set)),
    ];
    Factory::load(package, toolsets.into_iter().flatten().collect())
}

impl Home {
    /// 起一个核心：请求模型照 `models`，两个平台上的 10001 都是终端管理员本人（[`CONFIG`]）。
    pub fn two_platforms(models: Arc<dyn Models>) -> Home {
        Home::with_config(
            models,
            CONFIG,
            None,
            None,
            |_| resources(),
            (|_| {}, Catalog::default()),
        )
    }

    /// 临时目录里的一个子目录 `name`（测试自己放东西）。
    pub fn scratch(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

/// `serve` 换成 OneBot 加假平台 `tg` 两个驱动：工具照抄出来的包文件夹 `package` 读。
pub fn with_tg(serve: Serve, tg: &Telegram, package: &Package) -> Serve {
    Serve {
        drivers: vec![Arc::new(miyu_im_onebot::OneBot), Arc::new(tg.clone())],
        factory: factory_with(package, tg).expect("两套工具读得出"),
        ..serve
    }
}
