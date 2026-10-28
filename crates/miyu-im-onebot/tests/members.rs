//! 群成员的名字缓存（施工 O-22，`onebot.md` 第一条「群消息」第 5 条）：记下的取得到，过了记多久的取不到；按平台、按群分开（施工
//! I-70 上：两个平台同号的群不串）；
//! 再记一次盖掉原来的；`get_group_member_info` 回的群名片空白的取昵称。时刻是交进去的，不等。身份（施工 O-31，「平台工具（一）」
//! 第 7 条）照名字的记法记，`role` 只认三种。

use std::time::{Duration, Instant};

use serde_json::json;

use miyu_im::chat::Platform;
use miyu_im_onebot::onebot::{Members, PLATFORM as QQ, Rank, display_name, member_info, rank_of};

const TG: Platform = Platform::new("tg");

#[test]
fn a_name_lasts_as_long_as_it_is_kept_and_only_in_its_group() {
    let keep = Duration::from_secs(600);
    let start = Instant::now();
    let mut members = Members::new(keep);
    members.remember((QQ, 555, 20002), "小林".to_string(), start);
    assert_eq!(members.name((QQ, 555, 20002), start), Some("小林"));
    assert_eq!(
        members.name((QQ, 555, 20002), start + keep - Duration::from_millis(1)),
        Some("小林")
    );
    assert_eq!(
        members.name((QQ, 555, 20002), start + keep),
        None,
        "过了的不用"
    );
    assert_eq!(members.name((QQ, 666, 20002), start), None, "按群分开");
    assert_eq!(members.name((QQ, 555, 20003), start), None);
    members.remember((QQ, 555, 20002), "林".to_string(), start + keep);
    assert_eq!(
        members.name((QQ, 555, 20002), start + keep),
        Some("林"),
        "再记一次照新的"
    );
}

#[test]
fn two_platforms_with_the_same_numbers_do_not_mix() {
    let keep = Duration::from_secs(600);
    let start = Instant::now();
    let mut members = Members::new(keep);
    members.remember((QQ, 555, 20002), "小林".to_string(), start);
    assert_eq!(members.name((TG, 555, 20002), start), None, "按平台分开");
    members.remember((TG, 555, 20002), "Lin".to_string(), start);
    assert_eq!(members.name((QQ, 555, 20002), start), Some("小林"));
    assert_eq!(members.name((TG, 555, 20002), start), Some("Lin"));
    members.remember((QQ, 555, 20003), "小王".to_string(), start);
    let names = members.names_in(TG, 555, start);
    assert_eq!(names.len(), 1, "只有这个平台的：{names:?}");
    assert_eq!(names.get(&20002).map(String::as_str), Some("Lin"));
    members.ranked((QQ, 555, 20002), Rank::Owner, start);
    assert_eq!(members.rank((TG, 555, 20002), start), None);
    members.forget((TG, 555, 20002));
    assert_eq!(
        members.name((QQ, 555, 20002), start),
        Some("小林"),
        "忘的是那个平台的"
    );
    // 她自己的昵称、问没问过也按平台记。
    members.nicknamed((QQ, 30003), "Miyu".to_string(), start);
    assert_eq!(members.nickname((TG, 30003), start), None);
    assert!(members.ask_due((QQ, 30003), Some(555), start));
    assert!(
        members.ask_due((TG, 30003), Some(555), start),
        "别的平台的另算"
    );
    assert!(!members.ask_due((QQ, 30003), Some(555), start));
}

#[test]
fn the_member_info_asks_napcat_with_its_cache_and_reads_the_card_first() {
    assert_eq!(
        member_info(555, 20002),
        json!({"group_id": 555, "user_id": 20002, "no_cache": false})
    );
    assert_eq!(
        display_name(&json!({"card": "小林", "nickname": "lin"})).as_deref(),
        Some("小林")
    );
    assert_eq!(
        display_name(&json!({"card": "  ", "nickname": "lin"})).as_deref(),
        Some("lin")
    );
    assert_eq!(display_name(&json!({"card": "", "nickname": " "})), None);
    assert_eq!(display_name(&json!(null)), None);
}

#[test]
fn a_rank_lasts_like_a_name_and_only_three_roles_are_read() {
    let keep = Duration::from_secs(600);
    let start = Instant::now();
    let mut members = Members::new(keep);
    members.ranked((QQ, 555, 20002), Rank::Owner, start);
    assert_eq!(members.rank((QQ, 555, 20002), start), Some(Rank::Owner));
    assert_eq!(
        members.rank((QQ, 555, 20002), start + keep),
        None,
        "过了的不用"
    );
    assert_eq!(members.rank((QQ, 666, 20002), start), None, "按群分开");
    assert_eq!(members.name((QQ, 555, 20002), start), None, "身份不是名字");
    members.ranked((QQ, 555, 20002), Rank::Member, start + keep);
    assert_eq!(
        members.rank((QQ, 555, 20002), start + keep),
        Some(Rank::Member)
    );
    assert_eq!(rank_of(&json!({"role": "owner"})), Some(Rank::Owner));
    assert_eq!(rank_of(&json!({"role": "admin"})), Some(Rank::Admin));
    assert_eq!(rank_of(&json!({"role": "member"})), Some(Rank::Member));
    assert_eq!(rank_of(&json!({"role": "Owner"})), None);
    assert_eq!(rank_of(&json!({"card": "小林"})), None);
}
