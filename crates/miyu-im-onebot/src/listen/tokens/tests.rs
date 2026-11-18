use std::collections::BTreeMap;
use std::time::Duration;

use miyu_client::settings::Secret;

use super::Tokens;

/// 端口 → 令牌的一份表。
fn table(rows: &[(u16, Option<&str>)]) -> BTreeMap<u16, Option<Secret>> {
    rows.iter()
        .map(|(port, token)| (*port, token.map(|token| Secret::new(token).expect("令牌"))))
        .collect()
}

/// `revoked` 在 `within` 以内交回了。
async fn ended(tokens: &Tokens, port: u16, admitted: &str) -> bool {
    tokio::time::timeout(Duration::from_millis(200), tokens.revoked(port, admitted))
        .await
        .is_ok()
}

#[tokio::test]
async fn a_changed_or_cleared_token_revokes_and_an_unchanged_or_closed_port_does_not() {
    let tokens = Tokens::default();
    tokens.set(table(&[(8301, Some("old")), (8302, Some("other"))]));
    assert!(!ended(&tokens, 8301, "old").await, "没换的不断");

    // 别的端口换了：这个端口不断。
    tokens.set(table(&[(8301, Some("old")), (8302, Some("next"))]));
    assert!(!ended(&tokens, 8301, "old").await);

    // 端口不在表里了（关端口）：已经连着的不断。
    tokens.set(table(&[(8302, Some("next"))]));
    assert!(!ended(&tokens, 8301, "old").await, "关端口不算换令牌");

    // 换了：断。
    tokens.set(table(&[(8301, Some("new"))]));
    assert!(ended(&tokens, 8301, "old").await, "换了令牌断开");
    assert!(!ended(&tokens, 8301, "new").await, "照新令牌连进来的不断");

    // 删了：断。
    tokens.set(table(&[(8301, None)]));
    assert!(ended(&tokens, 8301, "new").await, "删了令牌断开");
}

#[tokio::test]
async fn a_change_while_waiting_wakes_the_wait() {
    let tokens = std::sync::Arc::new(Tokens::default());
    tokens.set(table(&[(8301, Some("old"))]));
    let waiting = {
        let tokens = std::sync::Arc::clone(&tokens);
        tokio::spawn(async move { tokens.revoked(8301, "old").await })
    };
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(!waiting.is_finished());
    tokens.set(table(&[(8301, Some("new"))]));
    tokio::time::timeout(Duration::from_secs(5), waiting)
        .await
        .expect("换了就醒")
        .expect("没 panic");
}
