//! 插件页的一项（`super::merged`）：并上核心说的版本、开关、后台页、挂在哪、能力，核心没列的只有 im 的几格。

use serde_json::json;

use super::merged;

#[test]
fn a_plugin_takes_what_the_core_says_about_its_package() {
    let packages = json!([
        {"package": "other", "version": "9", "page": null},
        {"package": "admin", "version": "0.3.1", "author": "x", "from": "local", "enabled": true, "status": "running",
         "page": {"path": "page"}, "upstream": null, "export": {"methods": ["chat.outbound", "chat.event", "else"]}},
    ]);
    let extensions = json!([{"package": "admin", "state": "running", "reason": null,
        "capabilities": [{"id": "packages.call", "name": "调别的软件", "summary": "…"}], "unapproved": []}]);
    let plugin = json!({"package": "admin", "name": {"zh": "群管"}});
    assert_eq!(
        merged(plugin, &packages, &extensions),
        json!({
            "package": "admin", "name": {"zh": "群管"}, "version": "0.3.1", "author": "x", "from": "local",
            "enabled": true, "status": "running", "page": true, "points": ["chat.event", "chat.outbound"],
            "state": "running", "capabilities": [{"id": "packages.call", "name": "调别的软件", "summary": "…"}],
            "unapproved": [],
        })
    );
    let alone = json!({"package": "gone", "unusable": "broken"});
    assert_eq!(merged(alone.clone(), &packages, &json!(null)), alone);
    let pageless = merged(json!({"package": "other"}), &packages, &extensions);
    assert_eq!(
        pageless,
        json!({"package": "other", "version": "9", "points": []})
    );
}
