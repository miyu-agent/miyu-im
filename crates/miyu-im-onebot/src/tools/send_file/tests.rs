//! 传文件只认工作区里的普通文件（施工 I-130）：相对的、绝对的都照真实位置比；`..`、指出去的链接、目录、不在的都不认。

use super::*;

/// 一个临时的工作区（`name` 是测试自己的名字），里面一份 `report.pdf`；工作区外面一份 `secret.txt`。
fn setup(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("im-send-file-{}-{name}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).expect("删得掉上一次留下的");
    }
    let workspace = root.join("ws");
    std::fs::create_dir_all(workspace.join("out")).expect("建得了");
    std::fs::write(workspace.join("out/report.pdf"), b"%PDF-1.7").expect("写得进");
    std::fs::write(root.join("secret.txt"), b"key").expect("写得进");
    (root, workspace)
}

#[test]
fn a_file_in_the_workspace_is_found_by_relative_or_absolute_path() {
    let (root, workspace) = setup("inside");
    let real = std::fs::canonicalize(workspace.join("out/report.pdf")).expect("在");
    assert_eq!(
        within(&workspace, Path::new("out/report.pdf")),
        Some((real.clone(), 8))
    );
    assert_eq!(
        within(&workspace, &workspace.join("out/report.pdf")),
        Some((real, 8))
    );
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[test]
fn anything_outside_the_workspace_or_not_a_file_is_refused() {
    let (root, workspace) = setup("outside");
    for asked in [
        Path::new("../secret.txt"),
        &root.join("secret.txt"),
        Path::new("out"),
        Path::new("missing.pdf"),
    ] {
        assert_eq!(within(&workspace, asked), None, "{}", asked.display());
    }
    assert_eq!(
        within(Path::new(""), Path::new("out/report.pdf")),
        None,
        "没给工作区的不认"
    );
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}

#[cfg(unix)]
#[test]
fn a_link_pointing_out_of_the_workspace_is_refused() {
    let (root, workspace) = setup("link");
    std::os::unix::fs::symlink(root.join("secret.txt"), workspace.join("looks-fine.txt"))
        .expect("建得了链接");
    assert_eq!(within(&workspace, Path::new("looks-fine.txt")), None);
    std::fs::remove_dir_all(root).expect("删得掉临时目录");
}
