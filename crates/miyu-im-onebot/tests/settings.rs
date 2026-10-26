//! 桥用的配置（施工 O-20，`onebot.md` 第一条「怎么走」第 1 条、「施工时定的」第 38 条）：握手交来的 `config` 读成 NapCat 的
//! 端口、令牌；没交的、`null` 的、不是 0 到 65535 的整数的端口照清单的默认值；令牌是字的照它（去掉前后空白），别的是没有；推来
//! 的只换带了的键，别的键不认。清单的默认值照出厂的清单读（8301），读不出来的说是哪个文件。白名单成员（施工 O-23）：
//! `im.whitelist`（施工 O-27 改名，旧键 `im.trusted` 不认）字的列表照收，别的是空的。原来桥自己的网页的端口
//! `im.web` 随施工 O-28 下去掉：桥不认，照出厂的清单拼的配置项里写它报不认识的键（「施工时定的」第 170 条）。

use serde_json::{Map, Value, json};

use miyu_config::secret::Secret;
use miyu_im::settings::{
    Defaults, driver_config, merge_driver, recall_receipts, whitelist, whitelist_key,
};
use miyu_im_onebot::settings::{NAMES, Settings};

/// OneBot 驱动的配置项那一组（平台名，施工 I-3 补：键是 `im.qq.<名字>`）。
const QQ: &str = "qq";
use miyu_store::resources::ResourceRoot;

use miyu_config::Layer;
use miyu_config::problem::Code;

use crate::support::{TOKEN, admin, defaults, package_in, resources, temp_root};

/// 测试用的默认值：和出厂的不一样，看得出照的是它。
fn fallback() -> Defaults {
    Defaults {
        recall_receipts: vec!["x".to_string()],
        driver: keys(json!({"host": "127.0.0.1", "listen": 18301})),
        site: keys(json!({"port": 18302})),
    }
}

/// 宿主挑、驱动读的整条路（施工 I-3）：握手交来的 `config` 里驱动的那几项照 `defaults` 补齐，读成 OneBot 的两项。
fn handed(config: &Value, defaults: &Defaults) -> Settings {
    read(
        &driver_config(config, (QQ, NAMES), &defaults.driver),
        defaults,
    )
}

/// 驱动读宿主交来的那几项：端口读不出的照默认值。
fn read(driver: &Map<String, Value>, defaults: &Defaults) -> Settings {
    Settings::read(driver, |name| defaults.driver.get(name).cloned()).expect("有端口")
}

/// 令牌 `token`。
fn secret(token: &str) -> Option<Secret> {
    Some(Secret::new(token).expect("合写法"))
}

/// `keys` 当推来的变化。
fn keys(value: Value) -> Map<String, Value> {
    value.as_object().expect("是对象").clone()
}

#[test]
fn the_shipped_manifest_gives_the_default_port() {
    // 撤回回执的命令出厂只有清除上下文、清除记忆的（施工 O-47）。
    assert_eq!(
        defaults(),
        Defaults {
            recall_receipts: vec!["clear".to_string(), "clear-memory".to_string()],
            driver: keys(json!({
                "host": "0.0.0.0",
                "listen": 8301,
                "identity_on": true,
                "identity_admin": true,
                "identity_card": true,
                "identity_people": [],
                "admin_on": true,
                "admin_mute": true,
                "admin_mute_min": 1,
                "admin_mute_max": 43200,
                "admin_kick": true,
                "admin_records": true,
                "admin_keep": 500,
                "admin_sync_unmute": true,
                "admin_log": true,
                "admin_summary": true,
            })),
            site: keys(json!({"port": 8302})),
        },
        "令牌没写默认值，不放；识人术的四项出厂开着、映射空的（施工 I-10 上）；群管理出厂开着、禁言 1 分钟到 30 天（施工 Q-1）；页面服务的端口出厂 8302（施工 I-5 上）"
    );
}

