//! 动不动得了：群主谁都动不了，管理员只有她是群主时动得了，别人（没身份的、普通成员）都动得了。

use miyu_im::chat::Rank;

use super::shielded;

#[test]
fn owners_are_shielded_and_admins_only_from_admins() {
    assert!(shielded(Some(Rank::Owner), false));
    assert!(shielded(Some(Rank::Owner), true), "群主谁都动不了");
    assert!(shielded(Some(Rank::Admin), false), "管理员动不了管理员");
    assert!(!shielded(Some(Rank::Admin), true), "群主动得了管理员");
    assert!(!shielded(Some(Rank::Member), false));
    assert!(!shielded(None, false));
}
