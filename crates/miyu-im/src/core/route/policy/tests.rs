use serde_json::{Value, json};

use super::Wanted;
use crate::core::route::applied::tests::applied;

fn wanted(model: Option<&str>, persona: Option<&str>, preset: Option<&str>) -> Wanted {
    Wanted {
        model: model.map(str::to_string),
        persona: persona.map(str::to_string),
        preset: preset.map(str::to_string),
    }
}

fn sent(fields: Option<serde_json::Map<String, Value>>) -> Value {
    fields.map_or(Value::Null, Value::Object)
}

#[test]
fn the_first_time_sends_what_is_set() {
    assert_eq!(
        sent(wanted(None, None, None).first()),
        Value::Null,
        "都没设的不发"
    );
    assert_eq!(
        sent(wanted(Some("a/m"), Some("mio"), None).first()),
        json!({"model": "a/m", "persona": "mio"})
    );
    assert_eq!(
        sent(wanted(None, None, Some("chat")).first()),
        json!({"preset": "chat"})
    );
}

#[test]
fn a_change_sends_what_changed_and_a_cleared_model_goes_back() {
    let before = wanted(Some("a/m"), Some("mio"), Some("chat"));
    assert_eq!(sent(before.since(&before)), Value::Null, "没变的不发");
    assert_eq!(
        sent(wanted(None, Some("mio"), Some("chat")).since(&before)),
        json!({"model": null}),
        "清掉的模型回到跟着配置"
    );
    assert_eq!(
        sent(wanted(Some("a/m"), Some("rin"), Some("full")).since(&before)),
        json!({"persona": "rin", "preset": "full"})
    );
    assert_eq!(
        sent(wanted(Some("a/m"), None, None).since(&before)),
        Value::Null,
        "清掉的人格、预设不发：核心的 null 是无人格"
    );
    assert_eq!(
        sent(wanted(Some("b/m"), Some("mio"), None).since(&Wanted::default())),
        json!({"model": "b/m", "persona": "mio"}),
        "原来没设的照新的发"
    );
}

#[test]
fn the_rules_give_the_three() {
    let set = applied("model = \"a/m\"\npersona = \"mio\"\npreset = \"chat\"\n");
    assert_eq!(
        Wanted::of(&set),
        wanted(Some("a/m"), Some("mio"), Some("chat"))
    );
    assert_eq!(
        Wanted::of(&applied("rate = \"0\"\n")),
        Wanted::default(),
        "没设的是空的"
    );
}
