//! 和平台无关的数据（施工 I-2 上，`docs/designs/01-接入通讯平台.md` 第二节、第三节）：驱动认出来交给宿主的、宿主要驱动去做的，
//! 各个平台一样的样子。原来放在 `onebot` 里，那里照旧 `pub use`。

mod bots;
mod calls;
mod cap;
mod event;
#[cfg(any(test, feature = "fake"))]
pub mod fake;
mod forwards;
mod id;
mod link;
mod members;
mod platform;
mod platforms;
mod segments;
mod source;

pub use bots::Bots;
pub use calls::{CallError, DETAIL};
pub use cap::{CAPS, Cap};
pub use event::{Event, GroupChange, Happened, Lead, Posted, Recall, Replier, To};
pub use forwards::Node;
pub use id::Id;
pub use link::{
    Face, Fetched, Friend, Group, GroupFile, GroupFiles, Implementation, Link, Located, Member,
    Outgoing, Pending, Reply, unsupported,
};
pub use members::{Members, Rank};
pub use platform::Platform;
pub use platforms::{Platforms, Unclaimed};
pub use segments::{CARD, FORWARD, Fetch, Forwarded, Media, MediaKind, Piece, Segments};
pub use source::Source;
