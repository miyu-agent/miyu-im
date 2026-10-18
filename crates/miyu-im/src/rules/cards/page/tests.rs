//! 两页的内容：「其他群」在最前、每组小标题加表单、存的参数带卡、值是卡片上写的（管理的人只写号）、默认值写成一句；加一项的选项
//! 不列已经有卡的、私聊的带 `p:`；删了的人格照样在选项里；默认值带 `value`（类型对得上、下拉里有的才写）；太大的先换成一种语言、再去掉后面几项；仓库里的字三种语言都齐。

use serde_json::{Map, Value, json};

use super::*;

/// 仓库里的 `page.json`。
fn words() -> Words {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../onebot/data/cards/human/page.json"
    );
    Words::parse(&std::fs::read_to_string(path).expect("读得到")).expect("写法对")
}

fn cards(value: Value) -> Cards {
    serde_json::from_value(value).expect("是卡片的样子")
}

/// 她在两个群、两个好友，两个人格；默认值：群 7 的插嘴是偶尔、其他群不接话。
fn known() -> Known {
    Known {
        groups: vec![(7, "测试群".into()), (8, "另一个群".into())],
        friends: vec![(9, "lin".into()), (10, "mo".into())],
        personas: vec![
            ("miyu".into(), "Miyu".into()),
            ("engineer".into(), "工程师".into()),
        ],
        whitelist: vec![9],
        defaults: json!({
            "other_groups": {"on": {"value": false, "from": null}},
            "other_privates": {"on": {"value": false, "from": null}, "rate": {"value": "5/300s", "from": "factory"}},
            "groups": {"7": {"chatty": {"value": "some", "from": "factory"}, "managers": {"value": ["qq:1001"], "from": "setting"}, "persona": {"value": "miyu", "from": "system"}}},
            "privates": {"9": {"long_chars": {"value": 300, "from": "factory"}}},
        }),
    }
}

/// 一项里键是 `key` 的那一格。
fn field<'a>(item: &'a Value, key: &str) -> &'a Value {
    item["blocks"]
        .as_array()
        .expect("几段")
        .iter()
        .filter(|block| block["type"] == "form")
        .flat_map(|form| form["fields"].as_array().expect("几格"))
        .find(|field| field["key"] == key)
        .expect("有这一格")
}

#[test]
fn the_groups_page_has_other_groups_first_and_forms_per_group() {
    let set = cards(
        json!({"other_groups": {"on": true}, "groups": [{"id": 7, "set": {"on": false, "managers": ["qq:1002"], "persona": "gone"}}]}),
    );
    let page = groups_page(&set, &known(), &words(), "zh");
    let items = page[1]["items"].as_array().expect("几项");
    assert_eq!(page[1]["type"], "sections");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "other");
    assert_eq!(items[0]["title"]["zh"], "其他群");
    assert_eq!(items[0]["state"], "good", "其他群写了接话");
    assert!(
        !items[0]["blocks"].to_string().contains(REMOVE),
        "其他群删不掉"
    );
    let group = &items[1];
    assert_eq!(
        (
            group["id"].clone(),
            group["title"].clone(),
            group["subtitle"].clone()
        ),
        (json!("7"), json!("测试群"), json!("7"))
    );
    assert_eq!(group["state"], "off");
    let headings: Vec<&Value> = group["blocks"]
        .as_array()
        .expect("几段")
        .iter()
        .filter(|block| block["type"] == "heading")
        .collect();
    assert_eq!(headings.len(), 4, "四组");
    let form = group["blocks"]
        .as_array()
        .expect("几段")
        .iter()
        .find(|block| block["type"] == "form")
        .expect("有表单");
    assert_eq!(form["id"], "chat-7");
    assert_eq!(form["submit"]["call"], SAVE_ONE);
    assert_eq!(
        form["submit"]["params"],
        json!({"card": {"kind": "group", "id": 7}})
    );
    assert_eq!(
        field(group, "managers")["value"],
        json!(["1002"]),
        "管理的人只写号"
    );
    assert_eq!(field(group, "rate")["value"], Value::Null, "没写的是 null");
    assert_eq!(
        field(group, "chatty")["default"]["text"]["zh"],
        "默认 · 偶尔"
    );
    assert_eq!(
        field(group, "managers")["default"]["text"]["zh"],
        "默认 · 1001"
    );
    assert_eq!(
        field(group, "persona")["default"]["text"]["en"],
        "Default · Miyu"
    );
    assert_eq!(field(group, "rate")["default"]["text"]["zh"], "默认 · 不设");
    let personas = field(group, "persona")["options"]
        .as_array()
        .expect("选项")
        .clone();
    assert_eq!(
        personas.last().expect("有")["value"],
        "gone",
        "删了的人格照样放进去"
    );
    assert_eq!(field(group, "long_chars")["max"], 100_000);
    assert_eq!(
        page[1]["add"]["options"],
        json!([{"value": "8", "label": "另一个群（8）"}]),
        "有卡的不列"
    );
    assert_eq!(page[0]["items"][0]["action"], json!({"open": FRIENDS}));
}

