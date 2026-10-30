//! 挂接点记谁挂在哪（施工 O-43 上）：在跑、导出了桥认的名字的才算；不在跑的、缺了依赖的、没导出的、导出别的名字的不算；照编号排。

use serde_json::json;

use super::*;

#[test]
fn only_running_packages_exporting_a_known_point_are_hooked() {
    let packages = json!([
        {"package": "zeta", "status": "running", "export": {"methods": ["chat.event", "other"]}},
        {"package": "alpha", "status": "running", "export": {"methods": ["chat.event"]}},
        {"package": "stopped", "status": "stopped", "export": {"methods": ["chat.event"]}},
        {"package": "missing", "status": "depends_missing", "export": {"methods": ["chat.event"]}},
        {"package": "plain", "status": "running"},
        {"package": "elsewhere", "status": "running", "export": {"methods": ["chat.action"]}},
        {"status": "running", "export": {"methods": ["chat.event"]}},
    ]);
    let hooked = table(&packages);
    assert_eq!(hooked_at(&hooked, EVENT), ["alpha", "zeta"]);
    assert_eq!(hooked_at(&hooked, "chat.action"), Vec::<String>::new());
    assert_eq!(hooked.len(), 2);
    assert_eq!(points(&json!(null)), Vec::<&str>::new());
    assert_eq!(
        points(&json!({"status": "running", "export": {"methods": "chat.event"}})),
        Vec::<&str>::new()
    );
}

#[test]
fn a_pushed_entry_hooks_or_unhooks_that_package_only() {
    let mut hooked = table(&json!([
        {"package": "alpha", "status": "running", "export": {"methods": ["chat.event"]}},
        {"package": "beta", "status": "running", "export": {"methods": ["chat.event"]}},
    ]));
    let running = json!({"status": "running", "export": {"methods": ["chat.event"]}});
    change(&mut hooked, "gamma", &running);
    assert_eq!(hooked_at(&hooked, EVENT), ["alpha", "beta", "gamma"]);
    change(
        &mut hooked,
        "alpha",
        &json!({"status": "stopped", "export": {"methods": ["chat.event"]}}),
    );
    change(&mut hooked, "gamma", &json!(null));
    assert_eq!(hooked_at(&hooked, EVENT), ["beta"]);
    change(
        &mut hooked,
        "beta",
        &json!({"status": "running", "export": {"methods": []}}),
    );
    assert_eq!(hooked_at(&hooked, EVENT), Vec::<String>::new());
    assert!(hooked.is_empty());
}

#[test]
fn the_heaviest_opinion_wins_and_names_the_first_package_giving_it() {
    let answers = |pairs: &[(&str, Steer)]| -> Vec<(String, Steer)> {
        pairs
            .iter()
            .map(|(package, steer)| ((*package).to_string(), *steer))
            .collect()
    };
    assert_eq!(heaviest(Vec::new()), Steered::pass());
    assert_eq!(
        heaviest(answers(&[("a", Steer::Pass), ("b", Steer::Pass)])),
        Steered::pass()
    );
    assert_eq!(
        heaviest(answers(&[
            ("a", Steer::Reply),
            ("b", Steer::Record),
            ("c", Steer::Record)
        ])),
        Steered {
            steer: Steer::Record,
            by: Some("b".to_string())
        }
    );
    assert_eq!(
        heaviest(answers(&[
            ("a", Steer::Pass),
            ("b", Steer::Reply),
            ("c", Steer::Pass)
        ])),
        Steered {
            steer: Steer::Reply,
            by: Some("b".to_string())
        }
    );
    for (route, steer) in [
        ("pass", Steer::Pass),
        ("reply", Steer::Reply),
        ("record", Steer::Record),
    ] {
        assert_eq!(Steer::read(&json!(route)), Some(steer));
        assert_eq!(steer.name(), route);
    }
    for wrong in [json!("Record"), json!(""), json!(1), json!(null)] {
        assert_eq!(Steer::read(&wrong), None, "{wrong}");
    }
}

