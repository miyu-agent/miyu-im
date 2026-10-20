//! 群文件的列法、存成的名字（施工 I-12）。

use miyu_im::chat::GroupFile;

use super::*;

#[test]
fn folders_come_first_and_files_say_their_size_and_uploader() {
    let files = GroupFiles {
        folders: vec![("/a1".to_string(), "资料".to_string(), 3)],
        files: vec![
            GroupFile {
                id: "/f1".to_string(),
                name: "排班.pdf".to_string(),
                size: 1_258_291,
                uploader: Some("小明".to_string()),
            },
            GroupFile {
                id: "/f2".to_string(),
                name: "a\nb.txt".to_string(),
                size: 12,
                uploader: None,
            },
        ],
    };
    assert_eq!(
        listed(&files, 50),
        (
            3,
            3,
            "[folder] 资料 (id=/a1, 3 files); 排班.pdf (id=/f1, 1.2 MB, by 小明); ab.txt (id=/f2, 12 B)".to_string()
        )
    );
    assert_eq!(listed(&files, 1).0, 1);
    assert_eq!(listed(&GroupFiles::default(), 50).0, 0);
}

#[test]
fn sizes_and_saved_names_read_well() {
    assert_eq!(size(512), "512 B");
    assert_eq!(size(2048), "2.0 KB");
    assert_eq!(size(3 << 30), "3.0 GB");
    assert_eq!(
        saved_name(None, "https://x.invalid/d?fname=排班.pdf&k=1", "/f1"),
        "排班.pdf"
    );
    assert_eq!(
        saved_name(None, "https://x.invalid/d?fname=../../etc/passwd", "/f1"),
        "passwd"
    );
    // 列表里的名字优先（SnowLuma 的 fname 是编号）；fname 就是编号的不算。
    assert_eq!(
        saved_name(Some("排班.pdf"), "https://x.invalid/d?fname=/f1", "/f1"),
        "排班.pdf"
    );
    assert_eq!(
        saved_name(None, "https://x.invalid/d?fname=/f1", "/f1"),
        "group-file-f1"
    );
    assert_eq!(
        saved_name(Some("../x/a.md"), "https://x.invalid/d", "/f1"),
        "a.md"
    );
    // 平台给的名字是 `%XX` 写的：解开再洗，解出来的路径照样去掉。
    assert_eq!(
        saved_name(
            None,
            "https://x.invalid/d?fname=%E6%8E%92%E7%8F%AD.pdf",
            "/f1"
        ),
        "排班.pdf"
    );
    assert_eq!(
        saved_name(
            None,
            "https://x.invalid/d?fname=%2E%2E%2F%2E%2E%2Fpasswd",
            "/f1"
        ),
        "passwd"
    );
    assert_eq!(
        saved_name(None, "https://x.invalid/d?fname=100%25%zz", "/f1"),
        "100%%zz"
    );
    assert_eq!(
        saved_name(None, "https://x.invalid/d?fname=..", "/f1"),
        "group-file-f1"
    );
    assert_eq!(
        saved_name(None, "https://x.invalid/d", "/abc-123"),
        "group-file-abc123"
    );
}
