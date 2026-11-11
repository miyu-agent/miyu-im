//! 模型多吐出来的下一轮不发进群（施工 I-214，`chat.md` 第五条「怎么走」第 2 条）：线上 2026-11-11 取的证，她的话后面跟着
//! 角色词 `user` 和一整段照给模型看的样子编出来的下一条用户消息，原样转成图发进了群。真核心拉起真桥，照那一条原样说。

use serde_json::json;

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::*;

/// 群号（同 `long_text.rs`）。
const GROUP: i64 = 671;

/// 假 NapCat 认得的群成员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 线上那一条：她的一句，后面是模型编的。超过 300 个字，不拦的话会转成图。
const LEAKED: &str = "他骂我垃圾 按群规直接禁了\n\nuser<style-note>In this group a message is usually a few words, rarely more than a dozen characters, and a sticker often stands in for words.</style-note>\n<another-line>You are replying to msg=-1604144125 only. Other messages are handled separately.</another-line>\n[18:31] Shorin (id=qq:2433516202, owner) [msg=-1604144125]: 直接踢了或者验证一下\n  reply-to: msg=-1488029772\n  @mentions: [you]";

#[tokio::test]
async fn a_made_up_next_turn_never_reaches_the_group() {
    assert!(LEAKED.chars().count() > 300, "比长文的阈值长：不拦会转成图");
    let script = Script::new([Play::Says(LEAKED)]);
    let (home, mut napcat, _) = started(&script, "", "", MEMBERS).await;
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        1,
        json!([at(BOT), plain(" 你为什么禁言")]),
        ("老板", "boss"),
    ));
    // 发进群的只有她自己的那一句：是字，不是图。
    assert_eq!(napcat.group_reply(GROUP).await, "他骂我垃圾 按群规直接禁了");
    until_logged(&home.root, |log| log.contains("next turn cut")).await;
    stopped(home).await;
}

#[test]
fn every_block_im_shows_the_model_is_on_the_factory_list() {
    // 给模型看的块（`data/facts`、`data/tail`、`data/system` 里尖括号开头的）新加一种，出厂的 `turn_marks` 要跟着加：不然模型
    // 照着它编下一轮时认不出来。名字是插件给的那一种（`<{kind}>`）列不出来，不算。
    let data = package().dir().join("data");
    let factory = miyu_chat::Params::read(&miyu_chat::File {
        source: miyu_chat::Source::Factory,
        name: "defaults.toml".to_string(),
        text: std::fs::read_to_string(data.join("defaults.toml")).expect("读得出"),
    })
    .expect("出厂的读得出");
    let marks = factory.outbound.turn_marks();
    let mut seen = 0;
    for folder in ["facts", "tail", "system"] {
        for entry in std::fs::read_dir(data.join(folder)).expect("有这个目录") {
            let path = entry.expect("读得出").path();
            let text = std::fs::read_to_string(&path).expect("读得出");
            if !text.starts_with('<') || text.starts_with("<{") {
                continue;
            }
            seen += 1;
            assert!(
                marks.iter().any(|mark| text.starts_with(mark.as_str())),
                "{} 的开头不在 turn_marks 里：{}",
                path.display(),
                text.lines().next().unwrap_or_default()
            );
        }
    }
    assert!(seen >= 14, "看到了 {seen} 个块");
}
