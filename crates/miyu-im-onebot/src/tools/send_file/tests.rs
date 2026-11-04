//! 传文件只认工作区里的普通文件、钉住了再交（施工 I-130）：相对的、绝对的都照真实位置比；`..`、指出去的链接、目录、不在的都不认；
//! 钉住以后原文件换成指出去的链接，钉住的那一份照旧是原来的内容（core1 11-04 审出来的时间差）。

use super::*;

/// 一个临时的场地（`name` 是测试自己的名字）：工作区 `ws/` 里一份 `out/report.pdf`，外面一份 `secret.txt`，`outbox/` 放钉住的。
fn setup(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("im-send-file-{}-{name}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("删得掉上一次留下的");
    }
    let workspace = root.join("ws");
    std::fs::create_dir_all(workspace.join("out")).expect("建得了");
    std::fs::write(workspace.join("out/report.pdf"), b"%PDF-1.7").expect("写得进");
    std::fs::write(root.join("secret.txt"), b"key").expect("写得进");
    let outbox = root.join("outbox");
    (root, workspace, outbox)
}

#[cfg(unix)]
#[test]
fn a_file_in_the_workspace_is_pinned_by_relative_or_absolute_path() {
    let (root, workspace, outbox) = setup("inside");
    for asked in [
        Path::new("out/report.pdf"),
        &workspace.join("out/report.pdf"),
    ] {
        let pinned = pin(&workspace, asked, &outbox).expect("认");
        assert_eq!((pinned.name.as_str(), pinned.bytes), ("report.pdf", 8));
        assert!(pinned.path.starts_with(&outbox), "交的是 outbox 里的那一份");
        assert_eq!(std::fs::read(&pinned.path).expect("读得了"), b"%PDF-1.7");
    }
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[test]
fn anything_outside_the_workspace_or_not_a_file_is_refused() {
    let (root, workspace, outbox) = setup("outside");
    for asked in [
        Path::new("../secret.txt"),
        &root.join("secret.txt"),
        Path::new("out"),
        Path::new("missing.pdf"),
    ] {
        assert_eq!(
            pin(&workspace, asked, &outbox),
            Err(Refused::NoFile),
            "{}",
            asked.display()
        );
    }
    assert_eq!(
        pin(Path::new(""), Path::new("out/report.pdf"), &outbox),
        Err(Refused::NoFile),
        "没给工作区的不认"
    );
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[cfg(unix)]
#[test]
fn a_link_pointing_out_of_the_workspace_is_refused() {
    let (root, workspace, outbox) = setup("link");
    std::os::unix::fs::symlink(root.join("secret.txt"), workspace.join("looks-fine.txt"))
        .expect("建得了链接");
    assert_eq!(
        pin(&workspace, Path::new("looks-fine.txt"), &outbox),
        Err(Refused::NoFile)
    );
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[cfg(unix)]
#[test]
fn swapping_the_file_for_a_link_after_the_check_does_not_change_what_is_sent() {
    let (root, workspace, outbox) = setup("swap");
    let pinned = pin(&workspace, Path::new("out/report.pdf"), &outbox).expect("认");
    // 判过、交出去以后，原文件换成指向工作区外面的链接：NapCat 读钉住的那一份，读到的照旧是原来的内容。
    let original = workspace.join("out/report.pdf");
    std::fs::remove_file(&original).expect("删得掉");
    std::os::unix::fs::symlink(root.join("secret.txt"), &original).expect("建得了链接");
    assert_eq!(std::fs::read(&pinned.path).expect("读得了"), b"%PDF-1.7");
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[cfg(not(unix))]
#[test]
fn without_inodes_a_file_is_not_sent() {
    let (root, workspace, outbox) = setup("no-inodes");
    assert_eq!(
        pin(&workspace, Path::new("out/report.pdf"), &outbox),
        Err(Refused::Unsendable)
    );
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[test]
fn only_pins_older_than_the_wait_are_swept() {
    let (root, _, outbox) = setup("sweep");
    std::fs::create_dir_all(&outbox).expect("建得了");
    let now = now_seconds();
    let (old, fresh) = (format!("{}-1-0", now - 4000), format!("{now}-1-1"));
    for name in [&old, &fresh] {
        std::fs::write(outbox.join(name), b"x").expect("写得进");
    }
    sweep(&outbox, Duration::from_secs(3600));
    assert!(!outbox.join(&old).exists(), "过了等的时限的清掉");
    assert!(outbox.join(&fresh).exists(), "还在传的留着");
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}
