//! 什么时候重读场所规则（施工 O-21，`onebot.md` 第一条「场所规则和出厂数据」第 3 条）：系统的规则文件、违规词表改了、加了、
//! 删了，隔够 `rules_check_millis` 的下一次用照新的，没隔够的照旧；这一刻的样子（文件列表、修改时刻、大小）没变的不重读。
//! 钟是交进去的时刻，测试不等。

use std::time::{Duration, Instant};

use miyu_im::rules::{Cards, Configured, Venues};

use crate::rules::{FACTORY_WORDS, clean, group, system_rule, system_words, text};
use crate::support::*;

#[test]
fn a_changed_system_file_is_read_at_the_next_use_after_the_interval() {
    let (dir, root) = temp_root();
    let every = Duration::from_secs(1);
    let start = Instant::now();
    let at = |millis| start + Duration::from_millis(millis);
    let mut venues = Venues::new(factory(), &root, Configured::default(), every, start);
    let sleep = |venues: &mut Venues, millis| {
        venues
            .current(at(millis))
            .at(&group("1"))
            .resolved
            .entries
            .get("sleep")
            .map(|entry| entry.value.clone())
    };
    assert_eq!(sleep(&mut venues, 0), None);
    let pushed = serde_json::json!({"im.sleep": "23:00-07:00"});
    venues.configure(pushed.as_object().expect("对象"));
    assert_eq!(sleep(&mut venues, 1), Some(text("23:00-07:00")), "不用等");
    system_rule(
        &root,
        "55-before.toml",
        "[[rule]]\nsleep = \"01:00-02:00\"\n",
    );
    assert_eq!(
        sleep(&mut venues, 1000),
        Some(text("23:00-07:00")),
        "排在前面的被盖"
    );
    system_rule(
        &root,
        "80-after.toml",
        "[[rule]]\nsleep = \"03:00-04:00\"\n",
    );
    assert_eq!(
        sleep(&mut venues, 2000),
        Some(text("03:00-04:00")),
        "排在后面的盖它"
    );
    // 推来别的键、同样的值：不重读，照手里的。
    let same = serde_json::json!({"im.qq.listen": 9000, "im.sleep": "23:00-07:00"});
    venues.configure(same.as_object().expect("对象"));
    assert_eq!(sleep(&mut venues, 2001), Some(text("03:00-04:00")));
    clean(&dir);
}

#[test]
fn an_unchanged_listing_is_not_read_again() {
    let (dir, root) = temp_root();
    let path = system_rule(&root, "80-x.toml", "[[rule]]\nrate = \"30/60s\"\n");
    let start = Instant::now();
    let mut venues = Venues::new(
        factory(),
        &root,
        Configured::default(),
        Duration::from_secs(1),
        start,
    );
    let modified = std::fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .expect("有修改时刻");
    // 一样长、修改时刻改回去：这一刻的样子和上一次的一样，不重读，照手里的。
    std::fs::write(&path, "[[rule]]\nrate = \"40/60s\"\n").expect("写得进");
    std::fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(modified))
        .expect("改得了修改时刻");
    let loaded = venues.current(start + Duration::from_secs(5));
    assert_eq!(
        loaded.at(&group("1")).resolved.entries["rate"].value,
        text("30/60s")
    );
    // 长短变了：重读。
    std::fs::write(&path, "[[rule]]\nrate = \"400/60s\"\n").expect("写得进");
    std::fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(modified))
        .expect("改得了修改时刻");
    let loaded = venues.current(start + Duration::from_secs(10));
    assert_eq!(
        loaded.at(&group("1")).resolved.entries["rate"].value,
        text("400/60s")
    );
    clean(&dir);
}

#[test]
fn a_changed_word_list_is_read_at_the_next_use_too() {
    let (dir, root) = temp_root();
    let start = Instant::now();
    let mut venues = Venues::new(
        factory(),
        &root,
        Configured::default(),
        Duration::from_secs(1),
        start,
    );
    assert_eq!(venues.current(start).keywords.len(), FACTORY_WORDS);
    let path = system_words(&root, "foo\n");
    assert_eq!(
        venues.current(start + Duration::from_secs(1)).keywords,
        ["foo"]
    );
    std::fs::remove_file(&path).expect("删得了");
    assert_eq!(
        venues
            .current(start + Duration::from_secs(2))
            .keywords
            .len(),
        FACTORY_WORDS
    );
    clean(&dir);
}

