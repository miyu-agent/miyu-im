//! 主程序在哪（施工 I-186）：先照 `MIYU_PROGRAM`，不在了照旁边，再照名字从 `PATH` 找。

use std::path::PathBuf;

use super::locate;

/// 临时目录 `label`（每个测试各用各的）里的「im」，`with_beside` 的旁边放一个「主程序」：交回（目录，im 的路径）。
fn installed(label: &str, with_beside: bool) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("miyu-im-locate-{}-{label}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("建得了目录");
    let im = dir.join(format!("miyu-im{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&im, "").expect("写得进");
    let beside = dir.join(format!("miyu{}", std::env::consts::EXE_SUFFIX));
    if with_beside {
        std::fs::write(&beside, "").expect("写得进");
    } else if beside.exists() {
        std::fs::remove_file(&beside).expect("删得掉");
    }
    (dir, im)
}

#[test]
fn the_program_handed_over_comes_first() {
    let (dir, im) = installed("given", true);
    let given = dir.join("elsewhere-miyu");
    std::fs::write(&given, "").expect("写得进");
    assert_eq!(locate(Some(given.clone()), &im), given);
    std::fs::remove_dir_all(&dir).expect("删得掉");
}

#[test]
fn a_handed_program_that_is_gone_falls_back_beside_then_to_path() {
    let (dir, im) = installed("beside", true);
    let gone = dir.join("no-such-miyu");
    let beside = std::fs::canonicalize(&dir)
        .expect("在")
        .join(format!("miyu{}", std::env::consts::EXE_SUFFIX));
    assert_eq!(locate(Some(gone.clone()), &im), beside);
    assert_eq!(locate(None, &im), beside);
    std::fs::remove_dir_all(&dir).expect("删得掉");
    // 装在数据根的 `opt/miyu-im/bin/` 下：旁边没有主程序，照名字从 PATH 找。
    let (dir, im) = installed("path", false);
    let by_name = PathBuf::from(format!("miyu{}", std::env::consts::EXE_SUFFIX));
    assert_eq!(locate(Some(gone), &im), by_name);
    assert_eq!(locate(None, &im), by_name);
    std::fs::remove_dir_all(&dir).expect("删得掉");
}
