//! 平台工具（一）对谁做（施工 O-31，`onebot.md` 第一条「平台工具（一）」第 3、4 条；施工 O-48 起不挡人）：往坏里测。冒充（叫她
//! 做的那条和 `by` 对不上）、目标是她自己、引用和 @ 是同一个人、@ 了两个人、她给的编号和号、秒数的边界、私聊；撤回编号核对归属；
//! 时长的写法。

use miyu_client::ids::ExternalId;
use serde_json::json;

use miyu_im::chat::{Fetched, Platform, To};
use miyu_im::toolset::{Origin, Quote, duration};

use super::{Arg, Caller, Order, Plan, Scene, Why, plan, recallable};

/// 测试用的平台。
const QQ: Platform = Platform::new("qq");

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
            platform: QQ,
        },
    )
}

/// 撤回，没给编号。
const RECALL: Order = Order::Recall(Arg::Absent);

/// 禁言 `seconds` 秒，没给号。
fn mute(seconds: u64) -> Order {
    Order::Mute {
        seconds: Some(seconds),
        users: Arg::Absent,
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
    assert_eq!(Order::read("qq_recall", &json!({})), Some(RECALL));
    assert_eq!(
        Order::read("qq_poke", &json!({})),
        Some(Order::Poke(Arg::Absent))
    );
    assert_eq!(Order::read("qq_skip_reply", &json!({})), None);
    for (msg, wanted) in [
        (json!("123"), Arg::Given("123".to_string())),
        (json!(-45), Arg::Given("-45".to_string())),
        (json!("1e3"), Arg::Bad("msg must be a message id")),
        (json!(""), Arg::Bad("msg must be a message id")),
        (json!(1.5), Arg::Bad("msg must be a message id")),
    ] {
        assert_eq!(
            Order::read("qq_recall", &json!({"msg": msg})),
            Some(Order::Recall(wanted)),
            "{msg}"
        );
    }
    for (args, seconds, users) in [
        (json!({"seconds": 0}), Some(0), Arg::Absent),
        (json!({"seconds": 2_592_000}), Some(2_592_000), Arg::Absent),
        (json!({"seconds": 2_592_001}), None, Arg::Absent),
        (json!({"seconds": -1}), None, Arg::Absent),
        (json!({"seconds": "600"}), Some(600), Arg::Absent),
        (json!({"seconds": "十分钟"}), None, Arg::Absent),
        (json!({}), None, Arg::Absent),
        (
            json!({"seconds": 60, "user": 40001}),
            Some(60),
            Arg::Given(vec![40001]),
        ),
        (
            json!({"seconds": 60, "user": " 40001 "}),
            Some(60),
            Arg::Given(vec![40001]),
        ),
        (
            json!({"seconds": 60, "user": 0}),
            Some(60),
            Arg::Bad("user must be a QQ number"),
        ),
        (
            json!({"seconds": 60, "user": "小明"}),
            Some(60),
            Arg::Bad("user must be a QQ number"),
        ),
        // 施工 I-17：一次几个人，和 `user` 二选一。
        (
            json!({"seconds": 60, "users": [40001, "40002", 40001]}),
            Some(60),
            Arg::Given(vec![40001, 40002]),
        ),
        (
            json!({"seconds": 60, "users": []}),
            Some(60),
            Arg::Bad("users must be 1 to 32 QQ numbers"),
        ),
        (
            json!({"seconds": 60, "user": 40001, "users": [40002]}),
            Some(60),
            Arg::Bad("give user or users, not both"),
        ),
    ] {
        assert_eq!(
            Order::read("qq_mute", &args),
            Some(Order::Mute { seconds, users }),
            "{args}"
        );
    }
    let many: Vec<i64> = (40001..=40033).collect();
    assert_eq!(
        Order::read("qq_kick", &json!({"users": many})),
        Some(Order::Kick {
            users: Arg::Bad("users must be 1 to 32 QQ numbers"),
            block: false
        }),
        "最多 32 个"
    );
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
        in_group(
            Order::Recall(Arg::Bad("msg must be a message id")),
            &caller(40001),
            Some(&others)
        ),
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
        in_group(
            Order::Poke(Arg::Absent),
            &caller(BOSS),
            Some(&someone_elses)
        ),
        Plan::Poke(qq(BOSS)),
        "戳叫她的人"
    );
    let unknown = Caller { id: None };
    assert_eq!(
        in_group(Order::Poke(Arg::Absent), &unknown, Some(&someone_elses)),
        Plan::Refuse(Why::NoTarget)
    );
}

