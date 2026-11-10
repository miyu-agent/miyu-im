//! 删群文件（施工 I-204，项目主人 11-10：群文件空间满了要能让她清，删只听管理的人）：`qq_group_files {delete}`。普通成员叫的不删、
//! 不问 QQ，答 `managers-only`；卡片上管理的人、终端管理员叫的照 `delete_group_file` 删。

use serde_json::json;

use crate::support::ADMIN;
use crate::support::group::*;
use crate::support::platform::*;
use crate::support::speaking::Lines;

#[tokio::test]
async fn only_a_manager_can_have_a_group_file_deleted() {
    let lines = Lines::new(
        [
            uses("qq_group_files", r#"{"delete":"/f1"}"#),
            uses("qq_group_files", r#"{"delete":"/f1"}"#),
            uses("qq_group_files", r#"{"delete":"/f2"}"#),
        ]
        .into_iter()
        .flatten(),
    );
    let (home, mut napcat) = up(&lines).await;
    // 三个人说的字各不一样：一样的会被进站链当成复读、只记下。
    let words = |text: &str| json!([at(MEMBER), plain(text)]);
    assert_eq!(
        asks(&home, &napcat, MEMBER, 1301, words(" 把那个文件删了"), 1).await,
        refused("managers-only", &[]),
        "普通成员叫的不删"
    );
    assert_eq!(
        asks(&home, &napcat, MANAGER, 1302, words(" 删掉排班表"), 2).await,
        done("group-file-deleted", &[]),
        "管理的人叫的删"
    );
    let deleted = next_platform(&mut napcat).await;
    assert_eq!(deleted["action"], "delete_group_file", "{deleted}");
    assert_eq!(deleted["params"]["file_id"], "/f1", "{deleted}");
    assert_eq!(
        asks(&home, &napcat, ADMIN, 1303, words(" 另一个也删"), 3).await,
        done("group-file-deleted", &[]),
        "终端管理员叫的也删"
    );
    let deleted = next_platform(&mut napcat).await;
    assert_eq!(
        deleted["params"]["file_id"], "/f2",
        "只问了 QQ 两次：{deleted}"
    );
    stopped(home).await;
}
