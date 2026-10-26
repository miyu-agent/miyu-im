//! `chat.media` 的参数怎么读（施工 I-39）：`session` 不空、`msg` 整数或写成整数的字、`index` 不写是 1、写了是正整数。

use serde_json::json;

use super::*;

#[test]
fn a_request_reads_session_msg_and_index() {
    assert_eq!(
        Wanted::read(&json!({"session": "s-1", "msg": "-523932813", "index": 2})),
        Some(Wanted {
            session: "s-1".to_string(),
            msg: "-523932813".to_string(),
            index: 2,
        })
    );
    let bare = Wanted::read(&json!({"session": "s-1", "msg": 601})).expect("读得出");
    assert_eq!((bare.msg.as_str(), bare.index), ("601", 1));
}

#[test]
fn a_wrong_request_reads_as_nothing() {
    for wrong in [
        json!({"msg": 601}),
        json!({"session": "", "msg": 601}),
        json!({"session": "s-1"}),
        json!({"session": "s-1", "msg": "六〇一"}),
        json!({"session": "s-1", "msg": 601, "index": 0}),
        json!({"session": "s-1", "msg": 601, "index": -1}),
        json!({"session": "s-1", "msg": 601, "index": "2"}),
    ] {
        assert_eq!(Wanted::read(&wrong), None, "{wrong}");
    }
}