#[test]
fn the_friends_page_lists_the_whitelist_and_offers_other_friends() {
    let set = cards(json!({"privates": [{"id": 9, "set": {"long_text": "split"}}, {"id": 11}]}));
    let page = friends_page(&set, &known(), &words(), "zh");
    let items = page[1]["items"].as_array().expect("几项");
    assert_eq!(
        items.len(),
        2,
        "「其他私聊」加白名单成员：不在白名单里的卡不列"
    );
    assert_eq!(items[0]["id"], "other", "「其他私聊」在最前面（施工 O-60）");
    assert_eq!(items[0]["state"], "off", "出厂不回复");
    assert!(!items[0]["blocks"].to_string().contains(REMOVE), "删不掉");
    assert_eq!(field(&items[0], "rate")["control"], "text");
    assert_eq!(items[1]["title"], "lin");
    assert!(items[1].get("state").is_none());
    assert_eq!(field(&items[1], "long_text")["value"], "split");
    assert_eq!(
        field(&items[1], "long_chars")["default"]["text"]["zh"],
        "默认 · 300"
    );
    assert_eq!(
        page[1]["add"]["options"],
        json!([{"value": "p:10", "label": "mo（10）"}])
    );
    let empty = friends_page(
        &Cards::default(),
        &Known {
            friends: vec![(9, "lin".into())],
            ..known()
        },
        &words(),
        "zh",
    );
    assert!(empty[1].get("add").is_none(), "没有可加的不写 add");
    assert_eq!(
        empty[1]["items"][1]["blocks"][1]["fields"][0]["value"],
        Value::Null,
        "没有卡的照空卡画"
    );
}

#[test]
fn managers_written_as_numbers_get_the_platform_prefix() {
    let values: Map<String, Value> =
        json!({"managers": ["1001", "qq:1002", "abc", ""], "rate": "1/60s"})
            .as_object()
            .expect("对象")
            .clone();
    let got = from_page(&values);
    assert_eq!(got["managers"], json!(["qq:1001", "qq:1002", "abc", ""]));
    assert_eq!(got["rate"], "1/60s");
}

#[test]
fn a_big_page_falls_back_to_one_language_and_then_drops_items() {
    let many = |count: u64| {
        let groups: Vec<Value> = (1..=count)
            .map(|id| json!({"id": id, "set": {"extra_prompt": "长".repeat(50)}}))
            .collect();
        cards(json!({"groups": groups}))
    };
    let small = groups_page(&many(3), &known(), &words(), "en");
    assert!(small[1]["items"][1]["title"].is_string());
    assert!(
        small[1]["items"][0]["title"].is_object(),
        "小的三种语言并列"
    );
    let middle = groups_page(&many(20), &known(), &words(), "en");
    assert!(size(&middle) <= MAX_BYTES);
    assert_eq!(
        middle[1]["items"][0]["title"], "Other groups",
        "大了换成一种语言"
    );
    let huge = groups_page(&many(199), &known(), &words(), "en");
    assert!(size(&huge) <= MAX_BYTES, "{}", size(&huge));
    let shown = huge[1]["items"].as_array().expect("几项").len();
    assert!(shown < 200 && shown > 1, "{shown}");
}

