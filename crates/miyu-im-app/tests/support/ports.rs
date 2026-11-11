//! 挑的空端口被别人先占了，换一个再来（偶发红的根因，2026-10-08 查的）：[`free_port`] 绑 `127.0.0.1:0` 拿到号就放掉，过一阵
//! 真的程序才去绑；机器负载高时这个号可能先被别的测试拿走（进程里的桥照端口 0 让系统挑、别的测试的 [`free_port`]），桥说端口
//! 被占、退出码 1。真的程序起桥的测试照 [`on_free_port`] 跑：桥说的正是挑的那个端口被占了，作废、从头再来；别的原因退出照旧
//! 当失败。进程里起桥的给端口 0、让系统挑，碰不到这件事。施工 O-28 下起桥只开 NapCat 的一个端口（原来还有 WebUI 的）。
//!
//! 施工 I-210 起 [`free_port`] 不再让系统挑：从系统自己挑不到的一段里往后发，放掉到用上之间没有别人会拿走（页面服务的端口
//! 没有 [`on_free_port`] 这样的重来，原来撞上了就是一条红）。[`on_free_port`] 照留，挡这一段里碰巧有别的程序占着的。

use miyu_im::serve::Failure;
use miyu_im::texts::Texts;
use miyu_store::resources::ResourceRoot;

use super::{package_in, resources};

/// 最多试几个端口。
pub const TRIES: usize = 5;

/// 桥说挑的端口被占了：作废，换一个再来。
#[derive(Debug)]
pub struct Taken;

/// 挑一个空端口（NapCat 的）跑 `attempt`；它交回 [`Taken`] 的，换一个从头再跑，最多 [`TRIES`] 次。
pub async fn on_free_port<T>(attempt: impl AsyncFn(u16) -> Result<T, Taken>) -> T {
    for _ in 0..TRIES {
        if let Ok(done) = attempt(free_port()).await {
            return done;
        }
    }
    panic!("试了 {TRIES} 个端口，桥都说被占了");
}

/// 桥说的 `said`（标准错误）里有没有说 `listen` 被占了。三种语言的那一句都认：照握手回的语言说（施工 O-20），看测试当的
/// 核心回的是哪种。
pub fn taken(said: &str, listen: u16) -> bool {
    ["zh", "en", "ja"].into_iter().any(|language| {
        let texts = Texts::load(
            ResourceRoot::at(resources()),
            package_in(resources()),
            language,
        )
        .expect("读得出来");
        said.contains(&texts.failure(&Failure::PortInUse(listen)))
    })
}

/// 一个空着的端口（施工 I-210）：从系统自己不会挑的一段里（[`FREE_PORTS`]）一个一个往后发，绑得上的才交。
///
/// 原来是让系统挑一个、马上放掉：放掉到用上之间，同一个测试程序里别的测试让系统挑端口（假 NapCat、假的图片服务），挑到的
/// 可以正是这一个，用的一方就绑不上（CI 上偶发的 `page port not opened … Address already in use`）。这一段在 Linux
///（32768 起）、macOS 和 Windows（49152 起）让系统挑的范围以外，同一个进程里发过的不再发。
pub fn free_port() -> u16 {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let span = u32::from(FREE_PORTS.end - FREE_PORTS.start);
    // 从哪里发起照进程号错开：一台机器上同时跑的两个测试程序不从同一个发起。
    let first = std::process::id() % span;
    loop {
        let nth = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let port = FREE_PORTS.start + u16::try_from((first + nth) % span).expect("在段里");
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
}

/// [`free_port`] 发的那一段：哪个系统让它自己挑都挑不到这里。
pub const FREE_PORTS: std::ops::Range<u16> = 20000..32000;

#[test]
fn free_ports_are_never_ones_the_system_hands_out_and_never_repeat() {
    // 施工 I-210：发的端口不在系统自己挑的范围里，同一个进程里不发第二次（原来让系统挑了就放，两样都不成立）。
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..50 {
        let port = free_port();
        assert!(FREE_PORTS.contains(&port), "{port}");
        assert!(seen.insert(port), "{port} 发了两次");
    }
}
