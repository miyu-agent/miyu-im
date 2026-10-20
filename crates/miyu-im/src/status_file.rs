//! 桥的状态文件（`onebot.md` 第一条「状态文件」，施工 O-18）：`<数据根>/state/packages/im/status.json`，一行 JSON：
//!
//! ```json
//! {"pid": 12345, "listen": 8301, "napcat": {"connected": true, "self_id": "30003"}}
//! ```
//!
//! - 桥的进程号、实际听的 NapCat 端口（推来的换过的照换过的）、页面服务实际听的端口 `web`（施工 I-5 上；O-28 下去掉的是桥原来
//!   自己的网页，I-5 上回来的是 im 自己的页面）、NapCat 的样子（同后台页的 `status`，`Bots::napcat`）。
//! - 端口听上以后写一次；之后有人叫一声（NapCat 连上、断开、认出号、问到实现，推来的换了端口）就照这一刻再写。和磁盘上
//!   一样的不写，先写临时文件再改名盖上（`miyu_client::places::write`）：读的人看不到写了一半的。写不进记一行，桥照跑。
//! - 照数据根算位置，不照工作目录的相对路径：核心给的工作目录就是这个目录（`extensions.md`「怎么走」第 1 条），在进程里跑的
//!   测试也不会写进源码树（「施工时定的」第 23 条）。
//! - 桥退出不删：`miyu im status` 只在核心说桥在跑、进程号对得上时用它（`crate::control`）。

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};

use miyu_client::places::DataRoot;

use crate::changed::Changed;
use crate::running::Running;
use crate::serve::Failure;
use crate::{PACKAGE, TARGET};

/// 文件名：在这个包放状态的目录里。
const FILE: &str = "status.json";

/// 数据根 `root` 里平台 `platform` 的驱动自己的状态目录（施工 Q-2、Q-6）：im 的状态目录下照平台的名字，同状态文件放一起。
pub fn store(root: &DataRoot, platform: crate::chat::Platform) -> PathBuf {
    path(root).with_file_name(platform.name())
}

/// 数据根 `root` 里桥的状态文件在哪。
pub fn path(root: &DataRoot) -> PathBuf {
    root.state().join("packages").join(PACKAGE).join(FILE)
}

/// 读数据根 `root` 里的状态文件；没有的、读不了的、不是 JSON 的是空的。
pub fn read(root: &DataRoot) -> Option<Value> {
    let bytes = std::fs::read(path(root)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// 照 `running` 这一刻的样子写进 `path`，之后每叫一声 `changed` 再写一次。一直写下去，桥停下时随一组任务一起停；交回的类型照
/// 那一组（`crate::serve`）。
pub(crate) async fn keep(
    path: PathBuf,
    running: Arc<Running>,
    changed: Arc<Changed>,
) -> Option<Failure> {
    let mut changes = changed.watch();
    loop {
        let mut now = json!({
            "pid": std::process::id(),
            "napcat": running.bots.napcat(),
        });
        // 驱动报的几格（施工 I-3：OneBot 的 `listen`）照原样并进来。
        for (key, value) in running.reported.get() {
            now[key] = value;
        }
        // 页面服务实际听的端口（施工 I-5 上）：没听上的不带。
        if let Some(port) = running.page.get() {
            now["web"] = json!(port);
        }
        // 没认出来的配置项（施工 O-41 补）：`miyu im status` 照它说一句；空的不带。
        let misread = running.current.misread();
        if !misread.is_empty() {
            now["misread"] = json!(misread);
        }
        let bytes = format!("{now}\n").into_bytes();
        let target = path.clone();
        match tokio::task::spawn_blocking(move || miyu_client::places::write(&target, &bytes)).await
        {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                tracing::warn!(target: TARGET, file = %path.display(), error = %error, "status not written");
            }
            Err(error) => {
                tracing::warn!(target: TARGET, error = %error, "status not written");
            }
        }
        if changes.changed().await.is_err() {
            // 叫的一头都放下了（桥在停）：不再写。
            return None;
        }
    }
}
