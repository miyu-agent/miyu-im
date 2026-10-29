//! 测试用的别的包（施工 O-43 上，`extending.md` 第二条）：程序是这个仓库的 `miyu-im-test-hook`（链到测试程序旁边，同
//! [`super::spawning::linked`]），清单写进管理员家目录那一层，核心起来时照开关拉起它。

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use miyu_endpoint::extensions::Timing;
use miyu_session::Models;
use miyu_store::root::DataRoot;
use miyu_tool::Catalog;

use super::events::run_log;
use super::group::WAIT;
use super::spawning::{self, link_beside};
use super::{CONFIG, Home, TOKEN, resources};

/// 测试程序旁边的 `miyu-im-test-hook`：头一次要的时候链上，之后留着。
pub fn linked() -> &'static str {
    static LINKED: OnceLock<PathBuf> = OnceLock::new();
    LINKED.get_or_init(|| {
        let exe = std::env::current_exe().expect("找得到测试程序");
        let dir = std::fs::canonicalize(&exe)
            .expect("测试程序在")
            .parent()
            .expect("有上一级")
            .to_path_buf();
        let path = dir.join(format!("miyu-im-test-hook{}", std::env::consts::EXE_SUFFIX));
        link_beside(Path::new(env!("CARGO_BIN_EXE_miyu-im-test-hook")), &path);
        path
    });
    "miyu-im-test-hook"
}

/// 往数据根 `root` 的系统那一层（`packages/`，核心 SU-5 上起直接在数据根下）装一个编号是 `id` 的扩展包：程序照 `args` 答（`record:`、`answer:`、`slow:`，见程序开头），
/// 导出 `exports` 那几个方法，核心起来就拉起（`start = "always"`），不要能力。导出了方法的是 im 的插件（施工 I-8 上）。
pub fn install(root: &DataRoot, id: &str, args: &[String], exports: &[&str]) {
    install_starting(root, id, args, exports, "always");
}

/// 往数据根 `root` 的系统那一层（`packages/`，核心 SU-5 上起直接在数据根下）装一个编号是 `id` 的扩展包，程序照 `args` 答，声明 `capabilities` 那几样能力、在
/// 批过（施工 O-51：登记工具要 `tools`），核心起来就拉起。
pub fn install_with(root: &DataRoot, id: &str, args: &[String], capabilities: &[&str]) {
    install_starting(root, id, args, &[], "always");
    approve(root, id, capabilities);
}

/// 装好的扩展包 `id` 声明 `capabilities` 那几样能力、批过（施工 I-8 上：插件调 im 的 `chat.note` 要 `packages.call`；核心 SU-2 中
/// 起批过的记在开关文件 `extensions.json` 的 `approved` 里，包的编号到能力，照名字排）。别的包批过的、开关照留着。
pub fn approve(root: &DataRoot, id: &str, capabilities: &[&str]) {
    let manifest = root.path().join("packages").join(id).join("package.toml");
    let text = std::fs::read_to_string(&manifest).expect("读得出");
    let listed: Vec<String> = capabilities
        .iter()
        .map(|name| format!("{name:?}"))
        .collect();
    let text = text.replace(
        "start = \"always\"\n",
        &format!(
            "start = \"always\"\ncapabilities = [{}]\n",
            listed.join(", ")
        ),
    );
    std::fs::write(&manifest, text).expect("写得进");
    let path = root.path().join("extensions.json");
    let mut switches: serde_json::Value = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| serde_json::json!({"version": 1, "on": {}}));
    let mut names: Vec<&str> = capabilities.to_vec();
    names.sort_unstable();
    switches["approved"][id] = serde_json::json!(names);
    std::fs::write(&path, switches.to_string()).expect("写得进");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("改得了权限");
    }
}