#[test]
fn a_missing_manifest_or_default_is_named() {
    let dir = std::env::temp_dir().join(format!("miyu-im-defaults-{}", std::process::id()));
    let packages = dir.join("packages");
    std::fs::create_dir_all(&packages).expect("建得了");
    let error = Defaults::load(&package_in(dir.clone()), (QQ, NAMES)).expect_err("没有清单");
    assert!(error.contains("im"), "{error}");
    // 照出厂的清单写一份，去掉 `im.recall_receipts` 的默认值（驱动的配置项没写默认值的不放，不算错，施工 I-3）。
    let shipped =
        std::fs::read_to_string(resources().join("packages").join("im").join("package.toml"))
            .expect("读得到出厂的清单");
    let without = shipped.replace("default = [\"clear\", \"clear-memory\"]\n", "");
    assert_ne!(without, shipped, "真的去掉了");
    std::fs::create_dir_all(packages.join("im")).expect("建得了");
    std::fs::write(packages.join("im").join("package.toml"), without).expect("写得进");
    let error = Defaults::load(&package_in(dir.clone()), (QQ, NAMES)).expect_err("没写默认值");
    let file = std::path::Path::new("im")
        .join("package.toml")
        .display()
        .to_string();
    assert!(
        error.contains(&file) && error.contains("im.recall_receipts"),
        "{error}"
    );
    std::fs::write(packages.join("im").join("package.toml"), "not toml [").expect("写得进");
    let error = Defaults::load(&package_in(dir.clone()), (QQ, NAMES)).expect_err("读不成");
    assert!(error.contains(&file), "{error}");
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里，不影响测试。
    }
}

#[test]
fn the_handed_config_is_taken_as_it_comes() {
    let config = json!({"im.qq.host": "192.168.1.5", "im.qq.listen": 9000, "im.qq.token": TOKEN, "im.whitelist": ["qq:20017"]});
    assert_eq!(
        handed(&config, &fallback()),
        Settings {
            host: "192.168.1.5".parse().expect("是地址"),
            port: 9000,
            token: secret(TOKEN)
        }
    );
    assert_eq!(
        handed(&json!({"im.qq.token": format!("  {TOKEN}\n")}), &fallback()).token,
        secret(TOKEN),
        "前后的空白去掉"
    );
    assert_eq!(
        handed(&json!({"im.qq.host": " ::1\n"}), &fallback()).host,
        "::1".parse::<std::net::IpAddr>().expect("是地址"),
        "地址也去掉前后的空白，IPv6 也认"
    );
    assert_eq!(
        handed(&json!({"im.qq.listen": 0}), &fallback()).port,
        0,
        "0 也照它"
    );
    assert_eq!(
        handed(&json!({"im.qq.listen": 65535}), &fallback()).port,
        65535,
        "65535 也照它"
    );
}

#[test]
fn what_is_not_handed_falls_back_to_the_manifest() {
    let nothing = Settings {
        host: "127.0.0.1".parse().expect("是地址"),
        port: 18301,
        token: None,
    };
    for config in [
        json!({}),
        Value::Null,
        json!("not an object"),
        json!({"im.qq.listen": null, "im.qq.token": null}),
        json!({"im.qq.listen": 65536, "im.qq.token": ""}),
        json!({"im.qq.listen": -1}),
        json!({"im.qq.listen": "8301", "im.qq.token": 42}),
        json!({"im.qq.listen": 8302.5}),
        // 地址（施工 I-27）：不是 IP 地址的字、不是字的照默认值。
        json!({"im.qq.host": "localhost"}),
        json!({"im.qq.host": "999.1.1.1", "im.qq.listen": "x"}),
        json!({"im.qq.host": 42}),
        json!({"im.qq.host": ""}),
        json!({"listen": 9000, "web.port": 9001, "onebotx.token": TOKEN, "onebot.tokens": TOKEN}),
        // 原来桥自己的网页的端口（施工 O-28 下去掉）：不认。
        json!({"im.web": 9001}),
    ] {
        assert_eq!(handed(&config, &fallback()), nothing, "{config}");
    }
}

