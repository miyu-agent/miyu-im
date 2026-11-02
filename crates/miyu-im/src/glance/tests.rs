//! 看一眼的纯逻辑：定时里的那一份写得出读得回、别的写法读不出；对账留每个场所头一个、删多的和认得却不该看的、
//! 不认得的留着、补没有的；
//! 新消息只算晚于上次、没当过一轮触发的，多了留最后 64 条。

use super::{MOST, Prompt, Reconciled, reconcile, unanswered};

fn prompt(venue: &str) -> Prompt {
    Prompt {
        venue: venue.to_string(),
        after: 7,
        seconds: 1500,
        hers: false,
    }
}

#[test]
fn the_prompt_reads_back_and_other_writings_do_not() {
    let one = prompt("qq:1:group:5");
    assert_eq!(Prompt::read(&one.text()), Some(one));
    assert_eq!(Prompt::read("每天早上看看服务器"), None);
    assert_eq!(Prompt::read("{\"venue\": \"qq:1:group:5\"}"), None);
}

#[test]
fn reconciling_keeps_one_per_venue_and_fills_the_gaps() {
    let want = ["a".to_string(), "b".to_string(), "c".to_string()];
    let known = [want.as_slice(), &["off".to_string()]].concat();
    let have = [
        ("t1".to_string(), Some(prompt("a"))),
        ("t2".to_string(), Some(prompt("a"))),
        ("t3".to_string(), Some(prompt("off"))),
        ("t4".to_string(), None),
        ("t5".to_string(), Some(prompt("c"))),
        ("t6".to_string(), Some(prompt("unseen"))),
        ("t7".to_string(), Some(prompt("unseen"))),
    ];
    assert_eq!(
        reconcile((&want, &known), &have),
        Reconciled {
            cancel: vec!["t2".into(), "t3".into(), "t4".into(), "t7".into()],
            add: vec!["b".into()],
        }
    );
    // 列会话没成（一个都不认得）：一个都不删。
    assert_eq!(reconcile((&[], &[]), &have[..1]), Reconciled::default());
}

#[test]
fn only_messages_after_the_last_look_and_not_answered_are_new() {
    let said = [(5, false), (8, false), (9, true), (10, false)];
    assert_eq!(unanswered(said.into_iter(), 7), [8, 10]);
    assert_eq!(unanswered(said.into_iter(), 10), Vec::<u64>::new());
    let many = (1..=100).map(|seq| (seq, false));
    let kept = unanswered(many, 0);
    assert_eq!(kept.len(), MOST);
    assert_eq!(kept.first(), Some(&37));
    assert_eq!(kept.last(), Some(&100));
}
