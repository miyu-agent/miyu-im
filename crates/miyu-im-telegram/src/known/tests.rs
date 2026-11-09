use super::{Known, Person};

#[test]
fn threads_follow_the_last_message_of_each_chat() {
    let known = Known::default();
    known.set_thread("-100", Some(42));
    assert_eq!(known.thread("-100"), Some(42));
    assert_eq!(known.thread("-200"), None);
    // 话题群的「General」、普通群的消息不在话题里：回话不带话题。
    known.set_thread("-100", None);
    assert_eq!(known.thread("-100"), None);
}

#[test]
fn people_and_titles_are_remembered() {
    let known = Known::default();
    known.set_person(
        "7",
        Person {
            name: "Alex Chen".to_string(),
            username: Some("alex".to_string()),
        },
    );
    assert_eq!(
        known.person("7").and_then(|person| person.username),
        Some("alex".to_string())
    );
    known.set_title("-100", "Miyu 测试群");
    assert_eq!(known.title("-100").as_deref(), Some("Miyu 测试群"));
    assert_eq!(known.title("-1"), None);
}
