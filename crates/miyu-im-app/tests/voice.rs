//! 她说的话合成的语音（施工 I-250，核心 SD-1；项目主人 11-17 本机验收第 26 条：QQ 里她永远在发文件）：真核心拉起真桥、假 NapCat，
//! 另装测试包（`miyu-im-test-hook` 带 `voice:`）登记一件工具 `speak`，她调了答 `sent` 里一段 `voice: true` 的语音。群里照她调的
//! 先后发：先是一个语音段（QQ 的 `record`，内容是那段音频），再是她的字；不当图发、不当文件传，`venue.delivered` 不记它。场所
//! 会话造的时候带上 QQ 一段语音的上限（`voice_max_ms` 60000），核心派给扩展的 `tool.call` 照样带。

use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::group::*;
use crate::support::hooked::install_with;
use crate::support::*;

/// 群号。
const GROUP: i64 = 671;

/// 假 NapCat 认得的群成员：她自己、终端管理员。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (ADMIN, "老板", "boss")];

/// 那段音频：几个字节的「MP3」（核心只看写法和大小，不解码）。
const MP3: &[u8] = b"ID3\x04\x00\x00\x00\x00\x00\x00fake mp3";

/// 那段音频写进一个临时文件，测试包照它 `blob.put`；另交回它记 `tool.call` 的文件。
fn files() -> (PathBuf, PathBuf) {
    let (dir, _) = temp_root();
    let audio = dir.join("say-01.mp3");
    std::fs::write(&audio, MP3).expect("写得进");
    (audio, dir.join("calls.jsonl"))
}

/// 等到测试包登记了 `speak`。
async fn until_provided(home: &Home) {
    let cache = home.root.provide_cache("speaker");
    let deadline = tokio::time::Instant::now() + WAIT;
    while !cache.exists() {
        assert!(tokio::time::Instant::now() < deadline, "测试包没登记工具");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn a_spoken_voice_reaches_the_group_as_a_voice_message() {
    let (audio, calls) = files();
    let args = vec![
        format!("voice:{}", audio.display()),
        format!("record:{}", calls.display()),
    ];
    let install =
        move |root: &miyu_store::root::DataRoot| install_with(root, "speaker", &args, &["tools"]);
    let script = Script::new([Play::calls(&[("speak", "{}")]), Play::Says("听听看")]);
    let (home, mut napcat, _) = started_installed(&script, "", MEMBERS, install).await;
    until_provided(&home).await;
    let words = json!([at(BOT), plain(" 说句话")]);
    napcat.send(group_frame(GROUP, ADMIN, 1, words, ("老板", "boss")));
    let voice = napcat.group_message(GROUP).await;
    assert_eq!(
        voice,
        [json!({"type": "record", "data": {"file": format!("base64://{}", STANDARD.encode(MP3))}})],
        "语音段，不是图、不是文件"
    );
    let words = napcat.group_message(GROUP).await;
    let text: String = words
        .iter()
        .filter_map(|segment| segment["data"]["text"].as_str())
        .collect();
    assert_eq!(text.trim(), "听听看", "{words:?}");
    let venue = format!("qq:{BOT}:group:{GROUP}");
    let events = until_events(&home.root, &venue, |events| {
        of_kind(events, "venue.delivered").len() == 1
    })
    .await;
    assert_eq!(
        events[0]["body"]["voice_max_ms"],
        json!(60_000),
        "{}",
        events[0]
    );
    let delivered = of_kind(&events, "venue.delivered");
    assert_eq!(
        delivered[0]["body"]["text"], "听听看",
        "语音不记 venue.delivered"
    );
    let queued: Vec<Value> = of_kind(&events, "ext.im.venues.queued")
        .iter()
        .map(|one| one["body"]["kind"].clone())
        .collect();
    assert_eq!(queued, ["image", "reply"], "语音也走出站队列，照先后");
    let called: Vec<Value> = std::fs::read_to_string(&calls)
        .expect("记了")
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(
        called[0]["tool_call"]["voice_max_ms"],
        json!(60_000),
        "{called:?}"
    );
    stopped(home).await;
}
