use serde_json::{Value, json};

use miyu_im::chat::{Event, Fetch, Id, MediaKind, Piece, Posted};

use super::{Me, event};
use crate::known::Known;

fn bot() -> Id {
    Id::new("111").expect("号")
}

fn taken(message: Value, known: &Known) -> (Option<Id>, Posted) {
    let bot = bot();
    let me = Me {
        bot: &bot,
        username: "miyu_bot",
    };
    match event(&me, &message, known).expect("收") {
        Event::Group { group, posted } => (Some(group), posted),
        Event::Private(posted) => (None, posted),
        other => panic!("{other:?}"),
    }
}

fn in_group(extra: Value) -> Value {
    let mut message = json!({
        "message_id": 9, "date": 1700000000,
        "chat": {"id": -1001, "type": "supergroup", "title": "Miyu 测试群"},
        "from": {"id": 7, "is_bot": false, "first_name": "Alex", "last_name": "Chen", "username": "alex"},
    });
    for (key, value) in extra.as_object().expect("对象") {
        message[key] = value.clone();
    }
    message
}

#[test]
fn a_group_message_names_the_sender_by_first_and_last_name() {
    let known = Known::default();
    let (group, posted) = taken(in_group(json!({"text": "hi"})), &known);
    assert_eq!(group.as_ref().map(Id::as_str), Some("-1001"));
    assert_eq!(posted.user.as_str(), "7");
    assert_eq!(posted.name.as_deref(), Some("Alex Chen"));
    assert_eq!((posted.message_id, posted.time), (9, 1700000000));
    assert_eq!(posted.text, "hi");
    assert_eq!(known.title("-1001").as_deref(), Some("Miyu 测试群"));
}

#[test]
fn mentions_of_her_and_text_mentions_are_ats_and_other_mentions_stay_text() {
    let known = Known::default();
    // 「😀 @miyu_bot 和 @Yuki 还有 @bob」：偏移照 UTF-16（😀 占两个）。
    let text = "😀 @miyu_bot 和 @Yuki 还有 @bob";
    let at = |part: &str| text[..text.find(part).expect("在")].encode_utf16().count();
    let entities = json!([
        {"type": "mention", "offset": at("@miyu_bot"), "length": 9},
        {"type": "text_mention", "offset": at("@Yuki"), "length": 5, "user": {"id": 8, "first_name": "Yuki"}},
        {"type": "mention", "offset": at("@bob"), "length": 4},
    ]);
    let (_, posted) = taken(
        in_group(json!({"text": text, "entities": entities})),
        &known,
    );
    assert_eq!(
        posted.segments.pieces,
        [
            Piece::Text("😀 ".to_string()),
            Piece::At(bot()),
            Piece::Text(" 和 ".to_string()),
            Piece::At(Id::new("8").expect("号")),
            Piece::Text(" 还有 @bob".to_string()),
        ]
    );
    assert_eq!(
        known.person("8").map(|person| person.name).as_deref(),
        Some("Yuki")
    );
}

#[test]
fn commands_at_her_go_as_miyu_commands_and_ones_at_another_bot_stay_text() {
    // 施工 I-183 下上：`/命令@她` 去掉 @ 照命令交（原来拆成「/help」加 @ 她，核心认不出是命令）；菜单登记的名字换回 Miyu 的。
    let known = Known::default();
    known.set_commands([("clear_memory".to_string(), "clear-memory".to_string())].into());
    let ours = in_group(
        json!({"text": "/help@miyu_bot", "entities": [{"type": "bot_command", "offset": 0, "length": 14}]}),
    );
    let (_, posted) = taken(ours, &known);
    assert_eq!(posted.text, "/help");
    assert!(posted.segments.at().is_empty());
    let mapped = in_group(
        json!({"text": "/clear_memory 全部", "entities": [{"type": "bot_command", "offset": 0, "length": 13}]}),
    );
    assert_eq!(taken(mapped, &known).1.text, "/clear-memory 全部");
    let theirs = in_group(
        json!({"text": "/help@other_bot", "entities": [{"type": "bot_command", "offset": 0, "length": 15}]}),
    );
    let (_, posted) = taken(theirs, &known);
    assert!(posted.segments.at().is_empty());
    assert_eq!(posted.text, "/help@other_bot");
}

#[test]
fn replying_to_her_calls_her_and_a_topic_root_is_not_a_reply() {
    let known = Known::default();
    let reply = in_group(
        json!({"text": "嗯", "reply_to_message": {"message_id": 5, "from": {"id": 111, "is_bot": true}}}),
    );
    let (_, posted) = taken(reply, &known);
    assert_eq!(posted.segments.reply_to.as_deref(), Some("5"));
    assert_eq!(posted.segments.at(), [bot()], "回复她算叫她");
    // 话题里的消息带着建话题那条当引用：不算引用，记下话题。
    let topic = in_group(
        json!({"text": "嗯", "is_topic_message": true, "message_thread_id": 3, "reply_to_message": {"message_id": 3, "from": {"id": 7}}}),
    );
    let (_, posted) = taken(topic, &known);
    assert_eq!(posted.segments.reply_to, None);
    assert!(posted.segments.at().is_empty());
    assert_eq!(known.thread("-1001"), Some(3));
}

