//! 桥的测试共用的（施工 O-8，`onebot.md`「守着它的」）：临时的数据根里起一个真的核心（请求模型照剧本回，系统配置里
//! `qq:10001` 是管理员本人；配置清单照真核心起来时那样拼进出厂的包的配置项，施工 O-20），起一个桥（端口 0 让系统挑），假的
//! NapCat 是一个 WebSocket 客户端。在进程里跑的桥经内存里的管道连核心（`pipe`，施工 O-18），握手的回应里照核心拉起扩展的样子
//! 填上配置（施工 O-20）；真核心拉起真桥、跑真的程序的在 `spawning`。

#![allow(dead_code, reason = "几个测试各用其中一部分")]

pub mod answering;
pub mod assembled;
pub mod browser;
pub mod events;
pub mod fake_core;
pub mod group;
pub mod head;
pub mod shipped;
pub use shipped::*;
pub mod hooked;
pub mod judge;
pub mod kicking;
pub mod media;
pub mod napcat;
pub mod notices;
pub mod pipe;
pub mod platform;
pub mod ports;
mod roots;
pub mod spawning;
pub mod speaking;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use miyu_config::secret::Secret;
use miyu_endpoint::Core;
use miyu_endpoint::config::{Config, Environment};
use miyu_endpoint::extensions::Timing;
use miyu_im::package::Package;
use miyu_im::serve::{Failure, Notice, Serve, run};
use miyu_im_onebot::settings::Settings;
use miyu_kernel::id::{AccountId, SessionId};
use miyu_session::Models;
use miyu_session::testkit::Script;
use miyu_store::log::read_events;
use miyu_store::resources::ResourceRoot;
use miyu_store::root::DataRoot;
use miyu_tool::Catalog;

pub use roots::temp_root;

#[allow(unused_imports, reason = "读配置的测试用不上假 NapCat")]
pub use napcat::*;
pub use pipe::{Bindings, Relay};

/// 终端管理员的 QQ 号：系统配置里对着管理员。
pub const ADMIN: i64 = 10001;

/// 不在对应表里的人。
pub const STRANGER: i64 = 20002;

/// 机器人的号。
pub const BOT: i64 = 30003;

/// 假 NapCat 发的私聊带的平台时刻（事件的 `time`，整数秒）：拼进命令编号的最后一段。
pub const TIME: i64 = 1_759_800_000;

/// 桥的访问令牌。
pub const TOKEN: &str = "napcat-test-token";

/// 系统配置：终端管理员对应表；`qq:10007` 是终端管理员的小号（施工 O-31：没在群里说过话，桥照 `venue.binding` 认）。
const CONFIG: &str = "[external.bindings]\n\"qq:10001\" = \"admin\"\n\"qq:10007\" = \"admin\"\n";

/// 一个用完就删的临时数据根，里面跑着一个核心。
pub struct Home {
    dir: PathBuf,
    pub root: DataRoot,
    core: Arc<Core>,
    running: JoinHandle<std::convert::Infallible>,
}

/// 管理员的账号。
pub fn admin() -> AccountId {
    AccountId::parse("admin").expect("账号合写法")
}

/// 出厂的资源目录：钉住的核心的 `resources/` 加上这个仓库的 `im/`，拼成的临时目录（施工 S-6 中，[`assembled`]）。
pub fn resources() -> PathBuf {
    assembled::resources()
}

/// 资源目录 `resources` 里出厂的那一份包文件夹（`packages/im/`，施工 S-6 上）。
pub fn package_in(resources: impl AsRef<Path>) -> Package {
    Package::at(resources.as_ref().join("packages").join("im"))
}

/// 这个仓库的包文件夹 `im/`：直接拉桥的照它交 `MIYU_PACKAGE_DIR`（施工 S-6 中）。
pub fn package() -> Package {
    Package::at(assembled::package_dir())
}

/// 源码树的资源目录抄一份到 `dir/resources`，`bridge.json` 照 `keys` 改那几格（施工 O-25 中）：交回抄好的目录。
pub fn tuned_resources(dir: &Path, keys: &serde_json::Map<String, Value>) -> PathBuf {
    let copy = dir.join("resources");
    copy_tree(&resources(), &copy);
    let file = package_in(&copy).data().join(miyu_im::tuning::FILE);
    let mut bridge: Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("读得了")).expect("是 JSON");
    for (key, value) in keys {
        bridge[key] = value.clone();
    }
    std::fs::write(&file, bridge.to_string()).expect("写得进");
    copy
}

