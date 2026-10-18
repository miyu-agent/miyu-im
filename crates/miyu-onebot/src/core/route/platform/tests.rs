//! 平台工具（一）对谁做（施工 O-31，`onebot.md` 第一条「平台工具（一）」第 3、4 条；施工 O-48 起不挡人）：往坏里测。冒充（叫她
//! 做的那条和 `by` 对不上）、目标是她自己、引用和 @ 是同一个人、@ 了两个人、她给的编号和号、秒数的边界、私聊；撤回编号核对归属；
//! 时长的写法。

use miyu_client::ids::ExternalId;
use serde_json::json;

use super::{Arg, Caller, Order, Origin, Plan, Quote, Scene, Why, duration, plan, recallable};
use crate::chat::{Fetched, To};

/// 平台上号是 `number` 的人。
fn qq(number: i64) -> ExternalId {
    ExternalId::parse(&format!("qq:{number}")).expect("合写法")
}

/// 她自己。
const ME: i64 = 30003;

/// 终端管理员。
const BOSS: i64 = 10001;

/// `number` 叫的。
fn caller(number: i64) -> Caller {
    Caller {
        id: Some(qq(number)),
    }
}

/// `number` 说的一条：引用 `quote`、@ 了 `mentions`。
fn origin(number: i64, quote: Option<Quote>, mentions: &[i64]) -> Origin {
    Origin {
        sender: qq(number),
        quote,
        mentions: mentions.iter().map(|one| qq(*one)).collect(),
    }
}

/// 引用平台编号 `msg` 那一条，发的人是 `sender`。
fn quote(msg: &str, sender: Option<i64>) -> Option<Quote> {
    Some(Quote {
        msg: msg.to_string(),
        sender: sender.map(qq),
    })
}

/// 在群里定。
fn in_group(order: Order, caller: &Caller, origin: Option<&Origin>) -> Plan {
    let me = qq(ME);
    plan(
        order,
        caller,
        origin,
        &Scene {
            peer: None,
            me: &me,
        },
    )
}

/// 撤回，没给编号。
const RECALL: Order = Order::Recall(Arg::Absent);

/// 禁言 `seconds` 秒，没给号。
fn mute(seconds: u64) -> Order {
    Order::Mute {
        seconds: Some(seconds),
        user: Arg::Absent,
    }
}

/// 撤平台编号是 `msg` 的那一条，`check` 的先核对。
fn recall(msg: &str, check: bool) -> Plan {
    Plan::Recall {
        msg: msg.to_string(),
        check,
    }
}

#[test]
fn orders_are_read_from_the_tool_with_what_she_gives() {
    assert_eq!(Order::read("recall", &json!({})), Some(RECALL));
    assert_eq!(Order::read("poke", &json!({})), Some(Order::Poke));
    assert_eq!(Order::read("skip_reply", &json!({})), None);
    for (msg, wanted) in [
        (json!("123"), Arg::Given("123".to_string())),
        (json!(-45), Arg::Given("-45".to_string())),
        (json!("1e3"), Arg::Bad),
        (json!(""), Arg::Bad),
        (json!(1.5), Arg::Bad),
    ] {
        assert_eq!(
            Order::read("recall", &json!({"msg": msg})),
            Some(Order::Recall(wanted)),
            "{msg}"
        );
    }
    for (args, seconds, user) in [
        (json!({"seconds": 0}), Some(0), Arg::Absent),
        (json!({"seconds": 2_592_000}), Some(2_592_000), Arg::Absent),
        (json!({"seconds": 2_592_001}), None, Arg::Absent),
        (json!({"seconds": -1}), None, Arg::Absent),
        (json!({"seconds": "600"}), None, Arg::Absent),
        (json!({}), None, Arg::Absent),
        (
            json!({"seconds": 60, "user": 40001}),
            Some(60),
            Arg::Given(40001),
        ),
        (
            json!({"seconds": 60, "user": " 40001 "}),
            Some(60),
            Arg::Given(40001),
        ),
        (json!({"seconds": 60, "user": 0}), Some(60), Arg::Bad),
        (json!({"seconds": 60, "user": "小明"}), Some(60), Arg::Bad),
    ] {
        assert_eq!(
            Order::read("mute", &args),
            Some(Order::Mute { seconds, user }),
            "{args}"
        );
    }
}

