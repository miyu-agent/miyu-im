//! 使用限制页（设计 08，施工 I-136）：真核心拉起真桥、假 NapCat。小林叫她、她回了以后，页面经通道 `limits.set` 存默认的线和
//! 单独设的人（写进 im 配置目录的 `limits.toml`），`limits.get` 读回来，最近用过的人带名字、三条线的用量和上限，满了的标出来；
//! 写错的整份不收。

use serde_json::{Value, json};

use miyu_session::testkit::{Play, Script};

use crate::support::browser::{Channel, call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群里的别人。
const LIN: i64 = 20004;

/// 群。
const GROUP: i64 = 777;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 一次主请求的四项合计：剧本照没命中 60、读缓存 40、输出 10 报。
const ONE: u64 = 110;

/// 等到 `limits.get` 列出了小林。
async fn listed(open: &mut Channel) -> Value {
    within("用量记下", async {
        let mut id = 100;
        loop {
            id += 1;
            let answer = call(open, id, "limits.get", json!({})).await;
            let result = answer["result"].clone();
            if result["usage"]
                .as_array()
                .is_some_and(|usage| !usage.is_empty())
            {
                return result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_page_sets_lines_and_shows_who_used_how_much() {
    let script = Script::new([Play::Says("在的。")]);
    let (home, mut napcat, _) = started(&script, "", "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |status| {
            status["napcat"]["connected"] == true && status["web"].is_u64()
        })
        .await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    let empty = call(&mut open, 1, "limits.get", json!({})).await;
    assert_eq!(
        empty["result"],
        json!({"default": {}, "people": [], "usage": []}),
        "{empty}"
    );

    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([at(BOT), plain(" 在吗")]),
        ("小林", "lin"),
    ));
    assert_eq!(napcat.group_reply(GROUP).await, "在的");

    let lin = format!("qq:{LIN}");
    let set = json!({
        "default": {"limit_5h": 1_000_000},
        "people": [{"id": lin, "limit_week": 100}],
    });
    let saved = call(&mut open, 2, "limits.set", set).await;
    assert_eq!(saved["result"], json!({"saved": true}), "{saved}");
    let written = std::fs::read_to_string(miyu_im::dirs::config(&home.root).join("limits.toml"))
        .expect("写进了配置目录");
    assert!(written.contains("limit_week = 100"), "{written}");

    let got = listed(&mut open).await;
    assert_eq!(got["default"], json!({"limit_5h": 1_000_000}), "{got}");
    assert_eq!(
        got["people"],
        json!([{"id": lin, "limit_week": 100}]),
        "{got}"
    );
    let mine = &got["usage"][0];
    assert_eq!(
        (&mine["id"], &mine["name"], &mine["used"]),
        (&json!(lin), &json!("小林"), &json!([ONE, ONE, ONE])),
        "{got}"
    );
    assert_eq!(
        (&mine["caps"], &mine["full"]),
        (&json!([null, 100, null]), &json!(true)),
        "单独设的整组替换默认的；7 天那条到了：{got}"
    );

    let bad = call(
        &mut open,
        3,
        "limits.set",
        json!({"people": [{"id": "10002"}]}),
    )
    .await;
    assert_eq!(bad["error"]["reason"], "bad_params", "{bad}");
    stopped(home).await;
}