/// 把目录 `from` 整个抄到 `to`。
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("建得了目录");
    for entry in std::fs::read_dir(from).expect("列得出") {
        let entry = entry.expect("读得了");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("看得出种类").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("抄得了");
        }
    }
}

impl Home {
    /// 起一个核心：请求模型照 `script`，没有工具，系统配置是终端管理员对应表。
    pub fn new(script: &Script) -> Home {
        Home::speaking(Arc::new(script.clone()))
    }

    /// 同 [`Home::new`]，请求模型照 `models`（施工 O-25 上：她照台词说，[`speaking::Lines`]）。
    pub fn speaking(models: Arc<dyn Models>) -> Home {
        Home::with_config(
            models,
            CONFIG,
            None,
            None,
            |_| resources(),
            (|_| {}, Catalog::default()),
        )
    }

    /// 起一个照开关拉起扩展的核心（施工 O-18）：系统配置是终端管理员对应表接着 `more`（端口、令牌、语言），密钥文件里 `onebot` 是
    /// [`TOKEN`]（[`spawning::ports_config`] 引用它；核心起来以前写好：核心握手时交的是它起来时读到的，施工 O-20），退避从
    /// 20 毫秒起、最多 100 毫秒，请扩展退出以后照出厂的等 5 秒再杀（等的时候桥得自己退）。出厂的清单里有桥：开了就拉起测试程序
    /// 旁边的 `miyu-im`（[`spawning::linked`]）。
    pub fn spawning(script: &Script, more: &str) -> Home {
        Home::spawning_with(Arc::new(script.clone()), more)
    }

    /// 同 [`Home::spawning`]，请求模型照 `models`（施工 O-23 下：判官另走一头，[`judge::models`]）。
    pub fn spawning_with(models: Arc<dyn Models>, more: &str) -> Home {
        Home::spawning_tuned(models, more, &Value::Null)
    }

    /// 同 [`Home::spawning_with`]，`tuned` 是对象的：资源目录是源码树的一份拷贝，`bridge.json` 照它改那几格（施工 O-25 中：
    /// 要等过期的测试改小 `queue_expire_seconds`；真核心拉起的桥读的是核心的资源目录，`onebot.md`「施工时定的」第 127 条）。
    pub fn spawning_tuned(models: Arc<dyn Models>, more: &str, tuned: &Value) -> Home {
        Home::spawning_from(models, more, |dir| match tuned.as_object() {
            Some(keys) => tuned_resources(dir, keys),
            None => resources(),
        })
    }

    /// 同 [`Home::spawning_with`]，资源目录是源码树的一份拷贝，抄好以后交给 `edit` 改（施工 O-26：桥的工具说明改成核心不收的）。
    pub fn spawning_edited(models: Arc<dyn Models>, more: &str, edit: impl FnOnce(&Path)) -> Home {
        Home::spawning_from(models, more, |dir| {
            let copy = tuned_resources(dir, &serde_json::Map::new());
            edit(&copy);
            copy
        })
    }

    /// 起一个照开关拉起扩展的核心（见 [`Home::spawning`]），资源目录照 `shipped`：交进临时目录，交回用哪个资源目录。
    fn spawning_from(
        models: Arc<dyn Models>,
        more: &str,
        shipped: impl FnOnce(&Path) -> PathBuf,
    ) -> Home {
        Home::spawning_installed(models, more, shipped, (|_| {}, Catalog::default()))
    }