#[test]
fn the_caller_is_read_from_the_platform_identity_only() {
    let by = |by: serde_json::Value| Caller::read(&json!({"by": by}));
    assert_eq!(
        by(json!({"kind": "external", "id": "qq:40001", "role": "member"})),
        caller(40001)
    );
    // 私聊里终端管理员记成本人：照经的平台身份认。
    assert_eq!(
        by(json!({"kind": "person", "account": "admin", "via": "qq:10001"})),
        caller(BOSS)
    );
    assert_eq!(
        by(json!({"kind": "person", "account": "admin"})).id,
        None,
        "本机的人没有平台身份"
    );
    assert_eq!(
        by(json!({"kind": "module", "id": "qq:10001"})).id,
        None,
        "别的种类不认"
    );
    assert_eq!(Caller::read(&json!({})).id, None);
}

#[test]
fn anyone_recalls_anything_and_her_own_msg_is_checked() {
    // 施工 O-48：谁叫的都撤，别人的、她的都交给 QQ。
    let others = origin(40001, quote("101", Some(40002)), &[]);
    assert_eq!(
        in_group(RECALL, &caller(40001), Some(&others)),
        recall("101", false)
    );
    let unknown = origin(40001, quote("4040", None), &[]);
    assert_eq!(
        in_group(RECALL, &caller(40001), Some(&unknown)),
        recall("4040", false)
    );
    // 她给了编号：撤它，先核对；引用的不看。
    let given = Order::Recall(Arg::Given("777".to_string()));
    assert_eq!(
        in_group(given.clone(), &caller(40001), Some(&others)),
        recall("777", true)
    );
    assert_eq!(
        in_group(given, &caller(40001), None),
        recall("777", true),
        "没有叫她的那条也撤得了"
    );
    assert_eq!(
        in_group(Order::Recall(Arg::Bad), &caller(40001), Some(&others)),
        Plan::Refuse(Why::NotFound)
    );
    let bare = origin(40003, None, &[40002]);
    assert_eq!(
        in_group(RECALL, &caller(40003), Some(&bare)),
        Plan::Refuse(Why::NoQuote)
    );
    assert_eq!(
        in_group(RECALL, &caller(40003), None),
        Plan::Refuse(Why::NoQuote)
    );
}

#[test]
fn the_origin_must_come_from_the_caller() {
    // 叫她做的那条是别人的（和 `by` 对不上）：当没有。终端管理员的身份配不上别人的引用。
    let someone_elses = origin(40002, quote("101", Some(40001)), &[40001]);
    assert_eq!(
        in_group(RECALL, &caller(BOSS), Some(&someone_elses)),
        Plan::Refuse(Why::NoQuote)
    );
    assert_eq!(
        in_group(mute(60), &caller(BOSS), Some(&someone_elses)),
        Plan::Refuse(Why::NoTarget)
    );
    assert_eq!(
        in_group(Order::Poke, &caller(BOSS), Some(&someone_elses)),
        Plan::Poke(qq(BOSS)),
        "戳叫她的人"
    );
    let unknown = Caller { id: None };
    assert_eq!(
        in_group(Order::Poke, &unknown, Some(&someone_elses)),
        Plan::Refuse(Why::NoTarget)
    );
}

#[test]
fn mute_takes_one_person_never_her_and_never_holds_anyone_back() {
    let anyone = caller(40009);
    let muting = |origin: &Origin| in_group(mute(60), &anyone, Some(origin));
    let muted = |user: i64| Plan::Mute {
        user: qq(user),
        seconds: 60,
    };
    // 引用的和 @ 的是同一个人：一个。
    assert_eq!(
        muting(&origin(40009, quote("101", Some(40001)), &[40001])),
        muted(40001)
    );
    // 引用一个、@ 另一个：两个。
    assert_eq!(
        muting(&origin(40009, quote("101", Some(40001)), &[40002])),
        Plan::Refuse(Why::ManyTargets)
    );
    assert_eq!(
        muting(&origin(40009, None, &[40001, 40002])),
        Plan::Refuse(Why::ManyTargets)
    );
    // @ 的是她：不算。
    assert_eq!(
        muting(&origin(40009, None, &[ME])),
        Plan::Refuse(Why::NoTarget)
    );
    assert_eq!(muting(&origin(40009, None, &[ME, 40001])), muted(40001));
    // 引用的那一条桥不认识发的人：只看 @。
    assert_eq!(
        muting(&origin(40009, quote("7", None), &[])),
        Plan::Refuse(Why::NoTarget)
    );
    // 施工 O-48：终端管理员也照禁，谁叫的都做（做不做由她，动不动得了由 QQ）。
    assert_eq!(muting(&origin(40009, None, &[BOSS])), muted(BOSS));
}

