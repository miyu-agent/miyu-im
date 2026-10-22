//! 复读（`chat.md` 第二条「守着它的」，施工 I-28）：指纹去掉空白、标点、大小写；别人说过同一句、首尾相接不超过窗口的是复读，
//! 正好窗口的算、多一毫秒的不算；中间夹着别的话不断；自己重复的、没有字的不算；终端管理员照样放行；后来有人跟着说的认得出。

use miyu_kernel::id::ExternalId;

use super::super::test_support::{PEOPLE, SECOND, at, ctx, judge, msg, plus};
use super::super::{Outcome, Standing, Why};
use super::{Spoken, echo, echoed, repeats};

/// 窗口：两分钟。
const WINDOW: i64 = 120 * SECOND;

/// 号是 `n` 的人。
fn who(n: u32) -> ExternalId {
    ExternalId::parse(&format!("qq:{n}")).expect("合写法")
}

/// `n` 号在此刻之前 `ago` 毫秒说了 `text`。
fn spoken(n: u32, ago: i64, text: &str) -> Spoken {
    Spoken {
        sender: who(n),
        at: plus(at(12, 0).now, -ago),
        echo: echo(text),
        counted: true,
    }
}

/// 此刻 1 号说 `text`，之前的人话是 `earlier`（从新到旧）：是不是复读。
fn repeat(text: &str, earlier: &[Spoken]) -> bool {
    repeats(echo(text), &who(1), at(12, 0).now, earlier, WINDOW)
}

#[test]
fn the_fingerprint_ignores_spacing_punctuation_and_case() {
    assert_eq!(echo("miyu似了"), echo(" Miyu 似了！"));
    assert_eq!(echo("miyu似了"), echo("MIYU，似了。"));
    assert_ne!(echo("miyu似了"), echo("miyu没似"));
    assert_eq!(echo("？？？"), None, "只有标点的没有指纹");
    assert_eq!(echo(""), None);
}

#[test]
fn someone_else_saying_it_within_the_window_is_a_repeat() {
    assert!(repeat("miyu似了", &[spoken(2, 5 * SECOND, "miyu似了")]));
    assert!(
        repeat("miyu似了", &[spoken(2, WINDOW, "miyu似了")]),
        "正好窗口的算"
    );
    assert!(
        !repeat("miyu似了", &[spoken(2, WINDOW + 1, "miyu似了")]),
        "多一毫秒的不算"
    );
    assert!(
        !repeat("miyu似了", &[spoken(2, 5 * SECOND, "早")]),
        "说的不一样"
    );
}

#[test]
fn a_chain_reaches_back_link_by_link_and_other_talk_does_not_break_it() {
    let earlier = [
        spoken(3, 10 * SECOND, "miyu似了"),
        spoken(4, 30 * SECOND, "这是啥"),
        spoken(2, 100 * SECOND, "miyu似了"),
    ];
    assert!(repeat("miyu似了", &earlier), "中间夹着别的话不断");
    let chained = [
        spoken(1, 100 * SECOND, "miyu似了"),
        spoken(2, 200 * SECOND, "miyu似了"),
    ];
    assert!(
        repeat("miyu似了", &chained),
        "自己 100 秒前说过，那一条再往前 100 秒有别人说过：首尾相接"
    );
    let gap = [
        spoken(4, 130 * SECOND, "早"),
        spoken(2, 140 * SECOND, "miyu似了"),
    ];
    assert!(!repeat("miyu似了", &gap), "隔得太久断了");
}

#[test]
fn a_line_nobody_was_judged_on_does_not_make_a_repeat() {
    // 前面那一条睡着时只记下了：这一串谁都没判过，后面的照常判；它照样接得上更早判过的那一条。
    let unjudged = Spoken {
        counted: false,
        ..spoken(2, 5 * SECOND, "在吗")
    };
    assert!(!repeat("在吗", std::slice::from_ref(&unjudged)));
    assert!(repeat("在吗", &[unjudged, spoken(3, 100 * SECOND, "在吗")]));
}

#[test]
fn saying_it_again_yourself_or_saying_nothing_is_not_a_repeat() {
    assert!(!repeat("miyu似了", &[spoken(1, 5 * SECOND, "miyu似了")]));
    assert!(
        !repeat("？", &[spoken(2, 5 * SECOND, "？")]),
        "没有字的不算"
    );
}

#[test]
fn a_repeat_is_only_recorded_except_from_the_terminal_admin() {
    let mut repeated = ctx();
    repeated.repeated = true;
    for (standing, kind) in PEOPLE {
        let verdict = judge(&msg(standing, kind), &repeated, at(12, 0));
        let wanted = match standing {
            Standing::Admin => Outcome::Pass,
            _ => Outcome::RecordOnly(Why::Repeated),
        };
        assert_eq!(verdict.outcome, wanted, "{standing:?} {kind:?}");
    }
    assert_eq!(
        judge(
            &msg(Standing::Member, crate::VenueKind::Group),
            &ctx(),
            at(12, 0)
        )
        .outcome,
        Outcome::Pass
    );
}

#[test]
fn a_message_others_echoed_afterwards_is_known() {
    let later = [spoken(3, 0, "早"), spoken(2, 0, "Miyu似了！")];
    assert!(echoed(echo("miyu似了"), &who(1), &later));
    assert!(
        !echoed(echo("miyu似了"), &who(1), &[spoken(1, 0, "miyu似了")]),
        "自己再说的不算"
    );
    assert!(!echoed(echo("miyu似了"), &who(1), &[spoken(2, 0, "早")]));
    assert!(!echoed(None, &who(1), &[spoken(2, 0, "？")]));
}