    /// 起一个核心：请求模型照 `models`，系统配置写成 `config`，有 `secrets` 的密钥文件写成它，读清单以前照 `install` 往数据根里
    /// 放东西（施工 O-43 上）；`extensions` 有的照它等、退避，
    /// 照开关拉起扩展。配置清单照真核心起来时那样拼进出厂的包的配置项（`Packaged`，施工 O-20：`onebot.*` 在桥的清单里）。
    /// 资源目录照 `shipped`：交进临时目录，交回用哪个（源码树的，或者一份改过的拷贝，[`tuned_resources`]）。
    fn with_config(
        models: Arc<dyn Models>,
        config: &str,
        secrets: Option<&str>,
        extensions: Option<Timing>,
        shipped: impl FnOnce(&Path) -> PathBuf,
        (install, catalog): (impl FnOnce(&DataRoot), Catalog),
    ) -> Home {
        let (dir, root) = temp_root();
        install(&root);
        let file = root.path().join("system").join("config.toml");
        std::fs::create_dir_all(file.parent().expect("有上一级")).expect("建得了目录");
        std::fs::write(&file, config).expect("写得进");
        if let Some(secrets) = secrets {
            std::fs::write(file.with_file_name("secrets.toml"), secrets).expect("写得进");
        }
        let shipped = shipped(&dir);
        let dirs = miyu_ipc::Dirs {
            runtime_dir: None,
            ..miyu_ipc::Dirs::current()
        };
        let opened = miyu_ipc::open(&root, &dirs).expect("起得来");
        let shipped = ResourceRoot::at(shipped);
        let mut found = miyu_endpoint::packages::load(&shipped, &root, &admin());
        let packaged = miyu_core::settings::Packaged::of(&mut found);
        let config = Config::load(&root, &admin(), None, packaged.all(), Environment::of(&[]));
        let core = Arc::new(
            Core::new(
                root.clone(),
                shipped,
                models,
                catalog,
                None,
                admin(),
                opened.token.clone(),
            )
            .with_config(config)
            .with_packages(found)
            .with_extension_timing(extensions.unwrap_or_default()),
        );
        if extensions.is_some() {
            core.start_extensions();
        }
        let running = tokio::spawn(miyu_endpoint::run(opened.listener, Arc::clone(&core)));
        Home {
            dir,
            root,
            core,
            running,
        }
    }

    /// 请核心拉起的扩展都退出，等它们退出（施工 O-18）。
    pub async fn stop_extensions(&self) {
        self.core.stop_extensions().await;
    }

    /// 管理员名下的会话（场所会话也在这里）。
    pub fn sessions(&self) -> Vec<SessionId> {
        self.root.sessions(&admin()).expect("列得出")
    }

    /// 会话 `session` 日志里的事件，写成 JSON。还没写出第一条的当是空的。
    pub fn events(&self, session: &SessionId) -> Vec<Value> {
        read_events(&self.root.session_dir(&admin(), session))
            .unwrap_or_default()
            .iter()
            .map(|event| serde_json::to_value(event).expect("写得成 JSON"))
            .collect()
    }

    /// 全部会话里人说的话（`message.user`），照会话、照先后。
    pub fn said(&self) -> Vec<Value> {
        self.sessions()
            .iter()
            .flat_map(|session| self.events(session))
            .filter(|event| event["kind"] == "message.user")
            .collect()
    }

    /// 全部会话里人说的话的文字。
    pub fn said_texts(&self) -> Vec<String> {
        self.said()
            .iter()
            .map(|event| {
                event["body"]["blocks"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            })
            .collect()
    }
}

impl Drop for Home {
    #[expect(
        clippy::let_underscore_must_use,
        reason = "删不掉就留在临时目录里，不影响测试"
    )]
    fn drop(&mut self) {
        self.running.abort();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 跑着的一个桥。
pub struct Bridge {
    /// NapCat 连进来的端口，实际听的那一个。
    pub port: u16,
    /// 它说给人听的，照先后。
    pub notices: Arc<Mutex<Vec<Notice>>>,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), Failure>>,
}

impl Bridge {
    /// 叫它停下，等它停完。
    pub async fn stop(mut self) -> Result<(), Failure> {
        if let Some(stop) = self.stop.take()
            && stop.send(()).is_err()
        {
            // 它已经自己停了。
        }
        within("桥停下", &mut self.task).await.expect("没崩")
    }

    /// 不叫它停，等它自己停下（施工 O-18：核心关了管道）。
    pub async fn ended(mut self) -> Result<(), Failure> {
        within("桥自己停下", &mut self.task).await.expect("没崩")
    }
}

/// 端口是 0（让系统挑），令牌是 [`TOKEN`]。
pub fn settings() -> Settings {
    with_token(Some(TOKEN))
}

/// 测试里的桥只听本机（施工 I-27：出厂的 `0.0.0.0` 谁都能连，测试不往外开）。
pub const LOCAL: std::net::IpAddr = std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);

/// 只听本机，端口是 0，令牌是 `token`；空的是没有令牌。
pub fn with_token(token: Option<&str>) -> Settings {
    Settings {
        host: LOCAL,
        port: 0,
        token: token.map(|token| Secret::new(token).expect("合写法")),
    }
}

/// 令牌没设的那一句（跟在说在哪等 NapCat 的后面）。
pub fn no_token() -> Notice {
    Notice::NoToken
}

