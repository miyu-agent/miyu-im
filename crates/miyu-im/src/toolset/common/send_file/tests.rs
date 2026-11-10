//! 传文件只认工作区里的普通文件、钉住了再交（施工 I-130）：相对的、绝对的都照真实位置比；`..`、指出去的链接、目录、不在的都不认；
//! 钉住以后原文件换成指出去的链接，钉住的那一份照旧是原来的内容（core1 11-04 审出来的时间差）。

use super::*;

/// 照 `/` 拆成几段（测试写路径用）。
fn parts(path: &str) -> Vec<std::ffi::OsString> {
    path.split('/').map(std::ffi::OsString::from).collect()
}

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
    let base = std::fs::canonicalize(&workspace).expect("有工作区");
    assert_eq!(
        taken(&base, &parts("out/report.pdf"), &outbox),
        Err(Refused::Link)
    );
    let leaked = std::fs::read_dir(&outbox)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| std::fs::read(entry.path()).is_ok_and(|bytes| bytes == b"key"));
    assert!(!leaked, "外面那份的内容没拷进 outbox");
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[cfg(unix)]
#[test]
fn a_folder_swapped_for_a_link_is_not_walked_through() {
    // core1 11-10 审出来的第二处：判完到开之间把工作区里的子目录换成指向外面目录（`~/.ssh` 这类）的链接，只管最后一段的
    // `O_NOFOLLOW` 拦不住中间那一段。一级一级往下开、每一级都不跟链接：中间那一段是链接的照样不开，`outbox/` 里没有外面的内容。
    let (root, workspace, outbox) = setup("folder");
    let outer = root.join("outer");
    std::fs::create_dir_all(&outer).expect("建得了");
    std::fs::write(outer.join("id_rsa"), b"private").expect("写得进");
    std::fs::remove_dir_all(workspace.join("out")).expect("删得掉");
    std::os::unix::fs::symlink(&outer, workspace.join("out")).expect("建得了链接");
    let base = std::fs::canonicalize(&workspace).expect("有工作区");
    assert_eq!(
        taken(&base, &parts("out/id_rsa"), &outbox),
        Err(Refused::Link)
    );
    assert_eq!(
        pin(&workspace, Path::new("out/id_rsa"), &outbox),
        Err(Refused::Link)
    );
    let leaked = std::fs::read_dir(&outbox)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| std::fs::read(entry.path()).is_ok_and(|bytes| bytes == b"private"));
    assert!(!leaked, "外面那份的内容没拷进 outbox");
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[test]
fn only_plain_names_below_the_workspace_are_walked() {
    // 绝对路径照系统的写法拼（Windows 上 `/srv/ws` 不算绝对路径）。
    let root = std::env::temp_dir();
    let (workspace, base) = (root.join("srv-ws"), root.join("real-ws"));
    let (workspace, base) = (workspace.as_path(), base.as_path());
    assert_eq!(
        below(workspace, base, Path::new("out/./a.pdf")),
        Some(parts("out/a.pdf"))
    );
    assert_eq!(
        below(workspace, base, &workspace.join("out").join("a.pdf")),
        Some(parts("out/a.pdf"))
    );
    assert_eq!(
        below(workspace, base, &base.join("a.pdf")),
        Some(parts("a.pdf"))
    );
    for bad in ["../a.pdf", "out/../../a.pdf", "/etc/passwd", ".", ""] {
        assert_eq!(below(workspace, base, Path::new(bad)), None, "{bad}");
    }
}

#[cfg(unix)]
#[test]
fn a_named_pipe_is_refused_at_once() {
    // core1 11-10：工作区里 `mkfifo` 一个具名管道再叫发它，阻塞地打开会一直等写的那头、卡死处理工具的那条线。最后一段不阻塞
    // 打开，一秒内答不是文件。
    let (root, workspace, outbox) = setup("fifo");
    let made = std::process::Command::new("mkfifo")
        .arg(workspace.join("pipe"))
        .status()
        .expect("跑得了 mkfifo");
    assert!(made.success(), "建得了管道");
    let (answer, answered) = std::sync::mpsc::channel();
    let (asked_in, asked_out) = (workspace.clone(), outbox.clone());
    std::thread::spawn(move || {
        let refused = pin(&asked_in, Path::new("pipe"), &asked_out);
        if answer.send(refused).is_err() {
            // 等的一头超时走了：不要紧。
        }
    });
    let refused = answered
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("一秒内答了，没卡住");
    assert_eq!(refused, Err(Refused::NoFile));
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}
