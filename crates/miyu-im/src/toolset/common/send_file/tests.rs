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
    // 工作区外面的答 `outside`，没有的、不是文件的答 `no-file`（施工 I-190 分开：原来一律 `no-file`，她当成了白名单）。
    for (asked, why) in [
        (Path::new("../secret.txt"), Refused::Outside),
        (&root.join("secret.txt"), Refused::Outside),
        (Path::new("out"), Refused::NoFile),
        (Path::new("missing.pdf"), Refused::NoFile),
    ] {
        assert_eq!(
            pin(&workspace, asked, &outbox),
            Err(why),
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
        Err(Refused::Link)
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

#[test]
fn a_file_in_the_workspace_is_sent_on_every_system() {
    // 施工 I-185 下下（项目主人 11-10：文件应该让发，别管什么身份）：原来不是 Unix 的系统一律不传，现在钉一份照传。
    let (root, workspace, outbox) = setup("every-system");
    let pinned = pin(&workspace, Path::new("out/report.pdf"), &outbox).expect("认");
    assert_eq!(std::fs::read(&pinned.path).expect("钉住了"), b"%PDF-1.7");
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

#[test]
fn an_open_session_sends_any_real_file_from_a_pinned_copy() {
    // 施工 LV-4（项目主人 11-10 定）：放开的会话工作区外面的也发；交出去的是钉在 `outbox/` 里的那一份，传完删它不碰原文件。
    let (root, workspace, outbox) = setup("open");
    let pinned = pin_open(&workspace, Path::new("../secret.txt"), &outbox).expect("外面的也认");
    assert_eq!(pinned.name, "secret.txt");
    assert_eq!(pinned.bytes, 3);
    assert!(pinned.path.starts_with(&outbox), "{pinned:?}");
    assert_eq!(std::fs::read(&pinned.path).expect("钉住了"), b"key");
    std::fs::remove_file(&pinned.path).expect("删得掉钉住的");
    assert_eq!(
        std::fs::read(root.join("secret.txt")).expect("原文件还在"),
        b"key"
    );
    let absolute = root.join("secret.txt");
    assert!(
        pin_open(Path::new(""), &absolute, &outbox).is_ok(),
        "没有工作区的认绝对路径"
    );
    for refused in [
        pin_open(&workspace, Path::new("out"), &outbox),
        pin_open(&workspace, Path::new("missing.txt"), &outbox),
        pin_open(Path::new(""), Path::new("secret.txt"), &outbox),
    ] {
        assert_eq!(refused, Err(Refused::NoFile));
    }
    std::fs::remove_dir_all(&root).expect("清得掉");
}

#[test]
fn a_leading_tilde_means_the_home_folder() {
    // 施工 I-190：从旧版导进来的会话工作区记的是 `~`，她写路径也爱写 `~/…`；两样都换成家目录。
    let home = Path::new("/home/someone");
    assert_eq!(
        paths("~", "~/Documents/a.md", Some(home)),
        (home.to_path_buf(), home.join("Documents/a.md"))
    );
    assert_eq!(
        paths("/srv/ws", "out/a.md", Some(home)),
        (PathBuf::from("/srv/ws"), PathBuf::from("out/a.md")),
        "别的照原样"
    );
    assert_eq!(
        paths("~", "~/a.md", None),
        (PathBuf::from("~"), PathBuf::from("~/a.md")),
        "家目录读不出的照原样"
    );
}

#[cfg(unix)]
#[test]
fn a_shortcut_is_not_followed_even_inside_the_workspace() {
    // 施工 I-185 下下（项目主人 11-10：别跟过去，是快捷方式就照快捷方式发）：外人的会话里链接一律不发，指向工作区里面的也一样；
    // `outbox/` 里什么都没留下。
    let (root, workspace, outbox) = setup("shortcut");
    std::os::unix::fs::symlink(workspace.join("out/report.pdf"), workspace.join("same.pdf"))
        .expect("建得了链接");
    assert_eq!(
        pin(&workspace, Path::new("same.pdf"), &outbox),
        Err(Refused::Link)
    );
    assert!(std::fs::read_dir(&outbox).map_or(true, |mut left| left.next().is_none()));
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[cfg(unix)]
#[test]
fn a_file_swapped_for_a_link_after_the_check_is_not_copied_through_it() {
    // core1 11-10 审出来的时间差：判的时候是普通文件，钉的时候换成了指向 `~/.ssh/id_rsa` 这类的链接。钉的那一步（`taken`）不跟
    // 链接：拿到的已经是判过的位置，换成链接的照样不拷，`outbox/` 里没有外面那份的内容。
    let (root, workspace, outbox) = setup("swapped");
    let at = workspace.join("out/report.pdf");
    std::fs::remove_file(&at).expect("删得掉");
    std::os::unix::fs::symlink(root.join("secret.txt"), &at).expect("建得了链接");
    assert_eq!(taken(&at, &outbox), Err(Refused::Link));
    let leaked = std::fs::read_dir(&outbox)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| std::fs::read(entry.path()).is_ok_and(|bytes| bytes == b"key"));
    assert!(!leaked, "外面那份的内容没拷进 outbox");
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}
