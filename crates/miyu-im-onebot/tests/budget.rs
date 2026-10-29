//! 桥的工具面预算（`docs/designs/10-自带软件.md` 第九节，施工 O-26、O-31、O-33）：`miyu-im/data/onebot/tools/` 下的说明加起来（施工 T-1 到 T-4 起都是驱动交的）
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
//! 预算 738 个 token，合 2523 字节。施工 Q-2 加 `kick`、给 `mute` 加 `reason`（2026-10-20 同一个端点、同一个办法，驱动的十件一起）：
//! 边际份量 101、115（`mute` 原来 95），十件合计 1002 个 token、2762 字节；预算是实测加一成，1102 个 token，合 3038 字节。施工 Q-4
//! 加 `moderation_log`（同一个端点、同一个办法，十一件一起）：第一稿 124，说明删到一句、参数不写说明以后 96；十一件合计 1098 个
//! token、3071 字节，预算是实测加一成，1208 个 token，合 3378 字节。施工 I-12 工具名统一加 `qq_`（2026-10-20 项目主人定；`chat_send`
//! 改回 `qq_send`）：十一件合计 1115 个 token（多 17），说明字节不变；同一张加 `qq_contacts`、`qq_group_files`、`qq_react`（边际
//! 89、90、89），十四件合计 1383 个 token、3920 字节（I-17 补参数以后 1508、4292，还在预算以内），预算是实测加一成，1521 个 token，合 4312 字节。施工 I-20 踢人去掉 `confirm`、加 `qq_activity_ranking`（2026-10-21 项目主人定要做；边际 88，`qq_kick` 131）：十五件合计 1587 个 token、4514 字节，超了；照 `docs/工具说明怎么写.md` 改短以后 1415 个 token、3914 字节，预算不动。施工 I-31 加 `qq_quote`（2026-10-23 同一个端点、同一个办法，十六件一起）：边际 58，说明合计 4100 字节，还在预算以内。施工 I-32 给 `qq_poke` 加 `user`：多 13，说明合计 4139 字节。施工 I-36 加 `qq_set_card`（十七件一起）：边际 54，第一稿超了预算，改短以后说明合计 4301 字节。施工 I-37 加 `qq_notices`（十八件一起，边际 30），禁言、踢人的 `reason` 不写说明、一句改短，合计 4307 字节。施工 C-1 下加 `qq_chat_history`（十九件一起，边际 80）超了 266 字节，照写法把调用以后才用得上的挪进结果、`limit` 不给，合计 4294 字节。加工具、改说明超了，重新量过
//! 再改这里和设计。

use std::path::Path;

/// 预算：字节，回车 `\r` 不算（Windows 上检出的可能多出回车）。
const BUDGET: usize = 4312;

/// 桥的工具：一件一份说明。
const TOOLS: [&str; 19] = [
    "qq_activity_ranking",
    "qq_avatar",
    "qq_chat_history",
    "qq_contacts",
    "qq_fetch_media",
    "qq_group_files",
    "qq_kick",
    "qq_members",
    "qq_mention",
    "qq_moderation_log",
    "qq_mute",
    "qq_notices",
    "qq_poke",
    "qq_quote",
    "qq_react",
    "qq_recall",
    "qq_send",
    "qq_set_card",
    "qq_skip_reply",
];

#[test]
fn the_bridge_tool_face_stays_within_its_budget() {
    // 施工 T-1 到 T-4 起都是驱动交的（`onebot/tools/`）：工具面是桥交的整套。
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../miyu-im/data");
    let mut total = 0;
    let mut names = Vec::new();
    let entries = ["onebot/tools"]
        .into_iter()
        .flat_map(|dir| std::fs::read_dir(data.join(dir)).expect("读得了工具说明的目录"));
    for entry in entries {
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