#[test]
fn pushed_settings_apply_at_once_and_sit_between_system_file_names() {
    // 配置项那一层（施工 O-41）：推来变了不等隔够；名字排在 `60-settings.toml` 前面的系统文件被它盖，后面的盖它。
    let (dir, root) = temp_root();
    let start = Instant::now();
    let at = |millis| start + Duration::from_millis(millis);
    let mut venues = Venues::new(
        factory(),
        &root,
        Configured::default(),
        Duration::from_secs(1),
        start,
    );
    let sleep = |venues: &mut Venues, millis| {
        venues
            .current(at(millis))
            .at(&group("1"))
            .resolved
            .entries
            .get("sleep")
            .map(|entry| entry.value.clone())
    };
    assert_eq!(sleep(&mut venues, 0), None);
    let pushed = serde_json::json!({"im.sleep": "23:00-07:00"});
    venues.configure(pushed.as_object().expect("对象"));
    assert_eq!(sleep(&mut venues, 1), Some(text("23:00-07:00")), "不用等");
    system_rule(
        &root,
        "55-before.toml",
        "[[rule]]\nsleep = \"01:00-02:00\"\n",
    );
    assert_eq!(
        sleep(&mut venues, 1000),
        Some(text("23:00-07:00")),
        "排在前面的被盖"
    );
    system_rule(
        &root,
        "80-after.toml",
        "[[rule]]\nsleep = \"03:00-04:00\"\n",
    );
    assert_eq!(
        sleep(&mut venues, 2000),
        Some(text("03:00-04:00")),
        "排在后面的盖它"
    );
    // 推来别的键、同样的值：不重读，照手里的。
    let same = serde_json::json!({"im.qq.listen": 9000, "im.sleep": "23:00-07:00"});
    venues.configure(same.as_object().expect("对象"));
    assert_eq!(sleep(&mut venues, 2001), Some(text("03:00-04:00")));
    clean(&dir);
}

#[test]
fn saved_cards_apply_at_once_and_sit_after_the_settings() {
    // 卡片那一层（施工 O-58 上）：换了不等隔够；排在配置项那一份后面，系统里名字排在后面的照旧盖它。
    let (dir, root) = temp_root();
    let start = Instant::now();
    let at = |millis| start + Duration::from_millis(millis);
    let mut venues = Venues::new(
        factory(),
        &root,
        Configured::default(),
        Duration::from_secs(1),
        start,
    );
    let rate = |venues: &mut Venues, millis, number| {
        venues
            .current(at(millis))
            .at(&group(number))
            .resolved
            .entries["rate"]
            .value
            .clone()
    };
    let allow = |venues: &mut Venues, number| {
        venues.current(at(0)).at(&group(number)).resolved.entries["allow"]
            .value
            .clone()
    };
    // 出厂没有卡片：「其他群」关着。
    assert_eq!(allow(&mut venues, "1"), miyu_config::Value::Bool(false));
    let cards: Cards = serde_json::from_value(serde_json::json!({
        "other_groups": {"rate": "1/60s"},
        "groups": [{"id": 1, "set": {"rate": "2/60s"}}, {"id": 3}],
    }))
    .expect("卡片的样子");
    venues.set_cards(cards.clone());
    assert_eq!(rate(&mut venues, 1, "1"), text("2/60s"), "不用等");
    assert_eq!(allow(&mut venues, "1"), miyu_config::Value::Bool(true));
    assert_eq!(
        rate(&mut venues, 1, "2"),
        text("1/60s"),
        "名单以外的照「其他群」"
    );
    assert_eq!(
        rate(&mut venues, 1, "3"),
        text("30/60s"),
        "名单里没写的不继承「其他群」：照出厂群规则的 30/60（施工 I-16）"
    );
    assert_eq!(venues.cards(), &cards);
    system_rule(&root, "80-after.toml", "[[rule]]\nrate = \"4/60s\"\n");
    assert_eq!(
        rate(&mut venues, 1000, "1"),
        text("4/60s"),
        "排在后面的系统文件盖它"
    );
    clean(&dir);
}
