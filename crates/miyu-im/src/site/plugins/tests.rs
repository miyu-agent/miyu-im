//! 插件页的一项（`super::merged`）：并上核心说的版本、开关、后台页、挂在哪、能力，核心没列的只有 im 的几格。

use serde_json::json;

use super::{builtin, merged};

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

#[test]
fn a_builtin_plugin_takes_its_switch_and_im_s_version() {
    // 施工 I-10 上：驱动带的插件不是软件包，总开关照配置，版本、作者照 im 的包。
    let plugin =
        json!({"package": "miyu-im-identity", "builtin": true, "settings": "im.qq.identity_"});
    let own = json!({"package": "miyu-im", "version": "0.8.0", "author": "miyu-agent", "from": "shipped"});
    let off = json!({"items": {"im.qq.identity_on": {"value": false}}});
    assert_eq!(
        builtin(plugin.clone(), &off, &own),
        json!({
            "package": "miyu-im-identity", "builtin": true, "settings": "im.qq.identity_", "version": "0.8.0",
            "author": "miyu-agent", "enabled": false, "from": "builtin", "points": ["chat.inbound"], "status": "stopped",
        })
    );
    let unset = builtin(plugin, &json!({"items": {}}), &json!(null));
    assert_eq!(unset["enabled"], true, "没写的开：{unset}");
    assert!(unset.get("version").is_none());
    let package = json!({"package": "dice", "version": "1"});
    assert_eq!(builtin(package.clone(), &off, &own), package);
}
