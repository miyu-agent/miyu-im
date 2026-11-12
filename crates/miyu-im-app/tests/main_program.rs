//! 核心没在跑时 `start` 拉得起核心（施工 I-186，core1 做便携版时撞见）：im 装在数据根的 `opt/im/bin/` 下，旁边没有主程序；
//! 核心转交子命令时交的 `MIYU_PROGRAM`（主程序的真实位置）照它拉起。只在 Unix 上测：拿一个 shell 脚本当主程序，记下它被叫时的参数。

#[cfg(unix)]
#[tokio::test]
async fn start_launches_the_core_from_the_program_handed_over() {
    use std::os::unix::fs::PermissionsExt;

    use crate::support::roots::temp_root;
    use crate::support::spawning::{program, ran_to_end};

    let (dir, root) = temp_root();
    let called = dir.join("called");
    let fake = dir.join("main-miyu");
    // 「主程序」：记下参数，不当核心（退出 1）；im 照拉不起核心说。
    std::fs::write(
        &fake,
        format!("#!/bin/sh\necho \"$@\" > '{}'\nexit 1\n", called.display()),
    )
    .expect("写得进");
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("改得了权限");
    let mut command = program(&root, &["start"]);
    command.env("MIYU_PROGRAM", &fake);
    let started = ran_to_end(command).await;
    assert_ne!(started.status.code(), Some(0), "没有真核心：{started:?}");
    let said = std::fs::read_to_string(&called)
        .unwrap_or_else(|_| panic!("没照 MIYU_PROGRAM 拉起：{started:?}"));
    assert_eq!(said.trim(), "core");
    std::fs::remove_dir_all(&dir).expect("删得掉");
}
