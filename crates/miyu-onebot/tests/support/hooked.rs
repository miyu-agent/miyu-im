//! 测试用的别的包（施工 O-43 上，`extending.md` 第二条）：程序是这个仓库的 `miyu-onebot-test-hook`（链到测试程序旁边，同
//! [`super::spawning::linked`]），清单写进管理员家目录那一层，核心起来时照开关拉起它。

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use miyu_endpoint::extensions::Timing;
use miyu_session::Models;
use miyu_store::root::DataRoot;
use miyu_tool::Catalog;

use super::spawning::{self, link_beside};
use super::{CONFIG, Home, TOKEN, resources};

/// 测试程序旁边的 `miyu-onebot-test-hook`：头一次要的时候链上，之后留着。
fn linked() -> &'static str {
    static LINKED: OnceLock<PathBuf> = OnceLock::new();
    LINKED.get_or_init(|| {
        let exe = std::env::current_exe().expect("找得到测试程序");
        let dir = std::fs::canonicalize(&exe)
            .expect("测试程序在")
            .parent()
            .expect("有上一级")
            .to_path_buf();
        let path = dir.join(format!(
            "miyu-onebot-test-hook{}",
            std::env::consts::EXE_SUFFIX
        ));
        link_beside(
            Path::new(env!("CARGO_BIN_EXE_miyu-onebot-test-hook")),
            &path,
        );
        path
    });
    "miyu-onebot-test-hook"
}

/// 往数据根 `root` 的管理员家目录装一个编号是 `id` 的扩展包：程序照 `args` 答（`record:`、`answer:`、`slow:`，见程序开头），
/// 导出 `exports` 那几个方法，核心起来就拉起（`start = "always"`），不要能力。
pub fn install(root: &DataRoot, id: &str, args: &[String], exports: &[&str]) {
    install_starting(root, id, args, exports, "always");
}

/// 同 [`install`]，`start` 照写的（`manual` 的要开关打开才拉起）。
pub fn install_starting(root: &DataRoot, id: &str, args: &[String], exports: &[&str], start: &str) {
    let program = linked();
    let dir = root
        .path()
        .join("home")
        .join("admin")
        .join("packages")
        .join(id);
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
        };
        let secrets = format!("onebot = \"{TOKEN}\"\n");
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
