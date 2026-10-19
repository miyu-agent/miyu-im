//! 判断说成原因（施工 I-7 上补）：照 `body` 的几种样子各说一遍。整条路（群里来消息、会话浮窗读到）在 `tests/site_venues.rs`。

use serde_json::json;

use super::why;

#[test]
fn record_only_says_why_it_was_held_back() {
    for (written, said) in [
        ("asleep", "asleep"),
        ("muted", "muted"),
        ("rate_limited", "rate"),
        ("not_allowed", "off"),
    ] {
        let body = json!({"inbound": "record_only", "why": written, "outcome": "record"});
        assert_eq!(why(&body), json!({"reason": said}), "{written}");
    }
}

#[test]
fn passed_ones_say_how_they_were_called_or_judged() {
    let direct = json!([{"kind": "direct", "bonus": 0.0}]);
    let body = json!({"inbound": "pass", "route": "commit", "conditions": direct});
    assert_eq!(why(&body), json!({"reason": "called"}));
    let followed = json!([{"kind": "probability"}, {"kind": "continuation"}]);
    let body = json!({"inbound": "pass", "route": "commit", "conditions": followed});
    assert_eq!(why(&body), json!({"reason": "continued"}), "主触发照先后挑");
    let body =
        json!({"inbound": "pass", "route": "commit", "conditions": [{"kind": "after_speaking"}]});
    assert_eq!(why(&body), json!({"reason": "after_speaking"}));
    let drawn = json!([{"kind": "probability"}]);
    let score = json!({"total": 0.34, "threshold": 0.8, "reply": false});
    let body = json!({"inbound": "pass", "route": "judge", "conditions": drawn, "score": score});
    assert_eq!(
        why(&body),
        json!({"reason": "judged", "trigger": "probability", "total": 0.34, "threshold": 0.8})
    );
    let body = json!({"inbound": "pass", "route": "judge", "conditions": direct});
    assert_eq!(
        why(&body),
        json!({"reason": "unjudged", "trigger": "direct"})
    );
    let body = json!({"inbound": "pass", "route": "moderation_only", "conditions": []});
    assert_eq!(why(&body), json!({"reason": "moderation"}));
    let body = json!({"inbound": "pass", "route": "record", "conditions": []});
    assert_eq!(why(&body), json!({"reason": "quiet"}));
    let body =
        json!({"inbound": "pass", "route": "record", "conditions": [{"kind": "probability"}]});
    assert_eq!(
        why(&body),
        json!({"reason": "skip_user"}),
        "跳过判断的人没冲她来"
    );
}

#[test]
fn a_hook_that_steered_is_named() {
    let body = json!({"inbound": "pass", "route": "commit", "conditions": [{"kind": "direct"}], "hook": {"package": "dice", "route": "reply"}});
    assert_eq!(why(&body), json!({"reason": "called", "hook": "dice"}));
}