/// 宿主交给驱动的那一份（施工 I-3）：只有驱动的那几项，照名字放；没交的、`null` 的照默认值补上，默认值也没有的不放。
#[test]
fn the_driver_gets_its_own_keys_with_defaults_filled_in() {
    let defaults = fallback();
    assert_eq!(
        driver_config(&json!({}), (QQ, NAMES), &defaults.driver),
        keys(json!({"host": "127.0.0.1", "listen": 18301})),
        "没交的地址、端口照默认值，令牌没有默认值不放"
    );
    assert_eq!(
        driver_config(
            &json!({"im.qq.listen": null, "im.qq.token": TOKEN, "im.whitelist": ["qq:1"]}),
            (QQ, NAMES),
            &defaults.driver
        ),
        keys(json!({"host": "127.0.0.1", "listen": 18301, "token": TOKEN})),
        "别的键不给驱动"
    );
}

#[test]
fn a_push_changes_only_the_keys_it_carries() {
    let defaults = fallback();
    let mut driver = driver_config(
        &json!({"im.qq.listen": 9000, "im.qq.token": TOKEN}),
        (QQ, NAMES),
        &defaults.driver,
    );
    let pushed = |value: Value, driver: &mut Map<String, Value>| {
        merge_driver(driver, &keys(value), (QQ, NAMES), &defaults.driver)
    };
    assert!(pushed(json!({"im.qq.token": "new"}), &mut driver));
    assert_eq!(
        read(&driver, &defaults),
        Settings {
            host: "127.0.0.1".parse().expect("是地址"),
            port: 9000,
            token: secret("new")
        }
    );
    pushed(json!({"im.qq.listen": 9100}), &mut driver);
    assert_eq!(read(&driver, &defaults).port, 9100);
    pushed(json!({"im.qq.token": null}), &mut driver);
    assert_eq!(
        read(&driver, &defaults),
        Settings {
            host: "127.0.0.1".parse().expect("是地址"),
            port: 9100,
            token: None
        },
        "null 当没有：令牌没了"
    );
    pushed(json!({"im.qq.listen": null}), &mut driver);
    assert_eq!(
        read(&driver, &defaults).port,
        18301,
        "null 当没有：端口照默认值"
    );
    assert!(
        !pushed(
            json!({"im.whitelist": ["qq:1"], "web.port": 1, "im.web": 9001}),
            &mut driver
        ),
        "别的键不认，没变"
    );
    assert_eq!(
        read(&driver, &defaults),
        Settings {
            host: "127.0.0.1".parse().expect("是地址"),
            port: 18301,
            token: None
        }
    );
    pushed(json!({"im.qq.host": "10.0.0.2"}), &mut driver);
    assert_eq!(
        read(&driver, &defaults).host,
        "10.0.0.2".parse::<std::net::IpAddr>().expect("是地址"),
        "地址也照推来的换"
    );
}

#[test]
fn whitelist_is_a_list_of_text() {
    assert_eq!(whitelist_key(), "im.whitelist");
    let handed = json!({"im.whitelist": ["qq:20003", 7, null, "qq:20005"]});
    assert_eq!(
        whitelist(&handed[whitelist_key()]),
        ["qq:20003", "qq:20005"],
        "列表里不是字的不要"
    );
    for nothing in [
        json!({}),
        json!({"im.whitelist": null}),
        json!({"im.whitelist": "qq:1"}),
        json!({"im.trusted": ["qq:20003"]}),
    ] {
        assert!(whitelist(&nothing[whitelist_key()]).is_empty(), "{nothing}");
    }
}

#[test]
fn the_platform_and_page_keys_are_grouped() {
    // 照真核心起来时那样把出厂的包的配置项拼进来（`Packaged`，同 `support::Home`）。包的程序不在主程序旁边的当没装、配置项不拼
    // （`package-pages.md`「程序不在就当没装」）：先把 `miyu-im` 链到测试程序旁边（同 `Home::spawning`）。
    crate::support::spawning::linked();
    let (dir, root) = temp_root();
    let shipped = ResourceRoot::at(resources());
    let mut found = miyu_endpoint::packages::load(&shipped, &root, &admin());
    let listed = miyu_core::settings::Packaged::of(&mut found).all();
    let codes = |source: &str| -> Vec<(Code, Option<String>)> {
        miyu_config::parse::parse(&listed, Layer::System, source)
            .expect("写法对")
            .problems
            .into_iter()
            .map(|problem| (problem.code, problem.key))
            .collect()
    };
    assert_eq!(
        codes("[im.qq]\nlisten = 8301\n[im.web]\nport = 8302\n"),
        Vec::<(Code, Option<String>)>::new(),
        "NapCat 的端口在平台那一组、页面服务的在 web 那一组（施工 I-3 补）"
    );
    assert_eq!(
        codes("[im]\nlisten = 8301\nweb_port = 8302\n"),
        [
            (Code::UnknownKey, Some("im.listen".to_string())),
            (Code::UnknownKey, Some("im.web_port".to_string()))
        ],
        "没分组的旧键不认（没发过版，不迁移）"
    );
    if std::fs::remove_dir_all(&dir).is_err() {
        // 删不掉就留在临时目录里，不影响测试。
    }
}