#[test]
fn mute_takes_one_person_never_her_and_never_holds_anyone_back() {
    let anyone = caller(40009);
    let muting = |origin: &Origin| in_group(mute(60), &anyone, Some(origin));
    let muted = |user: i64| Plan::Mute {
        users: vec![qq(user)],
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
    let given = |users: Arg<Vec<i64>>| Order::Mute {
        seconds: Some(600),
        users,
    };
    assert_eq!(
        in_group(
            given(Arg::Given(vec![40005])),
            &caller(40009),
            Some(&at_one)
        ),
        Plan::Mute {
            users: vec![qq(40005)],
            seconds: 600
        }
    );
    assert_eq!(
        in_group(given(Arg::Given(vec![40005])), &caller(40009), None),
        Plan::Mute {
            users: vec![qq(40005)],
            seconds: 600
        },
        "没有叫她的那条也禁得了"
    );
    assert_eq!(
        in_group(given(Arg::Given(vec![40005, 40006])), &caller(40009), None),
        Plan::Mute {
            users: vec![qq(40005), qq(40006)],
            seconds: 600
        },
        "一次几个人（施工 I-17）"
    );
    assert_eq!(
        in_group(
            given(Arg::Bad("user must be a QQ number")),
            &caller(40009),
            Some(&at_one)
        ),
        Plan::Refuse(Why::Bad("user must be a QQ number")),
        "写错的说哪个参数（施工 I-17）"
    );
    // 秒数先看。
    let no_seconds = Order::Mute {
        seconds: None,
        users: Arg::Given(vec![40005]),
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
        in_group(Order::Poke(Arg::Absent), &anyone, Some(&at_one)),
        Plan::Poke(qq(40002))
    );
    let at_two = origin(40001, None, &[40002, 40003]);
    assert_eq!(
        in_group(Order::Poke(Arg::Absent), &anyone, Some(&at_two)),
        Plan::Refuse(Why::ManyTargets)
    );
    let bare = origin(40001, quote("101", Some(40002)), &[]);
    assert_eq!(
        in_group(Order::Poke(Arg::Absent), &anyone, Some(&bare)),
        Plan::Poke(qq(40001)),
        "引用不算，戳叫她的人"
    );
}

#[test]
fn poke_takes_the_user_she_gives_over_the_mention() {
    // 施工 I-32：给了号的戳这个号，@ 的、叫她的都不管；号写错的说哪个参数。
    let anyone = caller(BOSS);
    let at_one = origin(BOSS, None, &[40002]);
    assert_eq!(
        Order::read("qq_poke", &json!({"user": "40003"})),
        Some(Order::Poke(Arg::Given(40003)))
    );
    assert_eq!(
        in_group(Order::Poke(Arg::Given(40003)), &anyone, Some(&at_one)),
        Plan::Poke(qq(40003))
    );
    assert_eq!(
        in_group(Order::Poke(Arg::Given(40003)), &anyone, None),
        Plan::Poke(qq(40003)),
        "没有她在回的那一条也戳得了"
    );
    assert_eq!(
        in_group(
            Order::read("qq_poke", &json!({"user": "小明"})).expect("是戳一戳"),
            &anyone,
            None
        ),
        Plan::Refuse(Why::Bad("user must be a QQ number"))
    );
}

#[test]
fn a_private_chat_recalls_any_quote_and_pokes_the_peer() {
    let me = qq(ME);
    let peer = qq(BOSS);
    let scene = Scene {
        peer: Some(&peer),
        me: &me,
        platform: QQ,
    };
    let anyone = caller(BOSS);
    let quoted = origin(BOSS, quote("77", None), &[40002]);
    assert_eq!(
        plan(RECALL, &anyone, Some(&quoted), &scene),
        recall("77", false)
    );
    assert_eq!(
        plan(Order::Poke(Arg::Absent), &anyone, Some(&quoted), &scene),
        Plan::Poke(peer.clone()),
        "私聊只戳对方"
    );
    assert_eq!(
        plan(Order::Poke(Arg::Given(BOSS)), &anyone, None, &scene),
        Plan::Poke(peer.clone()),
        "给的是对方照戳"
    );
    assert_eq!(
        plan(Order::Poke(Arg::Given(40002)), &anyone, None, &scene),
        Plan::Refuse(Why::NoTarget),
        "私聊里戳不了别人"
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
        Why::Bad("user must be a QQ number"),
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
            "bad-seconds",
            "bad-args"
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
