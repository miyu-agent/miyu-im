//! 挂接点记谁挂在哪（施工 O-43 上）：在跑、导出了桥认的名字的才算；不在跑的、缺了依赖的、没导出的、导出别的名字的不算；照编号排。

use serde_json::json;

use super::*;

#[test]
fn only_running_packages_exporting_a_known_point_are_hooked() {
    let packages = json!([
        {"package": "zeta", "status": "running", "export": {"methods": ["qq.event", "other"]}},
        {"package": "alpha", "status": "running", "export": {"methods": ["qq.event"]}},
        {"package": "stopped", "status": "stopped", "export": {"methods": ["qq.event"]}},
        {"package": "missing", "status": "depends_missing", "export": {"methods": ["qq.event"]}},
        {"package": "plain", "status": "running"},
        {"package": "elsewhere", "status": "running", "export": {"methods": ["qq.action"]}},
        {"status": "running", "export": {"methods": ["qq.event"]}},
    ]);
    let hooked = table(&packages);
    assert_eq!(hooked_at(&hooked, EVENT), ["alpha", "zeta"]);
    assert_eq!(hooked_at(&hooked, "qq.action"), Vec::<String>::new());
    assert_eq!(hooked.len(), 2);
    assert_eq!(points(&json!(null)), Vec::<&str>::new());
    assert_eq!(
        points(&json!({"status": "running", "export": {"methods": "qq.event"}})),
        Vec::<&str>::new()
    );
}

#[test]
fn a_pushed_entry_hooks_or_unhooks_that_package_only() {
    let mut hooked = table(&json!([
        {"package": "alpha", "status": "running", "export": {"methods": ["qq.event"]}},
        {"package": "beta", "status": "running", "export": {"methods": ["qq.event"]}},
    ]));
    let running = json!({"status": "running", "export": {"methods": ["qq.event"]}});
    change(&mut hooked, "gamma", &running);
    assert_eq!(hooked_at(&hooked, EVENT), ["alpha", "beta", "gamma"]);
    change(
        &mut hooked,
        "alpha",
        &json!({"status": "stopped", "export": {"methods": ["qq.event"]}}),
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
