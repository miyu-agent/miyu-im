//! `miyu im whitelist`（施工 I-13）：照终端的样子连上真核心，加、列、去掉白名单成员；写进系统配置的 `im.whitelist`（`qq:<号>`），
//! 号写错是用法不对、不写。

use serde_json::{Value, json};

use miyu_im::texts::Texts;
use miyu_im::whitelist::{Change, Said, run};
use miyu_store::resources::ResourceRoot;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

/// 跑一次：交回退出码、标准输出、标准错误。
async fn ran(home: &Home, change: Change) -> (u8, String, String) {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let texts = Texts::load(ResourceRoot::at(resources()), package_in(resources()), "zh")
        .expect("读得出来");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(
        &mut core,
        (&change, miyu_im_onebot::onebot::PLATFORM),
        &texts,
        (&mut out, &mut err),
    )
    .await;
    (
        code,
        String::from_utf8(out).expect("是字"),
        String::from_utf8(err).expect("是字"),
    )
}

/// 系统配置里的白名单。
async fn listed(home: &Home) -> Value {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let answered = core
        .call("w", "config.get", json!({"keys": ["im.whitelist"]}))
        .await
        .expect("读得到");
    answered["items"]["im.whitelist"]["value"].clone()
}

#[tokio::test]
async fn members_are_added_listed_and_removed_from_the_command_line() {
    let (home, _napcat) = up(&Lines::new([])).await;
    let texts = Texts::load(ResourceRoot::at(resources()), package_in(resources()), "zh")
        .expect("读得出来");
    let say = |said: Said| format!("{}\n", texts.whitelist(&said));
    // 出厂测试的配置里 FRIEND 已经在。
    let (code, out, _) = ran(&home, Change::Add("50001".to_string())).await;
    assert_eq!((code, out), (0, say(Said::Added("qq:50001".to_string()))));
    assert_eq!(
        listed(&home).await,
        json!([format!("qq:{FRIEND}"), "qq:50001"])
    );
    let (code, out, _) = ran(&home, Change::Add("50001".to_string())).await;
    assert_eq!((code, out), (0, say(Said::Already("qq:50001".to_string()))));
    let (code, out, _) = ran(&home, Change::List).await;
    assert_eq!((code, out), (0, format!("qq:{FRIEND}\nqq:50001\n")));
    let (code, out, _) = ran(&home, Change::Remove("50001".to_string())).await;
    assert_eq!((code, out), (0, say(Said::Removed("qq:50001".to_string()))));
    assert_eq!(listed(&home).await, json!([format!("qq:{FRIEND}")]));
    let (code, out, err) = ran(&home, Change::Add("五万".to_string())).await;
    assert_eq!(
        (code, out.as_str(), err),
        (2, "", say(Said::Bad("五万".to_string())))
    );
    stopped(home).await;
}
