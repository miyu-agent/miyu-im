//! 和平台无关的数据（施工 I-2 上，`docs/designs/01-接入通讯平台.md` 第二节、第三节）：驱动认出来交给宿主的、宿主要驱动去做的，
//! 各个平台一样的样子。原来放在 `onebot` 里，那里照旧 `pub use`。

mod bots;
mod calls;
mod event;
mod forwards;
mod link;
mod members;
mod segments;
mod source;

pub use bots::Bots;
pub use calls::{CallError, DETAIL};
pub use event::{Event, Lead, Posted, Recall, To};
pub use forwards::Node;
pub use link::{
    Fetched, Friend, Group, Implementation, Link, Located, Member, Msg, Outgoing, Pending, Reply,
};
pub use members::{Members, Rank};
pub use segments::{CARD, FORWARD, Fetch, Forwarded, Media, MediaKind, Piece, Segments};
pub use source::Source;