#[test]
fn an_outbound_answer_keeps_changes_drops_or_is_wrong() {
    assert_eq!(Edit::read(&json!({})), Ok(Edit::Keep));
    assert_eq!(Edit::read(&json!(null)), Ok(Edit::Keep));
    assert_eq!(
        Edit::read(&json!({"text": "新的"})),
        Ok(Edit::Change("新的".to_string()))
    );
    assert_eq!(Edit::read(&json!({"drop": true})), Ok(Edit::Drop));
    for wrong in [
        json!({"text": ""}),
        json!({"text": "  "}),
        json!({"text": 1}),
        json!({"drop": false}),
        json!({"drop": "yes"}),
        json!({"drop": true, "text": "x"}),
        json!("x"),
        json!([]),
    ] {
        assert_eq!(Edit::read(&wrong), Err("bad_answer".to_string()), "{wrong}");
    }
}

#[test]
fn bonuses_add_up_and_are_held_within_one() {
    let pairs = |list: &[(&str, f64)]| -> Vec<(String, f64)> {
        list.iter()
            .map(|(package, bonus)| ((*package).to_string(), *bonus))
            .collect()
    };
    assert_eq!(
        summed(Vec::new()),
        Nudge {
            bonus: 0.0,
            by: Vec::new()
        }
    );
    assert_eq!(
        summed(pairs(&[("a", 0.3), ("b", 0.0), ("c", -0.1)])),
        Nudge {
            bonus: 0.3 - 0.1,
            by: vec!["a".to_string(), "c".to_string()]
        }
    );
    assert_eq!(summed(pairs(&[("a", 1.0), ("b", 0.7)])).bonus, 1.0);
    assert_eq!(summed(pairs(&[("a", -1.0), ("b", -0.7)])).bonus, -1.0);
    // 每个先夹：给 5 的只算 1，和 -1 抵掉。
    assert_eq!(summed(pairs(&[("a", 5.0), ("b", -1.0)])).bonus, 0.0);
}

/// 驱动带的一个插件（施工 I-70 中上的测试用）：编号 `id`，设置项的开头 `x_`，声明哪个平台都行。
struct Carried(&'static str);

impl crate::plugins::Builtin for Carried {
    fn id(&self) -> &'static str {
        self.0
    }

    fn declared(&self) -> crate::plugins::Declared {
        crate::plugins::declared::read("[plugin]\nname = { en = \"c\" }\n").expect("读得懂")
    }

    fn settings(&self) -> &'static str {
        "x_"
    }
}

/// 挂 QQ（能力齐全，带插件 `qq-carried`，总开关 `qq_on` 照 `qq`）、tg（什么能力都没有，带插件 `tg-carried`）两个驱动的挂接点；认得
/// 软件包插件 `needs-mute`（要禁言）、`tg-only`（只在 tg 上用）。
fn two_hooks(qq: Value, tg: Value) -> Hooks {
    let (writer, _) = tokio::io::duplex(1024);
    let writer: super::super::caller::Writer = Arc::new(tokio::sync::Mutex::new(Box::new(writer)));
    let caller = Caller::new(writer, super::super::caller::Waiting::new("t-".to_string()));
    let mounted =
        |name: &'static str, caps: &'static [Cap], id: &'static str, config: Value| Mounted {
            platform: Platform::new(name),
            caps,
            builtins: vec![Arc::new(Carried(id)) as Arc<dyn Builtin>],
            config: watch::channel(config.as_object().cloned().unwrap_or_default()).1,
        };
    let hooks = Hooks::new(
        caller,
        Duration::from_millis(1),
        vec![
            mounted("qq", &crate::chat::CAPS, "qq-carried", qq),
            mounted("tg", &[], "tg-carried", tg),
        ],
    );
    hooks.set_plugins(Plugins::new(BTreeMap::from([
        (
            "needs-mute".to_string(),
            crate::plugins::declared::read("[plugin]\nname = { en = \"m\" }\nneeds = [\"mute\"]\n"),
        ),
        (
            "tg-only".to_string(),
            crate::plugins::declared::read(
                "[plugin]\nname = { en = \"t\" }\nplatforms = [\"tg\"]\n",
            ),
        ),
    ])));
    hooks
}

