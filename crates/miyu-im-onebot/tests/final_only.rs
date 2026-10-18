//! 仅发送最终消息（施工 O-57，`onebot.md`「仅发送最终消息」）：真核心拉起真桥、假 NapCat，她照台词边说边调工具。设了的群只发
//! 这一轮最后那段：中间带工具调用的不发；最后一步带着工具调用、这一轮到此结束的（表情包那样）回合结束时发；这一轮不说话了的扣着的也不发。支线照它自己的回合办。私聊出厂每一步都发，打开了的照样在回合结束时发扣着的。

use std::sync::Arc;

use serde_json::{Value, json};

use crate::support::group::*;
use crate::support::hooked::install_with;
use crate::support::speaking::{Line, Lines};
use crate::support::*;

/// 群号。
const GROUP: i64 = 672;

/// 一张 1×1 的 PNG（同 `stickers.rs`）：表情包。
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0xf0,
    0x1f, 0x00, 0x05, 0x00, 0x01, 0xff, 0x89, 0x99, 0x3d, 0x1d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

/// 白名单成员。
const FRIEND: i64 = 50007;

/// 支线那头叫她的人。
const LANE_B: i64 = 20022;

/// 假 NapCat 认得的群成员：她自己、终端管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 群 672 仅发送最终消息（测试的卡片给「其他群」关了它，这里照系统规则打开）。
fn rule() -> String {
    format!(
        "[[rule]]\nmatch = {{ kind = \"group\", group = [{GROUP}] }}\noutbound = {{ final_only = true }}\n"
    )
}

/// 终端管理员在群里 @ 她。
fn call_her(napcat: &Answering) {
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 讲讲")]),
        ("老板", "boss"),
    ));
}

/// 群会话里入队的她的话。
async fn queued_replies(home: &Home, ended: usize) -> Vec<Value> {
    let events = until_events(&home.root, &format!("qq:group:{GROUP}"), |events| {
        of_kind(events, "turn.ended").len() >= ended
    })
    .await;
    of_kind(&events, "ext.im.venues.queued")
        .iter()
        .map(|one| one["body"]["text"].clone())
        .collect()
}

#[tokio::test]
async fn only_the_last_words_of_a_turn_reach_the_group() {
    let lines = Lines::new([Line::calls("我查一下。"), Line::says("查到了")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines), &rule(), MEMBERS).await;
    call_her(&napcat);
    assert_eq!(napcat.group_reply(GROUP).await, "查到了");
    assert_eq!(
        queued_replies(&home, 1).await,
        [json!("查到了")],
        "中间那一步不发"
    );
    stopped(home).await;
}

#[tokio::test]
async fn words_with_a_tool_that_ends_the_turn_go_out_when_it_ends() {
    // 测试包的 `sticker` 答 `end_turn`：她说一句、发表情包，这一轮到此结束。字在回合结束时发。
    let (dir, _) = temp_root();
    let path = dir.join("cat.png");
    std::fs::write(&path, PNG).expect("写得进");
    let args = vec![
        format!("sticker:{}", path.display()),
        "sticker-end".to_string(),
    ];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    // 先说一句（带着调用），再只发表情包、这一轮到此结束：后面那步没有字，不把扣着的那句换掉。
    let lines = Lines::new([Line::calls("给你看"), Line::uses("", "sticker", "{}")]);
    let (home, mut napcat, _) =
        started_installed_by(Arc::new(lines), (&rule(), ""), MEMBERS, install).await;
    until_provided(&home).await;
    call_her(&napcat);
    let words = loop {
        let action = napcat.action().await;
        let message = &action["params"]["message"];
        if message[0]["type"] == "text" {
            break message[0]["data"]["text"].clone();
        }
    };
    assert_eq!(words, "给你看");
    stopped(home).await;
}

#[tokio::test]
async fn a_turn_she_stays_quiet_in_sends_nothing() {
    let lines = Lines::new([Line::calls("我想想"), Line::uses("", "skip_reply", "{}")]);
    let (home, mut napcat, _) = started_by(Arc::new(lines), &rule(), MEMBERS).await;
    call_her(&napcat);
    assert_eq!(
        queued_replies(&home, 1).await,
        Vec::<Value>::new(),
        "不说话了：扣着的也不发"
    );
    assert!(napcat.pending().is_none(), "没发东西");
    stopped(home).await;
}

