//! 查到的发的人填进引用（施工 LW-IM-7）：有引用的填上，没引用的、没有那一条的不动。

use miyu_client::ids::ExternalId;

use super::with_sender;
use crate::toolset::{Origin, Quote};

fn origin(quote: Option<&str>) -> Origin {
    Origin {
        sender: ExternalId::parse("qq:1").expect("合写法"),
        quote: quote.map(|msg| Quote {
            msg: msg.to_string(),
            sender: None,
        }),
        mentions: Vec::new(),
    }
}

#[test]
fn a_found_sender_fills_the_quote_only() {
    let found = ExternalId::parse("qq:7").ok();
    let filled = with_sender(Some(origin(Some("42"))), found.clone()).expect("还在");
    assert_eq!(filled.quote.expect("有引用").sender, found);
    let bare = with_sender(Some(origin(None)), found.clone()).expect("还在");
    assert!(bare.quote.is_none());
    assert!(with_sender(None, found).is_none());
}
