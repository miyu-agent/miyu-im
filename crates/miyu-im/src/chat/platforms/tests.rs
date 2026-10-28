//! 挂着的几个平台（施工 I-70 上）：照名字、照场所找；参数冲哪个平台来；找号不跨平台。

use std::sync::Arc;

use miyu_chat::Venue;
use miyu_client::ids::VenueId;
use serde_json::json;

use super::{Platforms, Unclaimed};
use crate::chat::fake::fake;
use crate::chat::{Bots, Platform};

const QQ: Platform = Platform::new("qq");
const TG: Platform = Platform::new("tg");

/// 号 `bots` 都连着（假连接）。
fn connected(bots: &[i64]) -> Arc<Bots> {
    let connected = Arc::new(Bots::default());
    for bot in bots {
        let (link, _asked) = fake();
        assert!(connected.insert(*bot, link).is_none());
    }
    connected
}

/// QQ 连着号 10、20，Telegram 连着号 5。
fn two() -> Platforms {
    Platforms::new(vec![(QQ, connected(&[20, 10])), (TG, connected(&[5]))])
}

fn venue(id: &str) -> Venue {
    Venue::parse(&VenueId::parse(id).expect("合写法")).expect("认得出")
}

#[test]
fn platforms_are_found_by_name_and_by_venue() {
    let platforms = two();
    assert_eq!(platforms.named("tg"), Some(TG));
    assert_eq!(platforms.named("qq"), Some(QQ));
    assert_eq!(platforms.named("discord"), None, "没挂的没有");
    assert_eq!(platforms.of_venue(&venue("tg:group:5")), Some(TG));
    assert_eq!(platforms.of_venue(&venue("qq:private:7")), Some(QQ));
    assert_eq!(platforms.of_venue(&venue("discord:group:5")), None);
    assert_eq!(platforms.first(), Some(QQ), "第一个驱动的");
    assert_eq!(Platforms::default().first(), None);
}

#[test]
fn args_name_their_platform_or_fall_to_the_only_one() {
    let two = two();
    let one = Platforms::new(vec![(QQ, connected(&[]))]);
    // 写了我们的：照写的。
    assert_eq!(two.ours(&json!({"platform": "tg"})), Ok(TG));
    assert_eq!(one.ours(&json!({"platform": "qq"})), Ok(QQ));
    // 写了别的：不收。
    assert_eq!(one.ours(&json!({"platform": "tg"})), Err(Unclaimed::Other));
    assert_eq!(two.ours(&json!({"platform": 1})), Err(Unclaimed::Other));
    // 没写、写 null：只挂了一个的算它。
    assert_eq!(one.ours(&json!({})), Ok(QQ));
    assert_eq!(one.ours(&json!({"platform": null})), Ok(QQ));
    // 挂了几个的不收，说缺。
    assert_eq!(two.ours(&json!({})), Err(Unclaimed::Missing));
    assert_eq!(
        two.ours(&json!({"platform": null})),
        Err(Unclaimed::Missing)
    );
}

#[test]
fn a_missing_bot_falls_back_within_its_own_platform() {
    let platforms = two();
    assert_eq!(platforms.sender(QQ, Some(20)), Some(20), "连着的照它");
    assert_eq!(
        platforms.sender(QQ, Some(5)),
        Some(10),
        "号 5 是 Telegram 的：退回 QQ 连着的最小的"
    );
    assert_eq!(platforms.sender(QQ, None), Some(10));
    assert_eq!(platforms.sender(TG, Some(10)), Some(5), "不退到 QQ 的号");
    assert_eq!(platforms.sender(TG, None), Some(5));
    let only_qq = Platforms::new(vec![(QQ, connected(&[10])), (TG, connected(&[]))]);
    assert_eq!(only_qq.sender(TG, Some(10)), None, "这个平台没有连着的号");
    assert_eq!(only_qq.sender(TG, None), None);
    assert!(only_qq.link(TG, 10).is_none());
    assert!(only_qq.link(QQ, 10).is_some());
    assert!(
        only_qq.bots(Platform::new("discord")).first().is_none(),
        "没挂的平台一个号都没有"
    );
}
