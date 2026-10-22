//! 复读只回一次（施工 I-28，`onebot.md`「群里怎么叫她」第 5、9 条）：真核心拉起真桥、假 NapCat、她照台词说。几个人接连发叫她
//! 名字的同一句，第一条照常判（叫她才回的群里直接回），后面的记成复读只记下；她只开一轮、只回一句，回的是一串复读，不引用、
//! 不 @。她被禁言着时几条都到齐，解禁以后才开口：她开口时后面的复读都已经在投影里，结果不看先后快慢。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号。
const GROUP: i64 = 3801;

/// 接连复读的三个人。
const PEOPLE: [i64; 3] = [20031, 20032, 20033];

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[
    (BOT, "米尤", "miyu"),
    (20031, "阿甲", "a"),
    (20032, "阿乙", "b"),
    (20033, "阿丙", "c"),
];

/// 系统的场所规则：叫她才回（不过判官），触发词是她的名字，总引用、马上 @：单回一个人的话一定带引用或 @。
fn rules() -> String {
    format!(
        "[[rule]]\nmatch = {{ kind = \"group\" }}\ndiscipline = \"when-called\"\noutbound = {{ quote_after = 0, mention_after = \"0s\" }}\n\n\
         [[rule]]\nmatch = {{ kind = \"group\", group = [{GROUP}] }}\nkeywords = [\"米尤\"]\n"
    )
}

/// 一条消息里的字。
fn words(segments: &[Value]) -> String {
    segments
        .iter()
        .filter(|segment| segment["type"] == "text")
        .filter_map(|segment| segment["data"]["text"].as_str())
        .collect::<String>()
        .trim()
        .to_string()
}

/// 群里记成的事件里种类是 `kind` 的有 `n` 条：交回那时的全部事件。
async fn until_count(home: &Home, kind: &str, n: usize) -> Vec<Value> {
    until_events(&home.root, &format!("qq:group:{GROUP}"), |events| {
        of_kind(events, kind).len() >= n
    })
    .await
}

#[tokio::test]
async fn a_chain_of_repeats_gets_one_reply_without_a_quote_or_mention() {
    let script = Script::new([Play::Says("没似"), Play::Says("第二轮")]);
    let (home, mut napcat, _) = started(&script, &rules(), "", MEMBERS).await;
    napcat.send(group_ban(GROUP, BOT, "ban", Some(600)));
    until_count(&home, "ext.im.venues.muted", 1).await;
    for (index, (user, name)) in PEOPLE.iter().zip(["阿甲", "阿乙", "阿丙"]).enumerate() {
        let message = i64::try_from(index).expect("小") + 1;
        let text = ["米尤似了", "米尤 似了！", "米尤似了"][index];
        napcat.send(group_frame(
            GROUP,
            *user,
            message,
            json!([plain(text)]),
            (name, "x"),
        ));
    }
    let events = until_count(&home, "ext.im.chat.decided", 3).await;
    let decided = of_kind(&events, "ext.im.chat.decided");
    assert_ne!(
        decided[0]["body"]["inbound"], "record_only",
        "第一条照常判：{decided:?}"
    );
    for later in &decided[1..] {
        assert_eq!(later["body"]["inbound"], "record_only", "{later}");
        assert_eq!(later["body"]["why"], "repeated", "{later}");
    }
    napcat.send(group_ban(GROUP, BOT, "lift_ban", None));
    let only = napcat.group_message(GROUP).await;
    assert_eq!(words(&only), "没似", "{only:?}");
    assert!(
        only.iter()
            .all(|segment| segment["type"] != "reply" && segment["type"] != "at"),
        "回一串复读不引用、不 @：{only:?}"
    );
    let events = until_count(&home, "turn.ended", 1).await;
    assert_eq!(of_kind(&events, "turn.started").len(), 1, "只开一轮");
    assert!(napcat.pending().is_none(), "没有第二句");
    stopped(home).await;
}
