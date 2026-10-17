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
