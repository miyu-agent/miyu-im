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