#[test]
fn every_word_on_the_pages_is_written_in_three_languages() {
    let words = words();
    let set = cards(json!({"groups": [{"id": 7}], "privates": [{"id": 9}]}));
    let text = format!(
        "{}{}",
        groups_page(&set, &known(), &words, "zh"),
        friends_page(&set, &known(), &words, "zh")
    );
    for prefix in [
        "\"field.",
        "\"heading.",
        "\"hint.",
        "\"chatty.",
        "\"long_text.",
        "\"other_groups_sub",
        "\"add_",
        "\"to_",
        "\"save\"",
        "\"remove\"",
        "\"unset\"",
    ] {
        assert!(!text.contains(prefix), "漏了的字照键画出来了：{prefix}");
    }
    for by in words.0.values() {
        assert_eq!(
            by.keys().cloned().collect::<Vec<_>>(),
            ["en", "ja", "zh"],
            "{by:?}"
        );
    }
    assert_eq!(words.get("save", "fr"), "Save", "没有的语言照英文");
    assert_eq!(words.get("nope", "zh"), "nope");
}

#[test]
fn defaults_carry_their_value_when_the_control_can_take_it() {
    let set = cards(json!({"other_groups": {}, "groups": [{"id": 7}], "privates": [{"id": 9}]}));
    let page = groups_page(&set, &known(), &words(), "zh");
    let group = &page[1]["items"][1];
    assert_eq!(
        field(group, "chatty")["default"]["value"],
        "some",
        "下拉照选项里的值"
    );
    assert_eq!(
        field(group, "managers")["default"]["value"],
        json!(["1001"]),
        "管理的人只写号"
    );
    assert_eq!(field(group, "persona")["default"]["value"], "miyu");
    assert!(
        field(group, "rate")["default"].get("value").is_none(),
        "没有默认的不写"
    );
    let other = &page[1]["items"][0];
    assert_eq!(field(other, "on")["default"]["value"], false, "开关是布尔");
    let friends = friends_page(&set, &known(), &words(), "zh");
    assert_eq!(
        field(&friends[1]["items"][1], "long_chars")["default"]["value"],
        300,
        "数字是数"
    );
    // 默认的人格不在选项里（删了）、类型对不上控件的不写。
    let gone = Known {
        defaults: json!({"other_groups": {}, "groups": {"7": {"persona": {"value": "gone"}, "on": {"value": "yes"}}}, "privates": {}}),
        ..known()
    };
    let page = groups_page(&set, &gone, &words(), "zh");
    assert!(
        field(&page[1]["items"][1], "persona")["default"]
            .get("value")
            .is_none()
    );
    assert!(
        field(&page[1]["items"][1], "on")["default"]
            .get("value")
            .is_none()
    );
}

#[test]
fn both_kinds_of_card_have_the_final_only_toggle() {
    let set = cards(json!({"groups": [{"id": 7, "set": {"final_only": false}}]}));
    let page = groups_page(&set, &known(), &words(), "zh");
    let toggle = field(&page[1]["items"][1], "final_only");
    assert_eq!(toggle["control"], "toggle");
    assert_eq!(toggle["value"], false);
    assert_eq!(toggle["label"]["zh"], "仅发送最终消息");
    let friends = friends_page(&Cards::default(), &known(), &words(), "zh");
    assert_eq!(
        field(&friends[1]["items"][1], "final_only")["value"],
        Value::Null
    );
}