#[tokio::test]
async fn private_chats_send_every_step_by_default() {
    let lines = Lines::new([Line::calls("我查一下"), Line::says("查到了")]);
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) = started_installed_by(
        Arc::new(lines),
        ("", &onebot),
        MEMBERS,
        |_: &miyu_store::root::DataRoot| {},
    )
    .await;
    napcat.send(private_frame(FRIEND, 1, json!([plain("在吗")])));
    let mut said = Vec::new();
    while said.len() < 2 {
        let action = napcat.action().await;
        if action["action"] == "send_private_msg" {
            said.push(action["params"]["message"][0]["data"]["text"].clone());
        }
    }
    assert_eq!(said, [json!("我查一下"), json!("查到了")]);
    stopped(home).await;
}

#[tokio::test]
async fn a_private_chat_set_to_final_only_sends_the_held_words_when_the_turn_ends() {
    // 私聊照规则打开：先说一句（带着调用），再只发表情包结束这一轮，那一句在回合结束时发。
    let (dir, _) = temp_root();
    let path = dir.join("cat.png");
    std::fs::write(&path, PNG).expect("写得进");
    let args = vec![
        format!("sticker:{}", path.display()),
        "sticker-end".to_string(),
    ];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    let lines = Lines::new([Line::calls("给你看"), Line::uses("", "sticker", "{}")]);
    let rule = format!(
        "[[rule]]\nmatch = {{ kind = \"private\", user = [{FRIEND}] }}\noutbound = {{ final_only = true }}\n"
    );
    let onebot = format!("whitelist = [\"qq:{FRIEND}\"]\n");
    let (home, mut napcat, _) =
        started_installed_by(Arc::new(lines), (&rule, &onebot), MEMBERS, install).await;
    until_provided(&home).await;
    napcat.send(private_frame(FRIEND, 1, json!([plain("在吗")])));
    let words = loop {
        let action = napcat.action().await;
        let message = &action["params"]["message"];
        if action["action"] == "send_private_msg" && message[0]["type"] == "text" {
            break message[0]["data"]["text"].clone();
        }
    };
    assert_eq!(words, "给你看");
    stopped(home).await;
}

#[tokio::test]
async fn a_lane_sends_its_held_words_when_its_turn_ends() {
    // 主线在回 A（卡着）时 B @ 她：支线回 B，先说一句（带着调用）、再只发表情包结束支线这一轮，那一句在支线回合结束时发。
    let (dir, _) = temp_root();
    let path = dir.join("cat.png");
    std::fs::write(&path, PNG).expect("写得进");
    let args = vec![
        format!("sticker:{}", path.display()),
        "sticker-end".to_string(),
    ];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "stickers", &args, &["tools"]);
    let (release_a, hold_a) = tokio::sync::oneshot::channel();
    let lines = Lines::new([
        Line::says("回A").released_by(hold_a),
        Line::calls("给B看"),
        Line::uses("", "sticker", "{}"),
    ]);
    let rule = format!(
        "[[rule]]\nmatch = {{ kind = \"group\", group = [{GROUP}] }}\ndiscipline = \"when-called\"\nrate = \"100/1h\"\noutbound = {{ final_only = true }}\n"
    );
    let members: &[Member] = &[
        (BOT, "米尤", "miyu"),
        (ADMIN, "老板", "boss"),
        (LANE_B, "阿乙", "b"),
    ];
    let (home, mut napcat, _) =
        started_installed_by(Arc::new(lines.clone()), (&rule, ""), members, install).await;
    until_provided(&home).await;
    call_her(&napcat);
    within("主线收到请求", async {
        while lines.requests().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    napcat.send(group_frame(
        GROUP,
        LANE_B,
        2,
        json!([at(BOT), plain(" 我也要")]),
        ("阿乙", "b"),
    ));
    let words = loop {
        let action = napcat.action().await;
        let message = &action["params"]["message"];
        let text = message
            .as_array()
            .and_then(|segments| {
                segments
                    .iter()
                    .find(|one| one["type"] == "text" && one["data"]["text"] != " ")
            })
            .map(|one| one["data"]["text"].clone());
        if let Some(text) = text {
            break text;
        }
    };
    assert_eq!(words, "给B看", "支线扣着的那一句在支线回合结束时发");
    release_a.send(()).expect("还在等");
    stopped(home).await;
}

/// 等到测试包登记了 `sticker`（同 `stickers.rs`）。
async fn until_provided(home: &Home) {
    let cache = home
        .root
        .path()
        .join("state")
        .join("providers")
        .join("stickers.json");
    let deadline = tokio::time::Instant::now() + WAIT;
    while !cache.exists() {
        assert!(tokio::time::Instant::now() < deadline, "测试包没登记工具");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
