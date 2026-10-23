//! 消息记录页的方法（施工 C-1 中，设计 04）：真核心拉起真桥、假 NapCat，群里说了几句以后，页面经通道读 im 自己的聊天记录库：一页
//! 从旧到新、带判断；搜关键词、按人；删一个人的、全部删。

use serde_json::{Value, json};

use miyu_session::testkit::Script;

use crate::support::browser::{Channel, call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::*;

/// 群号：没有卡的「陌生群」，出厂只记下不回（判断照样记）。
const GROUP: i64 = 3971;

/// 群里的别人。
const LIN: i64 = 20004;

/// 假 NapCat 认得的群成员：她自己、小林。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu"), (LIN, "小林", "lin")];

/// 场所编号。
fn venue() -> String {
    format!("qq:group:{GROUP}")
}

/// 一页，等到有 `least` 条消息、每条都判过。
async fn page(open: &mut Channel, least: usize) -> Value {
    within("聊天记录记上了", async {
        let mut id = 100;
        loop {
            id += 1;
            let answer = call(open, id, "records.page", json!({"venue": venue()})).await;
            let result = answer["result"].clone();
            // 每一条都判过了才算（判断和消息是两次写，后一条的判断可能还在路上）。
            let messages: Vec<&Value> = result["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| item["type"] == "message")
                .collect();
            let judged = messages
                .iter()
                .filter(|item| item["mine"] != true)
                .all(|item| item.get("judged").is_some());
            if messages.len() >= least && judged {
                return result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_records_page_reads_searches_and_deletes() {
    let (home, napcat, _) = started(&Script::new([]), "", "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |status| {
            status["napcat"]["connected"] == true && status["web"].is_u64()
        })
        .await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    napcat.send(group_frame(
        GROUP,
        LIN,
        1,
        json!([plain("今晚打团本吗")]),
        ("小林", "lin"),
    ));
    napcat.send(group_frame(
        GROUP,
        ADMIN,
        2,
        json!([at(BOT), plain(" 在吗")]),
        ("终端管理员", "o"),
    ));
    // 一页：从旧到新，人说的带判断：小林没叫她的只记下（没叫、没抽中），终端管理员 @ 她的回了（陌生群也照回）。
    let first = page(&mut open, 2).await;
    let items = first["items"].as_array().expect("有");
    let texts: Vec<&str> = items
        .iter()
        .filter_map(|item| item["text"].as_str())
        .collect();
    assert_eq!(texts, ["今晚打团本吗", "@米尤 在吗"], "{first}");
    assert_eq!(items[0]["sender"], format!("qq:{LIN}"));
    assert_eq!(items[0]["name"], "小林");
    assert_eq!(
        items[0]["judged"],
        json!({"outcome": "record", "why": {"reason": "quiet"}})
    );
    assert_eq!(
        items[1]["judged"],
        json!({"outcome": "reply", "why": {"reason": "called"}})
    );
    assert_eq!(first["more"], false);
    // 搜：关键词、按人。
    let found = call(&mut open, 2, "records.search", json!({"query": "打团本"})).await;
    assert_eq!(found["result"]["items"][0]["msg"], "1", "{found}");
    let by = call(&mut open, 3, "records.search", json!({"sender": "小林"})).await;
    assert_eq!(
        by["result"]["items"].as_array().map(Vec::len),
        Some(1),
        "{by}"
    );
    // 有记录的场所：这一个群，两条。
    let venues = call(&mut open, 9, "records.venues", json!({})).await;
    assert_eq!(venues["result"]["venues"][0]["venue"], venue(), "{venues}");
    assert_eq!(venues["result"]["venues"][0]["count"], 2, "{venues}");
    // 删：参数写错的拒；什么都没写的不删；删小林的；全部删。
    let wrong = call(&mut open, 4, "records.delete", json!({"from": "昨天"})).await;
    assert_eq!(wrong["error"]["reason"], "bad_params", "{wrong}");
    let listed = call(&mut open, 10, "records.delete", json!({"venues": "一个群"})).await;
    assert_eq!(listed["error"]["reason"], "bad_params", "{listed}");
    let nothing = call(&mut open, 5, "records.delete", json!({})).await;
    assert_eq!(nothing["result"], json!({"deleted": 0}));
    let lin = call(
        &mut open,
        6,
        "records.delete",
        json!({"venues": [venue()], "sender": format!("qq:{LIN}")}),
    )
    .await;
    assert_eq!(lin["result"], json!({"deleted": 1}), "{lin}");
    let all = call(&mut open, 7, "records.delete", json!({"all": true})).await;
    assert_eq!(all["result"], json!({"deleted": 1}), "{all}");
    let empty = call(&mut open, 8, "records.page", json!({"venue": venue()})).await;
    assert_eq!(empty["result"]["items"], json!([]), "{empty}");
    stopped(home).await;
}