/// 照核心拉起扩展时握手交的样子写 `settings`（施工 O-20，`extensions.md`「配置」）：端口、令牌的真值；没有令牌的不放这一键。
pub fn handed(settings: &Settings) -> Value {
    let mut config =
        json!({"im.qq.host": settings.host.to_string(), "im.qq.listen": settings.port});
    if let Some(token) = &settings.token {
        config["im.qq.token"] = json!(token.expose());
    }
    config
}

/// 在数据根 `root` 上起一个桥要的：握手交的配置照 `settings`，经内存里的管道连 `root` 上的那个核心（[`pipe::pipe_to`]，施工
/// O-18），`bindings` 有的照它答 `venue.binding`，说中文，出厂的 `bridge.json`、资源目录、清单的默认值和出厂的场所规则这几样
/// （施工 O-21）。另交回测试那一头的转接 [`Relay`]：推配置（施工 O-20）、看桥问了核心什么。
pub fn serve_relayed(
    root: DataRoot,
    settings: Settings,
    bindings: Option<Bindings>,
) -> (Serve, Relay) {
    serve_handed(root, handed(&settings), bindings)
}

/// 同 [`serve_relayed`]，握手交的配置是 `config`（施工 O-39：多交 `im.whitelist`）。
pub fn serve_handed(root: DataRoot, config: Value, bindings: Option<Bindings>) -> (Serve, Relay) {
    let (pipe, relay) = pipe::pipe_to(&root, config, bindings);
    let serve = Serve {
        pipe,
        root,
        locale: Some("zh_CN.UTF-8".to_string()),
        tuning: tuning(),
        defaults: browser::any_page_port(defaults()),
        factory: factory(),
        driver: std::sync::Arc::new(miyu_im_onebot::OneBot),
        package: package(),
    };
    (serve, relay)
}

/// 同 [`serve_relayed`]，账号照核心原样。
pub fn serve_pushing(root: DataRoot, settings: Settings) -> (Serve, Relay) {
    serve_relayed(root, settings, None)
}

/// 同 [`serve_pushing`]，不推配置。
pub fn serve(root: DataRoot, settings: Settings) -> Serve {
    serve_pushing(root, settings).0
}

/// 在 `home` 上起一个桥：端口 0，令牌是 [`TOKEN`]，等它说在哪个端口听。
pub async fn bridge(home: &Home) -> Bridge {
    start(serve(home.root.clone(), settings())).await
}

/// 照 `serve` 起一个桥，等它说在哪个端口等 NapCat（施工 O-28 下起只这一个端口）。令牌没设的那一句跟在后面说，要看它的自己等。
pub async fn start(serve: Serve) -> Bridge {
    let notices = Arc::new(Mutex::new(Vec::new()));
    let (stop, stopped) = oneshot::channel::<()>();
    let (told, mut telling) = tokio::sync::mpsc::unbounded_channel();
    let heard = Arc::clone(&notices);
    let task = tokio::spawn(run(
        serve,
        |_| {},
        move |notice| {
            if told.send(notice.clone()).is_err() {
                // 等的那头已经不在了。
            }
            heard.lock().expect("没 panic").push(notice);
        },
        async move {
            if stopped.await.is_err() {
                // 测试丢了停的那一头：照样停。
            }
        },
    ));
    let port = within("桥开始听", async {
        loop {
            if let Notice::Listening { port, .. } = telling.recv().await.expect("说了端口") {
                return port;
            }
        }
    })
    .await;
    Bridge {
        port,
        notices,
        stop: Some(stop),
        task,
    }
}

/// 等 `future`，最多 [`group::WAIT`]（六十秒）。正向的等待照仓库的规矩放到六十秒：真核心冷启动、重启再连，机器压满时
/// 十秒不够（整套测试一起跑时撞出来过）；平常一下就到，放长不拖慢。
pub async fn within<T>(what: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(group::WAIT, future)
        .await
        .unwrap_or_else(|_| panic!("六十秒内没等到{what}"))
}

/// 照核心转来后台页调 `status` 的样子，经 `relay` 往桥推一条 `method.call`（编号 `id`，各次不重），等到桥回了交回 `result`
/// （施工 O-28 下：原来看 WebUI 的 `/status`）。
pub async fn page_status(relay: &Relay, id: &str) -> Value {
    relay.request(json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "method.call",
        "params": {"method": "status", "params": {}},
    }));
    within("桥答 status", async {
        loop {
            if let Some(answer) = relay.answers().into_iter().find(|one| one["id"] == id) {
                return answer["result"].clone();
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
}
