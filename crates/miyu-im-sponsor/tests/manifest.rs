//! 清单（施工 I-46）：赞助记录的 `package.toml` 照核心装包时的那一道查读得进（`miyu_config::package::read`）。以前写了
//! `type = "string"`，核心只认 `text`，真核心装不上，测试却照样过：这里替装包把一道关。

#[test]
fn the_shipped_manifest_passes_the_core_check() {
    let text = include_str!("../../../sponsor/package.toml");
    if let Err(problem) = miyu_config::package::read(text) {
        panic!("核心读不进赞助记录的清单：{problem:?}");
    }
}
