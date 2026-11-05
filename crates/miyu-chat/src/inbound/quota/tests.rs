//! 额度：会话满了、人满了都只记下，冲她来的也一样；终端管理员、白名单成员不管；`quota_full` 只看会话。

use super::super::test_support::{PEOPLE, at, ctx, judge, member, msg};
use super::super::{Ctx, Outcome, Why};
use super::{Quota, quota_full};

const REACHED: Outcome = Outcome::RecordOnly(Why::QuotaReached);

fn with(quota: Quota) -> Ctx {
    let mut ctx = ctx();
    ctx.quota = quota;
    ctx
}

#[test]
fn a_full_venue_or_a_full_person_records_only_even_when_addressed() {
    let mut addressed = member();
    addressed.said.addressed = true;
    for quota in [
        Quota {
            venue: true,
            person: false,
        },
        Quota {
            venue: false,
            person: true,
        },
        Quota {
            venue: true,
            person: true,
        },
    ] {
        assert_eq!(
            judge(&member(), &with(quota), at(12, 0)).outcome,
            REACHED,
            "{quota:?}"
        );
        assert_eq!(
            judge(&addressed, &with(quota), at(12, 0)).outcome,
            REACHED,
            "{quota:?}"
        );
    }
    assert_eq!(
        judge(&member(), &ctx(), at(12, 0)).outcome,
        Outcome::Pass,
        "都没满"
    );
}

#[test]
fn admin_and_whitelisted_are_not_limited() {
    let full = with(Quota {
        venue: true,
        person: true,
    });
    let expected = [
        Outcome::Pass,
        Outcome::Pass,
        Outcome::Pass,
        REACHED,
        REACHED,
    ];
    for ((standing, kind), expected) in PEOPLE.into_iter().zip(expected) {
        let sent = msg(standing, kind);
        assert_eq!(
            judge(&sent, &full, at(12, 0)).outcome,
            expected,
            "{standing:?} {kind:?}"
        );
    }
}

#[test]
fn quota_full_only_looks_at_the_venue() {
    assert!(quota_full(&with(Quota {
        venue: true,
        person: false
    })));
    assert!(!quota_full(&with(Quota {
        venue: false,
        person: true
    })));
    assert!(!quota_full(&ctx()));
}
