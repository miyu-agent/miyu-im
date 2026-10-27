//! 每一项都落进自己那一格（`chat.md` 第八条「守着它的」）：规则里每一项写一个和别的都不一样的数，套上以后照原样落在
//! `Params` 的那一格。从 `tests.rs` 挪出来（施工 I-29：加了按人冷静的五项，那个文件过了 500 行）。

use super::super::test_support::{group, rules};
use super::tests::file;
use crate::params::test_support::defaults;
use crate::{
    Base64, Chatty, Followup, Judge, Outbound, Params, PersonRestraint, Restraint, Window,
};

#[test]
fn every_item_lands_in_its_own_field() {
    // 每一项写一个和别的都不一样的数：出厂有几项的数一样（两个 0.25、两个 0.15、两个 0.1、两个 30 秒），放错了格看不出来。
    let files = file(
        "[[rule]]\n\
         [rule.inbound]\nbase64_min_chars = 31\nbase64_max_chars = 4001\nbase64_printable = 801\nrepeat_window = \"33s\"\n\
         [rule.chatty]\nprobability = 41\nbase = 1.1\nrelevance = 1.2\nwillingness = 1.3\nsocial = 1.4\n\
         timing = 1.5\ncontinuity = 1.6\nadjust = 1.7\ndirect = 1.8\ncontinuation = 1.9\n\
         continuation_window = \"21s\"\nfollowup = false\nfollowup_bonus = 2.4\nafter_speaking = 2.1\nafter_speaking_window = \"22s\"\n\
         restraint = false\nrestraint_half_life = \"23s\"\nrestraint_cap = 2.2\nrestraint_k = 2.3\n\
         person_restraint = false\nperson_half_life = \"34s\"\nperson_start = 3.1\nperson_full = 3.2\nperson_cap = 3.3\n\
         severity_min = 9\njudge_called = false\nskip_media = false\nskip_users = [\"qq:1\"]\n\
         [rule.dispatch]\nsupersede_window = \"24s\"\n\
         [rule.judge]\nmodel = \"@cheap\"\nrecords = 25\npersona = false\nmax_tokens = 26\ntimeout = \"27s\"\n\
         moderation_timeout = \"28s\"\nattempt_timeout = \"15s\"\nretries = 2\nreason_chars = 29\n\
         [rule.outbound]\nquote = false\nmention = false\nquote_after = 5\nmention_after = \"31s\"\nmin_bigrams = 17\nsimilar = 67\n\
         strip_period = false\nhumanlike = true\ntyping_speed = 81\nsplit_chars = 3001\nlong_text = \"split\"\nlong_chars = 301\nfinal_only = true\ninvisible = [\"a\", \"b\"]\nleak_open = [\"<c\"]\nleak_close = [\"c>\"]\n",
    );
    let expected = Params {
        base64: Base64 {
            min_chars: 31,
            max_chars: 4001,
            printable: 801,
        },
        chatty: Chatty {
            probability: 41,
            base: 1.1,
            weights: [1.2, 1.3, 1.4, 1.5, 1.6],
            adjust: 1.7,
            direct: 1.8,
            continuation: Window {
                bonus: 1.9,
                window: 21_000,
            },
            followup: Followup {
                on: false,
                bonus: 2.4,
            },
            after_speaking: Window {
                bonus: 2.1,
                window: 22_000,
            },
            restraint: Restraint {
                on: false,
                half_life: 23_000,
                cap: 2.2,
                k: 2.3,
            },
            person: PersonRestraint {
                on: false,
                half_life: 34_000,
                start: 3.1,
                full: 3.2,
                cap: 3.3,
            },
            severity_min: 9,
            judge_called: false,
            skip_media: false,
            skip_users: vec!["qq:1".to_string()],
        },
        repeat_window: 33_000,
        supersede_window: 24_000,
        judge: Judge {
            model: Some("@cheap".to_string()),
            records: 25,
            persona: false,
            max_tokens: 26,
            timeout: 27_000,
            moderation_timeout: 28_000,
            attempt_timeout: 15_000,
            retries: 2,
            reason_chars: 29,
        },
        outbound: Outbound {
            quote: false,
            mention: false,
            quote_after: 5,
            mention_after: 31_000,
            min_bigrams: 17,
            similar: 67,
            invisible: vec!['a', 'b'],
            leak_open: vec!["<c".to_string()],
            leak_close: vec!["c>".to_string()],
            strip_period: false,
            humanlike: true,
            typing_speed: 81,
        },
        split_chars: 3001,
        long_text: crate::LongText::Split,
        long_chars: 301,
        final_only: true,
    };
    let resolved = rules(&files).resolve(&group("1"));
    assert_eq!(resolved.entries.len(), crate::params::items::ITEMS.len());
    assert_eq!(defaults().at(&resolved), expected);
}
