//! im 的插件（施工 I-8 上）：声明怎么读（认不出的键、能力不收，`default` 不写的开），在一个平台上能不能用（只在别的平台上用的、
//! 缺能力的不能），场所开关的先后（这个场所的卡、兜底卡、声明的默认）。

use std::collections::BTreeMap;

use miyu_chat::VenueKind;

use super::{Plugins, Toggles, Unusable, declared};
use crate::chat::{CAPS, Cap, Platform};
use crate::rules::{Cards, GroupCard, GroupSet, PrivateCard, PrivateSet};

/// 一份写得对的声明。
const GOOD: &str = r#"
[plugin]
name = { zh = "群管理", en = "Group admin" }
summary = { zh = "禁言、踢人、记下来" }
icon = "shield"
homepage = "https://example.invalid/admin"
readme = "README.md"
platforms = ["qq"]
needs = ["mute", "recall"]
default = false
"#;

#[test]
fn a_declaration_reads_with_its_links_and_needs() {
    let declared = declared::read(GOOD).expect("读得出");
    assert_eq!(declared.name["zh"], "群管理");
    assert_eq!(declared.summary.get("en"), None);
    assert_eq!(declared.links.icon.as_deref(), Some("shield"));
    assert_eq!(declared.links.readme.as_deref(), Some("README.md"));
    assert_eq!(declared.platforms, ["qq"]);
    assert_eq!(declared.needs, [Cap::Mute, Cap::Recall]);
    assert!(!declared.default);
    let bare = declared::read("[plugin]\nname = { en = \"x\" }\n").expect("只写名字也行");
    assert!(bare.default, "default 不写的开");
    assert!(bare.platforms.is_empty() && bare.needs.is_empty());
}

#[test]
fn a_wrong_declaration_is_refused_whole() {
    for (text, why) in [
        ("not toml [", "不是 TOML"),
        ("name = { en = \"x\" }", "少了 [plugin]"),
        ("[plugin]\nsummary = { en = \"x\" }", "少了 name"),
        ("[plugin]\nname = { en = \"  \" }", "名字是空的"),
        (
            "[plugin]\nname = { en = \"x\" }\nhooks = [\"inbound\"]",
            "认不出的键",
        ),
        (
            "[plugin]\nname = { en = \"x\" }\nneeds = [\"fly\"]",
            "认不出的能力",
        ),
        (
            "[plugin]\nname = { en = \"x\" }\ndefault = \"yes\"",
            "类型不对",
        ),
        (
            "[plugin]\nname = { en = \"x\" }\n[other]\na = 1",
            "多一张表",
        ),
    ] {
        assert!(declared::read(text).is_err(), "{why}");
    }
}

#[test]
fn a_plugin_is_usable_only_where_its_platform_and_needs_fit() {
    let qq = Platform::new("qq");
    let plugins = Plugins::new(BTreeMap::from([
        ("admin".to_string(), declared::read(GOOD)),
        (
            "broken".to_string(),
            Err("plugin.toml: not_found".to_string()),
        ),
        (
            "anywhere".to_string(),
            declared::read("[plugin]\nname = { en = \"x\" }\nneeds = [\"forward\"]\n"),
        ),
    ]));
    assert_eq!(plugins.usable("admin", qq, &CAPS), Ok(()));
    assert_eq!(
        plugins.usable("admin", Platform::new("tg"), &CAPS),
        Err(Unusable::OtherPlatforms)
    );
    assert_eq!(
        plugins.usable("admin", qq, &[Cap::Recall, Cap::Forward]),
        Err(Unusable::Missing(vec![Cap::Mute]))
    );
    assert_eq!(
        plugins.usable("anywhere", Platform::new("tg"), &[Cap::Forward]),
        Ok(())
    );
    assert_eq!(
        plugins.usable("nobody", qq, &CAPS),
        Err(Unusable::NotAPlugin)
    );
    assert!(matches!(
        plugins.usable("broken", qq, &CAPS),
        Err(Unusable::Broken(_))
    ));
    assert!(plugins.declared("admin").is_some() && plugins.declared("broken").is_none());
}

/// 开关表：`(包, 开没开)`。
fn switches(pairs: &[(&str, bool)]) -> Option<BTreeMap<String, bool>> {
    Some(
        pairs
            .iter()
            .map(|(package, on)| ((*package).to_string(), *on))
            .collect(),
    )
}

#[test]
fn a_venue_follows_its_card_then_the_fallback_card_then_the_default() {
    let mut cards = Cards::default();
    cards.groups.push(GroupCard {
        id: 1,
        set: GroupSet {
            plugins: switches(&[("admin", true)]),
            ..GroupSet::default()
        },
    });
    cards.other_groups.plugins = switches(&[("admin", false), ("memes", false)]);
    cards.privates.push(PrivateCard {
        id: 9,
        set: PrivateSet {
            plugins: switches(&[("memes", true)]),
            ..PrivateSet::default()
        },
    });
    let toggles = Toggles::from_cards(&cards);
    let group = |number| Some((VenueKind::Group, number));
    assert!(
        toggles.enabled(group(1), "admin", false),
        "这个群的卡上开着"
    );
    assert!(
        !toggles.enabled(group(2), "admin", true),
        "别的群照陌生群卡"
    );
    assert!(
        !toggles.enabled(group(1), "memes", true),
        "这个群的卡上没写，照陌生群卡"
    );
    assert!(
        toggles.enabled(group(1), "identity", true),
        "都没写的照默认"
    );
    assert!(!toggles.enabled(group(1), "identity", false));
    let private = |number| Some((VenueKind::Private, number));
    assert!(toggles.enabled(private(9), "memes", false));
    assert!(
        toggles.enabled(private(8), "admin", true),
        "私聊不照陌生群卡"
    );
    assert!(!toggles.enabled(None, "admin", false), "认不出场所的照默认");
}