#[test]
fn the_manifest_exports_only_the_methods_for_other_packages() {
    // 施工 O-42：别的包只调得到 `chat.action`、`chat.venue`（O-56 多 `chat.send`，I-39 多 `chat.media`）；后台页的两个不导出。
    let text = std::fs::read_to_string(package_in(resources()).dir().join("package.toml"))
        .expect("读得了");
    let manifest = miyu_client::manifest::read(&text).expect("读得出");
    assert_eq!(
        manifest.export,
        [
            "chat.action",
            "chat.venue",
            "chat.media",
            "chat.send",
            "chat.note"
        ]
    );
}

#[test]
fn setting_keys_read_as_their_names_in_the_language() {
    // 施工 O-41 补二：`miyu im status` 说没认出来的配置项用名字；清单里没有的照键。
    let package = package_in(resources());
    let keys = [
        "im.sleep".to_string(),
        "im.keywords".to_string(),
        "onebot.nope".to_string(),
        "other.sleep".to_string(),
    ];
    assert_eq!(
        miyu_im::settings::labels(&package, &keys, "zh"),
        ["睡觉时间", "群里的触发词", "onebot.nope", "other.sleep"]
    );
    assert_eq!(
        miyu_im::settings::labels(&package, &keys[..1], "en"),
        ["Sleep hours"]
    );
    let missing = package_in(std::env::temp_dir().join("miyu-im-no-such-resources"));
    assert_eq!(
        miyu_im::settings::labels(&missing, &keys[..1], "zh"),
        ["im.sleep"]
    );
}

#[test]
fn the_package_folder_is_named_after_the_package() {
    // 施工 O-44：核心照文件夹的名字定包的编号，仓库里的包文件夹一定叫 `onebot`，从源码 `miyu pkg install ./onebot` 装出来
    // 配置项的键才是 `onebot.*`（原来叫 `package/`，装出来编号是 `package`）。
    let dir = crate::support::assembled::package_dir()
        .canonicalize()
        .expect("在");
    assert_eq!(
        dir.file_name().and_then(|name| name.to_str()),
        Some(miyu_im::PACKAGE)
    );
    assert!(dir.join("package.toml").is_file());
}

#[test]
fn recall_receipts_fall_back_to_the_default_and_take_names_with_a_slash() {
    // 施工 O-47：推来 `null`、没交、不是列表的回到清单的默认值；写了的照写的，开头的 `/` 去掉，不是字的不要。
    let defaults = vec!["clear".to_string(), "clear-memory".to_string()];
    for value in [Value::Null, json!("clear"), json!({"a": 1})] {
        assert_eq!(recall_receipts(&value, &defaults), defaults, "{value}");
    }
    assert_eq!(
        recall_receipts(&json!(["/stop", " goal ", 3]), &defaults),
        ["stop", "goal"]
    );
    assert!(
        recall_receipts(&json!([]), &defaults).is_empty(),
        "空的就是都不撤"
    );
}

#[test]
fn without_a_usable_address_anyone_can_connect() {
    // 交来的、清单的都读不出地址：照旧 Miyu 听 0.0.0.0（施工 I-27）。
    let read =
        Settings::read(&keys(json!({"host": "nope", "listen": 9000})), |_| None).expect("有端口");
    assert_eq!(
        read.host,
        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
    );
}
