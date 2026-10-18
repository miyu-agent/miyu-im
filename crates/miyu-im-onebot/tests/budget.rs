//! 桥的工具面预算（`docs/designs/10-自带软件.md` 第九节，施工 O-26、O-31、O-33）：`im/data/tools/` 下的说明加起来
//! 不超过 [`BUDGET`] 字节。
//!
//! 照基础系统、记忆的办法：2026-10-09 照 magpie 网关的 `clinepass/cline-pass/deepseek-v4.1-flash` 量，十六件（基础系统十五件加
//! `skip_reply`）一起时 `skip_reply` 的边际份量 70 个 token，说明 249 字节，预算 280 字节。施工 O-31 加 `recall`、`mute`、`poke`
//! （2026-10-10 同一个端点、同一个办法）：十九件一起时四件的边际份量 70、36、80、46，合计 232 个 token、825 字节；预算是实测加
//! 一成，255 个 token，合 910 字节。施工 O-33 加 `fetch_media`（2026-10-11 同一个端点、同一个办法）：二十件一起时边际份量 114，
//! 五件合计 346 个 token、1206 字节；预算是实测加一成，381 个 token，合 1327 字节。施工 O-48 给 `recall`、`mute` 加参数（2026-10-17
//! 同一个端点、同一个办法，二十三件一起）：五件合计 380 个 token、1302 字节，还在预算里。施工 O-54 加 `avatar`、
//! `members`（2026-10-17 同一个端点、同一个办法，二十五件一起）：边际份量 84、55，七件合计 519 个 token、1795 字节；预算是实测
//! 加一成，571 个 token，合 1974 字节。施工 O-55 加 `mention`（二十六件一起）：边际份量 66，八件合计 585 个 token、2026 字节；
//! 预算 643 个 token，合 2228 字节。施工 O-56 加 `chat_send`（二十七件一起）：边际份量 86，九件合计 671 个 token、2294 字节；
//! 预算 738 个 token，合 2523 字节。加工具、改说明超了，重新量过再改这里和设计。

use std::path::Path;

/// 预算：字节，回车 `\r` 不算（Windows 上检出的可能多出回车）。
const BUDGET: usize = 2523;

/// 桥的工具：一件一份说明。
const TOOLS: [&str; 9] = [
    "avatar",
    "chat_send",
    "fetch_media",
    "members",
    "mention",
    "mute",
    "poke",
    "recall",
    "skip_reply",
];

#[test]
fn the_bridge_tool_face_stays_within_its_budget() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../im/data/tools");
    let mut total = 0;
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("读得了工具说明的目录") {
        let path = entry.expect("读得了目录项").path();
        let bytes = std::fs::read(&path).expect("读得了说明");
        total += bytes.iter().filter(|byte| **byte != b'\r').count();
        names.push(
            path.file_stem()
                .expect("有名字")
                .to_string_lossy()
                .into_owned(),
        );
    }
    names.sort();
    assert_eq!(names, TOOLS, "一件工具一份说明");
    assert!(total <= BUDGET, "说明合计 {total} 字节，超过预算 {BUDGET}");
}
