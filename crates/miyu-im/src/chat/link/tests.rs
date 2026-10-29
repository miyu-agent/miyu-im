//! 驱动没写的那几样默认答「做不到」（施工 I-82）：接一个新平台只写发、对端是谁、关掉，别的都有默认。

use super::*;

/// 只会发的一条连接。
#[derive(Debug)]
struct Bare;

impl Link for Bare {
    fn begin<'a>(&'a self, _: To, _: Outgoing<'a>) -> Reply<'a, Pending> {
        Box::pin(async {
            let sent: Pending = Box::pin(async { Ok(Some("1".to_string())) });
            Ok(sent)
        })
    }

    fn implementation(&self) -> Option<Implementation> {
        None
    }

    fn close(&self) {}
}

#[tokio::test]
async fn what_a_driver_leaves_out_is_unsupported() {
    let (bare, to, id) = (Bare, To::Group(Id::from(1)), Id::from(2));
    let lead = Lead::default();
    let sent = bare
        .send(
            to.clone(),
            Outgoing::Text {
                text: "在",
                lead: &lead,
            },
        )
        .await;
    assert_eq!(sent, Ok(Some("1".to_string())), "发照它写的");
    assert_eq!(bare.recall(&to, "1").await, Err(CallError::Unsupported));
    assert_eq!(
        bare.react(&to, "1", "76", true).await,
        Err(CallError::Unsupported)
    );
    assert_eq!(bare.message(&to, "1").await, Err(CallError::Unsupported));
    assert_eq!(bare.mute(&id, &id, 60).await, Err(CallError::Unsupported));
    assert_eq!(
        bare.poke(to.clone(), &id).await,
        Err(CallError::Unsupported)
    );
    assert_eq!(bare.members(&id).await, Err(CallError::Unsupported));
    assert_eq!(bare.groups().await, Err(CallError::Unsupported));
    assert_eq!(bare.nickname().await, Err(CallError::Unsupported));
    assert_eq!(
        bare.avatar(&Face::User(id.clone())).await,
        Err(CallError::Unsupported)
    );
    assert_eq!(
        bare.raw("x", serde_json::json!({})).await,
        Err(CallError::Unsupported)
    );
}
