//! `miyu im access`（施工 I-16）：照终端的样子连上真核心，经 `package.call` 调桥自己的卡片方法，加名单里的群、加这个群跳过判断的人、
//! 列出来、去掉；群不在名单里的、号写错的是用法不对。

use miyu_im::access::{Access, Said, run};
use miyu_im::texts::Texts;
use miyu_im::whitelist::Change;
use miyu_store::resources::ResourceRoot;

use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;
use crate::support::*;

/// 跑一次：交回退出码、标准输出、标准错误。
async fn ran(home: &Home, access: Access) -> (u8, String, String) {
    let mut core = miyu_client::open::Core::connect_running(&home.root, "test")
        .await
        .expect("连得上核心");
    let texts = texts();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(
        &mut core,
        (&access, miyu_im_onebot::onebot::PLATFORM),
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

fn texts() -> Texts {
    Texts::load(ResourceRoot::at(resources()), package_in(resources()), "zh").expect("读得出来")
}

#[tokio::test]
async fn groups_and_skipped_people_are_managed_from_the_command_line() {
    let (home, _napcat) = up(&Lines::new([])).await;
    let texts = texts();
    let say = |said: Said| format!("{}\n", texts.access(&said));
    let groups = texts.access_list("groups", "");
    let skip = texts.access_list("skip", "7001");
    // 群不在名单里：先加群。
    let (code, _, err) = ran(
        &home,
        Access::Skip("7001".into(), Change::Add("50001".into())),
    )
    .await;
    assert_eq!((code, err), (2, say(Said::NoGroup("7001".into()))));
    let (code, out, _) = ran(&home, Access::Group(Change::Add("7001".into()))).await;
    assert_eq!(
        (code, out),
        (0, say(Said::Added(groups.clone(), "7001".into())))
    );
    let (code, out, _) = ran(&home, Access::Group(Change::Add("7001".into()))).await;
    assert_eq!(
        (code, out),
        (0, say(Said::Already(groups.clone(), "7001".into())))
    );
    let (code, out, _) = ran(
        &home,
        Access::Skip("7001".into(), Change::Add("50001".into())),
    )
    .await;
    assert_eq!(
        (code, out),
        (0, say(Said::Added(skip.clone(), "qq:50001".into())))
    );
    let (code, out, _) = ran(&home, Access::Skip("7001".into(), Change::List)).await;
    assert_eq!((code, out), (0, "qq:50001\n".to_string()));
    let (code, out, _) = ran(&home, Access::List).await;
    assert_eq!(code, 0);
    assert!(out.contains("7001") && out.contains("qq:50001"), "{out}");
    let (code, out, _) = ran(
        &home,
        Access::Skip("7001".into(), Change::Remove("50001".into())),
    )
    .await;
    assert_eq!(
        (code, out),
        (0, say(Said::Removed(skip, "qq:50001".into())))
    );
    let (code, out, _) = ran(&home, Access::Group(Change::Remove("7001".into()))).await;
    assert_eq!((code, out), (0, say(Said::Removed(groups, "7001".into()))));
    let (code, _, err) = ran(&home, Access::Group(Change::Add("七千".into()))).await;
    assert_eq!((code, err), (2, say(Said::Bad("七千".into()))));
    stopped(home).await;
}
