//! 正在传的文件（施工 I-231）：同一份还在传的不再传，传完了能再传。

use super::*;

#[test]
fn the_same_file_is_not_uploaded_twice_until_the_first_is_done() {
    let uploading = Uploading::default();
    let first = uploading.start("qq:1:group:31", "src.zip", "26.8 MB");
    assert!(first.is_some());
    assert!(
        uploading
            .start("qq:1:group:31", "src.zip", "26.8 MB")
            .is_none(),
        "还在传"
    );
    assert!(
        uploading
            .start("qq:1:group:32", "src.zip", "26.8 MB")
            .is_some(),
        "别的场所照传"
    );
    assert!(
        uploading
            .start("qq:1:group:31", "src.zip", "1.0 KB")
            .is_some(),
        "同名不同大小的照传"
    );
    drop(first);
    assert!(
        uploading
            .start("qq:1:group:31", "src.zip", "26.8 MB")
            .is_some(),
        "传完了能再传"
    );
}