#[test]
fn mute_takes_the_user_she_gives_over_the_quote() {
    let at_one = origin(40009, None, &[40002]);
    let given = |user: Arg<i64>| Order::Mute {
        seconds: Some(600),
        user,
    };
    assert_eq!(
        in_group(given(Arg::Given(40005)), &caller(40009), Some(&at_one)),
        Plan::Mute {
            user: qq(40005),
            seconds: 600
        }
    );
    assert_eq!(
        in_group(given(Arg::Given(40005)), &caller(40009), None),
        Plan::Mute {
            user: qq(40005),
            seconds: 600
        },
        "没有叫她的那条也禁得了"
    );
    assert_eq!(
        in_group(given(Arg::Bad), &caller(40009), Some(&at_one)),
        Plan::Refuse(Why::NoTarget)
    );
    // 秒数先看。
    let no_seconds = Order::Mute {
        seconds: None,
        user: Arg::Given(40005),
    };
    assert_eq!(
        in_group(no_seconds, &caller(40009), None),
        Plan::Refuse(Why::BadSeconds)
    );
}

#[test]
fn poke_takes_one_mention_or_the_caller() {
    let anyone = caller(40001);
    let at_one = origin(40001, None, &[ME, 40002]);
    assert_eq!(
        in_group(Order::Poke, &anyone, Some(&at_one)),
        Plan::Poke(qq(40002))
    );
    let at_two = origin(40001, None, &[40002, 40003]);
    assert_eq!(
        in_group(Order::Poke, &anyone, Some(&at_two)),
        Plan::Refuse(Why::ManyTargets)
    );
    let bare = origin(40001, quote("101", Some(40002)), &[]);
    assert_eq!(
        in_group(Order::Poke, &anyone, Some(&bare)),
        Plan::Poke(qq(40001)),
        "引用不算，戳叫她的人"
    );
}

#[test]
fn a_private_chat_recalls_any_quote_and_pokes_the_peer() {
    let me = qq(ME);
    let peer = qq(BOSS);
    let scene = Scene {
        peer: Some(&peer),
        me: &me,
    };
    let anyone = caller(BOSS);
    let quoted = origin(BOSS, quote("77", None), &[40002]);
    assert_eq!(
        plan(RECALL, &anyone, Some(&quoted), &scene),
        recall("77", false)
    );
    assert_eq!(
        plan(Order::Poke, &anyone, Some(&quoted), &scene),
        Plan::Poke(peer.clone()),
        "私聊只戳对方"
    );
    assert_eq!(
        plan(RECALL, &anyone, Some(&origin(BOSS, None, &[])), &scene),
        Plan::Refuse(Why::NoQuote)
    );
}

#[test]
fn a_given_msg_is_recallable_only_in_this_chat() {
    let one = |private, group, user| Fetched {
        private,
        group,
        user,
        segments: Default::default(),
    };
    let group = To::Group(888);
    assert!(recallable(&one(false, Some(888), Some(40001)), group, ME));
    assert!(
        recallable(&one(false, Some(888), Some(ME)), group, ME),
        "她自己的也算"
    );
    assert!(
        !recallable(&one(false, Some(999), None), group, ME),
        "别的群的"
    );
    assert!(!recallable(&one(true, None, Some(40001)), group, ME));
    let private = To::Private(BOSS);
    assert!(recallable(&one(true, None, Some(BOSS)), private, ME));
    assert!(
        recallable(&one(true, None, Some(ME)), private, ME),
        "她发给他的"
    );
    assert!(
        !recallable(&one(true, None, Some(40001)), private, ME),
        "别人的私聊"
    );
    assert!(!recallable(&one(false, Some(888), Some(BOSS)), private, ME));
}

#[test]
fn every_refusal_names_its_sentence() {
    let names: Vec<&str> = [
        Why::NoQuote,
        Why::NotFound,
        Why::NoTarget,
        Why::ManyTargets,
        Why::BadSeconds,
    ]
    .into_iter()
    .map(Why::name)
    .collect();
    assert_eq!(
        names,
        [
            "no-quote",
            "not-found",
            "no-target",
            "many-targets",
            "bad-seconds"
        ]
    );
}

#[test]
fn a_duration_is_written_in_human_units() {
    for (seconds, wanted) in [
        (1, "1s"),
        (45, "45s"),
        (60, "1m"),
        (600, "10m"),
        (3_600, "1h"),
        (5_400, "1h 30m"),
        (86_400, "1d"),
        (90_061, "1d 1h 1m 1s"),
        (2_592_000, "30d"),
    ] {
        assert_eq!(duration(seconds), wanted, "{seconds}");
    }
}