#[test]
fn private_chats_and_media_and_what_is_not_taken() {
    let known = Known::default();
    let private = json!({"message_id": 1, "date": 1, "chat": {"id": 7, "type": "private"}, "from": {"id": 7, "first_name": "Alex"}, "photo": [{"file_id": "small"}, {"file_id": "big", "file_size": 2048}], "caption": "看"});
    let (group, posted) = taken(private, &known);
    assert_eq!(group, None);
    assert_eq!(posted.text, "看");
    assert_eq!(posted.segments.media.len(), 1);
    assert_eq!(
        (
            posted.segments.media[0].kind,
            posted.segments.media[0].id.as_str(),
            posted.segments.media[0].size
        ),
        (MediaKind::Image, "big", Some(2048))
    );
    let voice = in_group(json!({"voice": {"file_id": "v", "duration": 12}}));
    assert_eq!(
        taken(voice, &known).1.segments.media[0].kind,
        MediaKind::Voice
    );
    let file = in_group(json!({"document": {"file_id": "d", "file_name": "报告.pdf"}}));
    assert_eq!(
        taken(file, &known).1.segments.media[0].name.as_deref(),
        Some("报告.pdf")
    );
    let bot = bot();
    let me = Me {
        bot: &bot,
        username: "miyu_bot",
    };
    let from_bot =
        in_group(json!({"text": "x", "from": {"id": 9, "is_bot": true, "first_name": "B"}}));
    assert!(
        event(&me, &from_bot, &known).is_none(),
        "别的机器人说的不收"
    );
    let channel = json!({"message_id": 1, "date": 1, "chat": {"id": -5, "type": "channel"}, "from": {"id": 7}, "text": "x"});
    assert!(event(&me, &channel, &known).is_none(), "频道不收");
    assert!(
        event(&me, &Value::Null, &known).is_none(),
        "不是消息的更新不收"
    );
}

#[test]
fn pictures_and_still_stickers_can_be_fetched_and_the_rest_cannot() {
    // 施工 I-185 上：图、静态贴纸照图取；动的贴纸、语音、视频、文件只给占位。
    let known = Known::default();
    let fetch = |extra: Value| taken(in_group(extra), &known).1.segments.media[0].fetch;
    assert_eq!(
        fetch(json!({"photo": [{"file_id": "p"}]})),
        Some(Fetch::Image)
    );
    assert_eq!(
        fetch(
            json!({"sticker": {"file_id": "s", "emoji": "😀", "is_animated": false, "is_video": false}})
        ),
        Some(Fetch::Image)
    );
    assert_eq!(
        fetch(json!({"sticker": {"file_id": "s", "is_animated": true}})),
        None
    );
    assert_eq!(
        fetch(json!({"sticker": {"file_id": "s", "is_video": true}})),
        None
    );
    assert_eq!(fetch(json!({"voice": {"file_id": "v"}})), None);
    assert_eq!(fetch(json!({"video": {"file_id": "v"}})), None);
    assert_eq!(fetch(json!({"document": {"file_id": "d"}})), None);
}

#[test]
fn the_message_and_the_one_it_quotes_are_remembered_for_the_host() {
    // 施工 I-185 上：宿主问引用的那一条（取它带的图）时照收的时候记下的答。
    let known = Known::default();
    let quoted = json!({
        "message_id": 5, "date": 1700000000,
        "chat": {"id": -1001, "type": "supergroup"},
        "from": {"id": 8, "is_bot": false, "first_name": "Bea"},
        "photo": [{"file_id": "pic"}],
    });
    taken(
        in_group(json!({"text": "看这个", "reply_to_message": quoted})),
        &known,
    );
    let remembered = known.message("-1001", "5").expect("记下了引用的那一条");
    assert!(!remembered.private);
    assert_eq!(remembered.group, Id::new("-1001"));
    assert_eq!(remembered.user, Id::new("8"));
    assert_eq!(remembered.segments.media[0].id, "pic");
    assert_eq!(remembered.segments.media[0].fetch, Some(Fetch::Image));
    let own = known.message("-1001", "9").expect("这一条也记下了");
    assert_eq!(own.segments.reply_to.as_deref(), Some("5"));
    assert!(known.message("-1001", "6").is_none(), "没见过的没有");
    assert!(
        known.message("-1002", "5").is_none(),
        "别的聊天的同一个编号对不上"
    );
}