/// 同 [`install`]，`start` 照写的（`manual` 的要开关打开才拉起）。
pub fn install_starting(root: &DataRoot, id: &str, args: &[String], exports: &[&str], start: &str) {
    let program = linked();
    let dir = root.path().join("packages").join(id);
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let quote = |items: &mut dyn Iterator<Item = String>| {
        items
            .map(|item| format!("{item:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let manifest = format!(
        "[package]\nprotocol = [1, 1]\nname = {{ en = \"Hook {id}\" }}\n\n[command]\nname = \"{id}\"\nprogram = \"{program}\"\nabout = {{ en = \"H\" }}\n\n[process]\nargs = [{}]\nstart = \"{start}\"\n\n[export]\nmethods = [{}]\n",
        quote(&mut args.iter().cloned()),
        quote(&mut exports.iter().map(|name| (*name).to_string())),
    );
    // 挂挂接点的是 im 的插件（施工 I-8 上）：给 im 带一个目录，根上放 `plugin.toml`，哪个平台、哪个场所都开着。
    let manifest = match exports.is_empty() {
        true => manifest,
        false => {
            let plugin = dir.join("im");
            std::fs::create_dir_all(&plugin).expect("建得了目录");
            let declared = format!("[plugin]\nname = {{ en = \"Hook {id}\" }}\n");
            std::fs::write(plugin.join("plugin.toml"), declared).expect("写得进");
            format!("{manifest}\n[content.miyu-im]\ndir = \"im\"\n")
        }
    };
    std::fs::write(dir.join("package.toml"), manifest).expect("写得进");
}

impl Home {
    /// 同 [`Home::spawning_with`]，核心起来读清单以前先照 `install` 往数据根里放东西（施工 O-43 上：管理员自己装的别的包，
    /// [`install`]）。
    pub fn spawning_with_installed(
        models: Arc<dyn Models>,
        more: &str,
        install: impl FnOnce(&DataRoot),
    ) -> Home {
        Home::spawning_installed(models, more, |_| resources(), (install, Catalog::default()))
    }

    /// 起一个照开关拉起扩展的核心，资源目录照 `shipped`，起来以前照 `install` 往数据根里放东西，工具目录是 `catalog`（施工
    /// O-46：要她能调 `show` 的照 [`showing`]，别的是空的）。
    pub(super) fn spawning_installed(
        models: Arc<dyn Models>,
        more: &str,
        shipped: impl FnOnce(&Path) -> PathBuf,
        (install, catalog): (impl FnOnce(&DataRoot), Catalog),
    ) -> Home {
        spawning::linked();
        let timing = Timing {
            grace: Duration::from_secs(5),
            stable: Duration::from_secs(60),
            backoff: Duration::from_millis(20),
            longest: Duration::from_millis(100),
            ..Timing::default()
        };
        let secrets = format!("im = \"{TOKEN}\"\n");
        let config = format!("{CONFIG}{more}");
        Home::with_config(
            models,
            &config,
            Some(&secrets),
            Some(timing),
            shipped,
            (install, catalog),
        )
    }
}

/// 只有基础系统的 `show` 一件的工具目录（施工 O-46）：她给人看文件，核心记 `file.shown`，桥发进 QQ。字照源码树的资源目录读。
pub fn showing() -> Catalog {
    let tools = miyu_basesystem::tools(&resources())
        .expect("基础系统的字读得出")
        .into_iter()
        .filter(|tool| tool.spec().name == "show")
        .collect::<Vec<_>>();
    assert_eq!(tools.len(), 1, "基础系统有 show");
    Catalog::new(tools).expect("登记得上")
}

/// 等到挂了 `count` 个包、读到了 `count` 个插件（施工 I-8 上：插件另起任务读，两样都齐了挂接点才问）：同 [`until_plugged`]。
pub async fn until_hooked(home: &Home, count: usize) {
    until_plugged(home, count, count).await;
}

/// 等到挂了 `hooked` 个包（导出了挂接点、在跑的）、读到了 `plugins` 个插件：桥运行日志里最后一行说挂了几个的（订阅的回应
/// `hooks learned`、推来的变化 `hooks changed`）、最后一行说读到几个插件的（`plugins loaded`）。
pub async fn until_plugged(home: &Home, hooked: usize, plugins: usize) {
    let said = |log: &str, marks: &[&str], count: usize| {
        let wanted = format!("count={count}");
        log.lines()
            .rev()
            .find(|line| marks.iter().any(|mark| line.contains(mark)))
            .is_some_and(|line| line.split_whitespace().any(|word| word == wanted))
    };
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let log = run_log(&home.root);
        if said(&log, &["hooks learned", "hooks changed"], hooked)
            && said(&log, &["plugins loaded"], plugins)
        {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "等不到挂上 {hooked} 个、{plugins} 个插件：{log}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
