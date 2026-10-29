//! 合并转发里带的东西怎么编号（施工 I-75）：接着自己带的数，照展开的先后、每条先数带的再数套着的；编号不合写法的不数；打不开的
//! 转发到这里为止；按需取照同一个数法找回那一样。

use super::*;
use crate::chat::{Fetch, MediaKind, Segments, fake};

/// 一样图。
fn image(id: &str) -> Media {
    Media {
        kind: MediaKind::Image,
        id: id.to_string(),
        name: None,
        size: None,
        fetch: Some(Fetch::Image),
        url: None,
    }
}

/// 转发里的一条：带的、套着的转发。
fn node(media: &[&str], forwards: Vec<Forwarded>) -> Node {
    Node {
        user: Some(1),
        name: Some("阿杰".to_string()),
        segments: Segments {
            media: media.iter().map(|id| image(id)).collect(),
            forwards,
            ..Segments::default()
        },
    }
}

/// 内联了 `nodes` 的转发。
fn inline(nodes: Vec<Node>) -> Forwarded {
    Forwarded {
        id: Some("fw".to_string()),
        content: Some(nodes),
    }
}

/// 没内联的转发：要调 `get_forward_msg`。
fn bare() -> Forwarded {
    Forwarded {
        id: Some("fw-bare".to_string()),
        content: None,
    }
}

/// 这几个转发里的每一样照先后的编号：照 [`find`] 一个个找回来，找不到的停。
async fn numbers(own: usize, forwards: &[Forwarded]) -> Vec<(usize, String)> {
    let (link, _asked) = fake::fake();
    let mut found = Vec::new();
    for index in own + 1.. {
        let mut numbering = Numbering::after(own);
        match find(link.as_ref(), forwards, index, &mut numbering).await {
            Some(item) => found.push((index, item.id)),
            None => break,
        }
    }
    found
}

#[tokio::test]
async fn numbers_go_on_after_the_message_own_in_the_order_shown() {
    let forwards = [
        inline(vec![
            node(&["a"], vec![inline(vec![node(&["b", "c"], vec![])])]),
            node(&["", "d"], vec![]),
        ]),
        inline(vec![node(&["e"], vec![])]),
    ];
    let found = numbers(2, &forwards).await;
    let want: Vec<(usize, String)> = [(3, "a"), (4, "b"), (5, "c"), (6, "d"), (7, "e")]
        .into_iter()
        .map(|(n, id)| (n, id.to_string()))
        .collect();
    assert_eq!(found, want, "每条先数带的再数套着的；编号是空的那一样不数");
    let mut counted = Numbering::after(2);
    counted.skip(&forwards);
    assert_eq!(
        (counted.done(), counted.stopped()),
        (7, false),
        "只数的一样"
    );
}

#[tokio::test]
async fn an_unopened_forward_ends_the_numbers() {
    let forwards = [
        inline(vec![node(&["a"], vec![bare()]), node(&["b"], vec![])]),
        inline(vec![node(&["c"], vec![])]),
    ];
    let found = numbers(0, &forwards).await;
    assert_eq!(found, [(1, "a".to_string())], "取不到的那个和后面的都不编");
    let mut counted = Numbering::after(0);
    counted.skip(&forwards);
    assert_eq!((counted.done(), counted.stopped()), (1, true));
    assert_eq!(counted.take(), None, "停了的不再编");
    let mut beyond = Numbering::after(0);
    assert!(
        find(fake::fake().0.as_ref(), &forwards, 2, &mut beyond)
            .await
            .is_none()
    );
    assert_eq!(beyond.done(), 1, "停在数到的地方");
}
