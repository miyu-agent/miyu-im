//! im 的插件（施工 I-8 上）：声明怎么读（认不出的键、能力不收，`default` 不写的开），在一个平台上能不能用（只在别的平台上用的、
//! 缺能力的不能），场所开关的先后（这个场所的卡、兜底卡、声明的默认）。

use std::collections::BTreeMap;

use miyu_chat::VenueKind;

use super::{Plugins, Toggles, Unusable, declared};
use crate::chat::{CAPS, Cap, Id, Platform};
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
    // 施工 I-51：只在几种场所用；不写的都行，认不出的种类整份不收。
    assert!(bare.venues.is_empty() && bare.fits("group") && bare.fits("private"));
    let group =
        declared::read("[plugin]\nname = { en = \"x\" }\nvenues = [\"group\"]\n").expect("读得出");
    assert!(group.fits("group") && !group.fits("private"));
    assert!(declared::read("[plugin]\nname = { en = \"x\" }\nvenues = [\"channel\"]\n").is_err());
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
        id: Id::from(1),
        set: GroupSet {
            plugins: switches(&[("admin", true)]),
            ..GroupSet::default()
        },
    });
    cards.other_groups.plugins = switches(&[("admin", false), ("memes", false)]);
    cards.privates.push(PrivateCard {
        id: Id::from(9),
        set: PrivateSet {
            plugins: switches(&[("memes", true)]),
            ..PrivateSet::default()
        },
    });
    let toggles = Toggles::from_cards(&cards);
    let ids: Vec<Id> = (0..10).map(Id::from).collect();
    let group = |number: usize| Some((VenueKind::Group, &ids[number]));
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
    let private = |number: usize| Some((VenueKind::Private, &ids[number]));
    assert!(toggles.enabled(private(9), "memes", false));
    assert!(
        toggles.enabled(private(8), "admin", true),
        "私聊不照陌生群卡"
    );
    assert!(!toggles.enabled(None, "admin", false), "认不出场所的照默认");
}

#[test]
fn ui_declarations_read_and_bad_ones_are_refused() {
    // 施工 I-8 下：首页卡片、浮窗一页、整页。
    let text = "[plugin]\nname = { en = \"x\" }\n[plugin.ui]\nhome = true\npanel = true\n[[plugin.ui.pages]]\nid = \"library\"\ntitle = { zh = \"图库\" }\nmethod = \"memes.library\"\n";
    let declared = declared::read(text).expect("读得出");
    assert!(declared.ui.home && declared.ui.panel);
    assert_eq!(declared.ui.pages[0].id, "library");
    assert_eq!(declared.ui.pages[0].method, "memes.library");
    let bare = declared::read("[plugin]\nname = { en = \"x\" }\n").expect("读得出");
    assert_eq!(bare.ui, declared::Ui::default());
    let page = |id: &str, title: &str, method: &str| {
        format!(
            "[[plugin.ui.pages]]\nid = \"{id}\"\ntitle = {{ en = \"{title}\" }}\nmethod = \"{method}\"\n"
        )
    };
    let head = "[plugin]\nname = { en = \"x\" }\n";
    let many: String = (0..=declared::PAGES)
        .map(|n| page(&format!("p{n}"), "T", "m"))
        .collect();
    let wrongs = [
        ("编号大写", page("Lib", "T", "m")),
        ("编号太长", page(&"a".repeat(33), "T", "m")),
        (
            "重了",
            format!("{}{}", page("a", "T", "m"), page("a", "T", "n")),
        ),
        ("方法名不对", page("a", "T", "Memes")),
        ("没有标题", page("a", " ", "m")),
        ("太多", many),
        ("认不出的键", "[plugin.ui]\nwidget = true\n".to_string()),
    ];
    for (why, tail) in wrongs {
        assert!(declared::read(&format!("{head}{tail}")).is_err(), "{why}");
    }
    assert!(
        declared::read(&format!(
            "{head}{}",
            page(&"a".repeat(32), "T", &"m".repeat(64))
        ))
        .is_ok()
    );
}