#[tokio::test]
async fn a_plugin_is_usable_by_the_venues_platform() {
    // 施工 I-70 中上：能不能用照场所的平台的驱动的能力表、声明的平台算；没挂的平台都不算。
    let hooks = two_hooks(json!({}), json!({}));
    let (qq, tg, to) = (
        Platform::new("qq"),
        Platform::new("tg"),
        Some(To::Group(Id::from(1))),
    );
    assert!(hooks.allowed("needs-mute", qq, None, to.clone()));
    assert!(
        !hooks.allowed("needs-mute", tg, None, to.clone()),
        "tg 不会禁言"
    );
    assert!(hooks.allowed("tg-only", tg, None, to.clone()));
    assert!(!hooks.allowed("tg-only", qq, None, to.clone()));
    assert!(
        !hooks.allowed("needs-mute", Platform::new("wx"), None, to),
        "没挂的平台"
    );
}

#[tokio::test]
async fn a_venue_switch_is_looked_up_on_the_venues_platform() {
    // 施工 I-80：场所开关照场所的平台找卡：QQ 群 1 的卡关了 `needs-mute`，tg 群 1 不照它（tg 也不会禁言，换一个不要能力的插件看）。
    let hooks = two_hooks(json!({}), json!({}));
    let mut cards = crate::rules::Cards::default();
    cards.groups.push(crate::rules::GroupCard {
        platform: "qq".to_string(),
        bot: None,
        id: Id::from(1),
        set: crate::rules::GroupSet {
            plugins: Some(BTreeMap::from([("tg-only".to_string(), false)])),
            ..Default::default()
        },
    });
    cards.groups.push(crate::rules::GroupCard {
        platform: "tg".to_string(),
        bot: None,
        id: Id::from(2),
        set: crate::rules::GroupSet {
            plugins: Some(BTreeMap::from([("tg-only".to_string(), false)])),
            ..Default::default()
        },
    });
    hooks.set_toggles(crate::plugins::Toggles::from_cards(&cards));
    let tg = Platform::new("tg");
    assert!(
        hooks.allowed("tg-only", tg, None, Some(To::Group(Id::from(1)))),
        "QQ 群 1 的卡不管 tg 群 1"
    );
    assert!(!hooks.allowed("tg-only", tg, None, Some(To::Group(Id::from(2)))));
}

#[tokio::test]
async fn a_carried_plugin_follows_its_own_driver() {
    // 驱动带的插件只在带它的驱动的平台上开着，总开关照那个驱动的配置。
    let hooks = two_hooks(json!({"x_on": false}), json!({"x_on": true, "k": 1}));
    let (qq, tg, to) = (
        Platform::new("qq"),
        Platform::new("tg"),
        To::Private(Id::from(1)),
    );
    assert_eq!(hooks.active(tg, None, to.clone()), ["tg-carried"]);
    assert!(
        hooks.active(qq, None, to.clone()).is_empty(),
        "QQ 的总开关关着"
    );
    assert_eq!(
        hooks.plugins_on(tg, None, to.clone()),
        ["tg-carried", "tg-only"]
    );
    assert_eq!(hooks.plugins_on(qq, None, to), ["needs-mute"]);
    assert_eq!(hooks.driver_config(tg)["k"], 1);
    assert!(hooks.driver_config(Platform::new("wx")).is_empty());
    // 插件页照第一个驱动（QQ）算能不能用，设置项的键照带它的驱动的平台拼。
    let listed = hooks.listed();
    let carried = listed
        .iter()
        .find(|one| one["package"] == "tg-carried")
        .expect("认得");
    assert_eq!(carried["settings"], "im.tg.x_");
    let only = listed
        .iter()
        .find(|one| one["package"] == "tg-only")
        .expect("认得");
    assert_eq!(only["unusable"], "other_platforms");
}
