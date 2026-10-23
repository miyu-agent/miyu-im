//! 消息记录页现取带的东西（施工 C-1 中下补，设计 04 第一节第 1 条）：真核心拉起真桥、假 NapCat，页面经通道调 `records.media`：
//! 这个群里一条的图照 `get_msg`、`get_image` 取回来、是那张图；文件带名字、不是图；过期了的、别的群的、编号写错的取不到。

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use miyu_session::testkit::Script;

use crate::support::browser::{call, logged_in};
use crate::support::group::*;
use crate::support::head::Head;
use crate::support::media::{encoded, file, image, in_group, png};
use crate::support::platform::{GROUP, venue};
use crate::support::*;

/// 假 NapCat 认得的群成员：她自己。
const MEMBERS: &[Member] = &[(BOT, "米尤", "miyu")];

#[tokio::test(flavor = "multi_thread")]
async fn the_records_page_fetches_a_picture_or_a_file_of_this_venue_on_demand() {
    let (home, napcat, _) = started(&Script::new([]), "", "", MEMBERS).await;
    let mut head = Head::connect(&home.root).await;
    let status = head
        .until("status", |status| {
            status["napcat"]["connected"] == true && status["web"].is_u64()
        })
        .await;
    let web = u16::try_from(status["web"].as_u64().expect("有端口")).expect("是端口");
    let mut open = logged_in(&home.root, web).await;
    let picture = png(4, 3);
    napcat.stock_message(71, in_group(json!([image("p.jpg", 4096)])));
    napcat.stock_file("p.jpg", Ok(json!({"base64": encoded(&picture)})));
    napcat.stock_message(72, in_group(json!([file("排班.pdf", "u-72", 12)])));
    napcat.stock_file(
        "u-72",
        Ok(json!({"file_name": "排班.pdf", "base64": encoded(b"%PDF-1.4 x")})),
    );
    napcat.stock_message(73, in_group(json!([file("旧.zip", "u-73", 99)])));
    napcat.stock_file("u-73", Err("文件已过期"));
    let mut elsewhere = in_group(json!([image("p.jpg", 4096)]));
    elsewhere["group_id"] = json!(GROUP + 1);
    napcat.stock_message(74, elsewhere);

    let media = |msg: &str, id: &str| json!({"venue": venue(), "msg": msg, "id": id});
    let got = call(&mut open, 1, "records.media", media("71", "p.jpg")).await;
    assert_eq!(got["result"]["type"], "image/png", "{got}");
    assert_eq!(got["result"]["kind"], "image");
    let bytes = STANDARD
        .decode(got["result"]["data"].as_str().expect("有内容"))
        .expect("是 base64");
    assert_eq!(bytes, picture, "取回来的就是那张图");
    let pdf = call(&mut open, 2, "records.media", media("72", "u-72")).await;
    assert_eq!(
        (&pdf["result"]["type"], &pdf["result"]["name"]),
        (&json!("application/octet-stream"), &json!("排班.pdf")),
        "{pdf}"
    );
    for (id, (msg, item)) in [
        (3, ("73", "u-73")),
        (4, ("74", "p.jpg")),
        (5, ("71", "nope")),
    ] {
        let refused = call(&mut open, id, "records.media", media(msg, item)).await;
        assert_eq!(refused["error"]["reason"], "gone", "{msg} {refused}");
    }
    let wrong = call(
        &mut open,
        6,
        "records.media",
        json!({"venue": "qq:nowhere:1", "msg": "71", "id": "p.jpg"}),
    )
    .await;
    assert_eq!(wrong["error"]["reason"], "bad_params", "{wrong}");
    stopped(home).await;
}
