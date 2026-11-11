//! 模型多吐出来的下一轮（施工 I-214，`chat.md` 第五条「怎么走」第 2 条）：线上那一条原样只留下她的那一句；没有块、直接是
//! 一行场所消息的；整条都是编的；她自己会写的不动；名单是空的只认场所消息行。

use super::OutWhy;
use super::test_support::{ctx, drop, judge, send, text};

/// 线上取的证（2026-11-11）：她的话后面多吐了一个角色词和一整段编出来的下一条用户消息。
const LEAKED_TURN: &str = "他骂我垃圾 按群规直接禁了\n\nuser<style-note>In this group a message is usually a few words.</style-note>\n<another-line>You are replying to msg=-1604144125 only. Other messages are handled separately.</another-line>\n[18:31] Shorin (id=qq:2433516202, owner) [msg=-1604144125]: 直接踢了或者验证一下\n  reply-to: msg=-1488029772\n  @mentions: [you]";

#[test]
fn a_next_turn_the_model_ran_on_into_is_cut_off() {
    // 施工 I-214：给模型看的块的开头她自己不会写，出现了就是模型没停、接着编下一轮。从那里起整段不发，前面挂着的角色词
    // 也不发；她自己的那一句照发。
    assert_eq!(
        judge(text(LEAKED_TURN), &ctx()),
        send(text("他骂我垃圾 按群规直接禁了"))
    );
    // 没有块、直接是一行场所消息的样子（前面紧贴着角色词的、角色词单独一行的、带冒号的）。
    for leaked in [
        "好\nuser[18:31] Shorin (id=qq:1, owner) [msg=-5]: 踢了",
        "好\n\nUser:\n[18:31] Shorin (id=qq:1) [msg=7]: 踢了",
        "好\n  [08:01] 小明 (id=tg:9) [msg=7]: 在吗\n  reply-to: msg=3",
    ] {
        assert_eq!(judge(text(leaked), &ctx()), send(text("好")), "{leaked}");
    }
    // 整条都是编出来的下一轮：不发。
    assert_eq!(
        judge(
            text("user<another-line>You are replying</another-line>"),
            &ctx()
        ),
        drop(OutWhy::Leaked)
    );
    assert_eq!(
        judge(text("[18:31] Shorin (id=qq:1) [msg=7]: 踢了"), &ctx()),
        drop(OutWhy::Leaked)
    );
}

#[test]
fn words_that_only_look_a_bit_like_a_turn_are_left_alone() {
    // 她自己会写的：单独一个 user、带时间的聊天记录（没有 `[msg=`）、提到 msg 编号、别的尖括号。
    for own in [
        "用户名填 user 就行",
        "先切到\nuser\n再跑一遍",
        "[18:31] 小明: 在吗\n[18:32] 小红: 在",
        "那条是 msg=-1604144125",
        "写 <div> 就行",
        "时间是 [18:31] 左右 [msg 我没存]",
    ] {
        assert_eq!(judge(text(own), &ctx()), send(text(own)), "{own}");
    }
    // 标记的名单是空的：只认场所消息行。
    let mut bare = ctx();
    bare.outbound.turn_marks = Vec::new();
    let noted = "好<style-note>x</style-note>";
    assert_eq!(judge(text(noted), &bare), send(text(noted)));
    assert_eq!(
        judge(text("好\n[18:31] A (id=qq:1) [msg=7]: x"), &bare),
        send(text("好"))
    );
}
