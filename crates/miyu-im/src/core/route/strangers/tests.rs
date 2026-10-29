//! 陌生人的私聊开过的回合：照场所记，一天以前的扔掉。

use miyu_client::time::Timestamp;

use super::Strangers;

fn at(millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(millis).expect("是个时刻")
}

#[test]
fn turns_are_kept_per_venue_for_a_day() {
    let mut strangers = Strangers::default();
    strangers.opened("qq:30003:private:1", at(1_000));
    strangers.opened("qq:30003:private:2", at(2_000));
    strangers.opened("qq:30003:private:1", at(3_000));
    assert_eq!(strangers.of("qq:30003:private:1"), [at(1_000), at(3_000)]);
    assert_eq!(strangers.of("qq:30003:private:3"), []);
    strangers.opened("qq:30003:private:1", at(1_000 + super::DAY + 1));
    assert_eq!(
        strangers.of("qq:30003:private:1"),
        [at(3_000), at(1_000 + super::DAY + 1)],
        "一天以前的扔掉"
    );
}
