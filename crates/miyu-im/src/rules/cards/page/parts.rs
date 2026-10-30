//! 拼两页用的几个小件：加一项的选项、卡片写成 JSON、页面上的值、控件的名字、号和人格的名字。

use serde_json::{Map, Value, json};

use super::{Control, MAX_OPTIONS};
use crate::chat::Id;

/// 加一项的选项：`all` 里不在 `taken` 的，照号排，最多 [`MAX_OPTIONS`] 个，值是 `prefix` 加号，字是「名字（号）」。
pub(super) fn choices(all: &[(Id, String)], taken: &[Id], prefix: &str) -> Vec<Value> {
    let mut left: Vec<&(Id, String)> = all.iter().filter(|(id, _)| !taken.contains(id)).collect();
    left.sort_by(|(left, _), (right, _)| left.cmp(right));
    left.into_iter()
        .take(MAX_OPTIONS)
        .map(|(id, name)| json!({"value": format!("{prefix}{id}"), "label": format!("{name}（{id}）")}))
        .collect()
}

/// 卡片的一组格写成 JSON 对象（没写的格不在里面）。
pub(super) fn to_object(set: &impl serde::Serialize) -> Map<String, Value> {
    match serde_json::to_value(set) {
        Ok(Value::Object(object)) => object,
        _ => Map::new(),
    }
}

/// 页面上的值：没写的 `null`，管理的人去掉平台的前缀（`qq:`，施工 I-80：哪个平台的都去掉，页面上只写号）。
pub(super) fn page_value(key: &str, value: Option<&Value>) -> Value {
    match (key, value) {
        (_, None) => Value::Null,
        ("managers", Some(Value::Array(list))) => list
            .iter()
            .map(|one| match one.as_str() {
                Some(text) => json!(bare(text)),
                None => one.clone(),
            })
            .collect(),
        (_, Some(value)) => value.clone(),
    }
}

/// 平台上的身份 `<平台>:<号>` 去掉平台，只留号；不是这样写的照原样。
pub(super) fn bare(person: &str) -> &str {
    person.split_once(':').map_or(person, |(_, number)| number)
}

/// 控件的名字。
pub(super) fn control_name(control: Control) -> &'static str {
    match control {
        Control::Toggle => "toggle",
        Control::Persona | Control::LongText => "select",
        Control::Textarea => "textarea",
        Control::List => "list",
        Control::Text => "text",
        Control::Number => "number",
    }
}

/// 号对应的名字。
pub(super) fn name_of(list: &[(Id, String)], id: &Id) -> Option<String> {
    list.iter()
        .find(|(one, _)| one == id)
        .map(|(_, name)| name.clone())
        .filter(|name| !name.is_empty())
}

/// 人格编号对应的名字。
pub(super) fn name_of_persona(list: &[(String, String)], id: &str) -> Option<String> {
    list.iter()
        .find(|(one, _)| one == id)
        .map(|(_, name)| name.clone())
}

/// 一格默认值写给头的样子（`default.value`）：照控件的类型，开关是布尔，下拉要是选项里的值，数字是数，字是字，列表是字的数组
/// （管理的人去掉 `qq:`）；没有的（`null`）、类型对不上的、下拉里没有的交回空的，不写。
pub(super) fn default_value(
    key: &str,
    control: Control,
    value: &Value,
    options: &Value,
) -> Option<Value> {
    let fits = match (control, value) {
        (Control::Toggle, Value::Bool(_)) | (Control::Number, Value::Number(_)) => true,
        (Control::Text | Control::Textarea, Value::String(_)) => true,
        (Control::List, Value::Array(list)) => list.iter().all(Value::is_string),
        (Control::Persona | Control::LongText, Value::String(_)) => options
            .as_array()
            .is_some_and(|options| options.iter().any(|option| option["value"] == *value)),
        _ => false,
    };
    fits.then(|| page_value(key, Some(value)))
}
