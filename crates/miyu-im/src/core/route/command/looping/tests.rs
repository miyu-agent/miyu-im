//! 群里的 `/loop` 怎么认（施工 I-239）。

use super::*;

#[test]
fn bare_loop_is_on_off_is_off_and_the_rest_wants_usage() {
    assert_eq!(asked("/loop"), Some(Asked::On));
    assert_eq!(asked("  /loop  "), Some(Asked::On));
    assert_eq!(asked("/loop off"), Some(Asked::Off));
    assert_eq!(asked("/loop OFF"), Some(Asked::Off));
    assert_eq!(asked("/loop 5m 看看群里"), Some(Asked::Other));
    assert_eq!(asked("/loop 看看群里"), Some(Asked::Other));
    assert_eq!(asked("/looping"), None, "别的命令不算");
    assert_eq!(asked("/model"), None);
    assert_eq!(asked("loop"), None);
}
