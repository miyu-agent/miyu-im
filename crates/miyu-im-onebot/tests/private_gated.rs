//! 白名单成员的私聊忽略限流、忽略睡眠（施工 I-7 中三下一）：出厂、没写的都忽略；卡上（这里照系统的规则写）关了忽略限流的，他的私聊
//! 照限流数：额度满了只记下、不开回合，运行日志说一声。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::napcat::private_frame;
use crate::support::*;

/// 白名单成员。
const FRIEND: i64 = 20004;

/// 假 NapCat 认得的人：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (FRIEND, "小林", "lin")];

#[tokio::test(flavor = "multi_thread")]
async fn a_listed_friend_who_does_not_ignore_the_rate_is_held_to_it() {
    // 剧本只排了一句：第二条要是开了回合，假模型会报「没排第 2 次」。
    let script = Script::new([Play::Says("在的。")]);
    let rules = format!(
        "[[rule]]\nmatch = {{ kind = \"private\", user = [{FRIEND}] }}\nignore_rate = false\nrate = \"1/300s\"\n"
    );
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started(&script, &rules, &onebot, MEMBERS).await;
    napcat.send(private_frame(FRIEND, 1, json!([plain("在吗")])));
    let first = napcat.action().await;
    assert_eq!(first["action"], "send_private_msg", "{first}");
    napcat.send(private_frame(FRIEND, 2, json!([plain("还在吗")])));
    let log = within("第二条只记下", async {
        loop {
            let log = run_log(&home.root);
            if log.contains("listed recorded only") {
                return log;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(log.contains("why=RateLimited"), "{log}");
    assert!(napcat.pending().is_none(), "第二条不回");
    stopped(home).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn only_what_is_turned_off_is_checked() {
    // 只关了忽略睡眠（睡眠写成不睡）：限流照旧忽略，额度一条也两条都回。
    let script = Script::new([Play::Says("在的。"), Play::Says("还在。")]);
    let rules = format!(
        "[[rule]]\nmatch = {{ kind = \"private\", user = [{FRIEND}] }}\nignore_sleep = false\nsleep = \"off\"\nrate = \"1/300s\"\n"
    );
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started(&script, &rules, &onebot, MEMBERS).await;
    for message in 1..=2 {
        napcat.send(private_frame(FRIEND, message, json!([plain("在吗")])));
        let said = napcat.action().await;
        assert_eq!(said["action"], "send_private_msg", "{said}");
    }
    assert!(!run_log(&home.root).contains("listed recorded only"));
    stopped(home).await;
}

/// 盖住此刻的睡眠时间：当地时间前后各一小时（同桥：照本机的时区）。
fn asleep_now() -> String {
    let now = jiff::Zoned::now();
    let at = |zoned: jiff::Zoned| format!("{:02}:{:02}", zoned.hour(), zoned.minute());
    let hour = jiff::SignedDuration::from_hours(1);
    let start = now.checked_sub(hour).expect("算得出");
    let end = now.checked_add(hour).expect("算得出");
    format!("{}-{}", at(start), at(end))
}

#[tokio::test(flavor = "multi_thread")]
async fn sleep_stays_ignored_when_only_the_rate_is_checked() {
    // 只关了忽略限流，睡眠时间正盖着此刻：睡眠照旧忽略，照回。
    let script = Script::new([Play::Says("在的。")]);
    let rules = format!(
        "[[rule]]\nmatch = {{ kind = \"private\", user = [{FRIEND}] }}\nignore_rate = false\nrate = \"5/300s\"\nsleep = \"{}\"\n",
        asleep_now()
    );
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started(&script, &rules, &onebot, MEMBERS).await;
    napcat.send(private_frame(FRIEND, 1, json!([plain("在吗")])));
    let said = napcat.action().await;
    assert_eq!(said["action"], "send_private_msg", "{said}");
    assert!(!run_log(&home.root).contains("listed recorded only"));
    stopped(home).await;
}
