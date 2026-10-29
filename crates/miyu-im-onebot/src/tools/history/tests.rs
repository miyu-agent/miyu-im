//! 查聊天记录的参数、每一行的样子（施工 C-1 下）。

use serde_json::json;

use super::*;

/// 东八区。
fn east8() -> TimeZone {
    TimeZone::fixed(jiff::tz::offset(8))
}

/// 一天的毫秒数。
const DAY: i64 = 86_400_000;

/// 2026-10-20 00:00 东八区的毫秒。
const OCT20: i64 = 1_792_425_600_000;

fn asked() -> Asked {
    Asked {
        text: String::new(),
        user: None,
        from: None,
        to: None,
        everywhere: false,
    }
}

#[test]
fn the_words_the_person_the_span_and_the_scope_are_read() {
    assert_eq!(read(&json!({}), &east8()), Ok(asked()));
    assert_eq!(
        read(&json!({"query": " 团本 ", "user": 40001}), &east8()),
        Ok(Asked {
            text: "团本".to_string(),
            user: Some("40001".to_string()),
            ..asked()
        })
    );
    assert_eq!(
        read(&json!({"user": "小明", "all_chats": true}), &east8()),
        Ok(Asked {
            user: Some("小明".to_string()),
            everywhere: true,
            ..asked()
        })
    );
    assert_eq!(
        read(&json!({"from": "2026-10-20", "to": "2026-10-20"}), &east8()),
        Ok(Asked {
            from: Some(OCT20),
            to: Some(OCT20 + DAY),
            ..asked()
        }),
        "只写日子的「到」算到那天结束"
    );
    assert_eq!(
        read(&json!({"from": "2026-10-20 14:03"}), &east8()),
        Ok(Asked {
            from: Some(OCT20 + (14 * 60 + 3) * 60_000),
            ..asked()
        })
    );
}

#[test]
fn wrong_arguments_say_which_one() {
    let zone = east8();
    assert_eq!(
        read(&json!({"from": "昨天"}), &zone),
        Err("from and to must be YYYY-MM-DD or YYYY-MM-DD HH:MM")
    );
    assert_eq!(
        read(&json!({"from": "2026-10-21", "to": "2026-10-20"}), &zone),
        Err("from must be before to")
    );
    assert_eq!(
        read(&json!({"all_chats": "yes"}), &zone),
        Err("all_chats must be true or false")
    );
    assert_eq!(
        read(&json!({"user": [1]}), &zone),
        Err("user must be a QQ number or a name")
    );
    assert_eq!(read(&json!({"query": 3}), &zone), Err("query must be text"));
}

#[test]
fn a_line_has_time_speaker_id_text_media_quote_and_recall() {
    let at = OCT20 + 60_000;
    let time = clock(u64::try_from(at / 1000).expect("正的"));
    let one = json!({
        "venue": "qq:30003:group:31", "msg": "123", "sender": "qq:40001", "name": "小明\u{7}", "at": at,
        "text": "今晚\n打团本吗", "reply_to": "99", "mine": false, "recalled_at": at + 1,
        "media": [{"kind": "image", "id": "a"}, {"kind": "file", "id": "f", "name": "排班.pdf"}],
    });
    assert_eq!(
        line(&one, Some("qq:10001"), false),
        format!(
            "{time} 小明 (40001) msg=123: 今晚 打团本吗 [image] [file 排班.pdf] (reply to msg=99) (recalled)"
        )
    );
    let mine = json!({"venue": "qq:30003:private:7", "msg": "5", "sender": "qq:10001", "name": null, "at": at, "text": "好", "media": [], "mine": true});
    assert_eq!(
        line(&mine, Some("qq:10001"), true),
        format!("{time} private 7, you msg=5: 好")
    );
    let nameless = json!({"venue": "qq:30003:group:31", "msg": "6", "sender": "qq:40002", "name": "  ", "at": at, "text": "", "media": [{"kind": "sticker"}], "mine": false});
    assert_eq!(
        line(&nameless, None, true),
        format!("{time} group 31, 40002 msg=6: [sticker]")
    );
}

#[test]
fn only_all_chats_drops_the_venue() {
    let here = query_of(&asked(), "qq:30003:group:31");
    assert_eq!(here.venue.as_deref(), Some("qq:30003:group:31"));
    assert_eq!(here.limit, SHOWN);
    let everywhere = Asked {
        everywhere: true,
        user: Some("小明".to_string()),
        ..asked()
    };
    let all = query_of(&everywhere, "qq:30003:group:31");
    assert_eq!(all.venue, None, "跨场所的不按场所筛");
    assert_eq!(all.sender.as_deref(), Some("小明"));
}
