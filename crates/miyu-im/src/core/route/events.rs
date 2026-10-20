//! NapCat 那头读出来的事交给哪一段办（从 `route.rs` 挪来：那个文件到了行数上限）。

use super::{Route, request};
use crate::chat::Event;
use crate::core::Gone;

impl Route {
    /// NapCat 那头读出来的一件事。
    pub(super) async fn event(&mut self, event: Event) -> Result<(), Gone> {
        match event {
            Event::Private(posted) => self.private(posted).await,
            Event::Group { group, posted } => self.group(group, posted).await,
            Event::Backfilled { group, posted } => self.backfilled(group, posted).await,
            Event::Recalled(recall) => self.recalled(recall).await,
            Event::Muted {
                bot,
                group,
                seconds,
                operator,
            } => self.muted((bot, group), Some(seconds), operator).await,
            Event::Unmuted {
                bot,
                group,
                operator,
            } => self.muted((bot, group), None, operator).await,
            // 号连上了：排着的照先后发（施工 O-25 中，「出站队列」第 3 条）。
            Event::Connected { .. } => self.pump().await,
            Event::Befriend { bot, user, flag } => self.befriend(bot, user, flag).await,
            Event::Noted {
                bot,
                to,
                kind,
                text,
            } => self.note_built_in(bot, to, kind, &text).await,
            Event::Asked { to, request, reply } => {
                self.ask_model(to, request, reply);
                Ok(())
            }
            Event::Invited { bot, group, user } => {
                request::invited(bot, group, user);
                Ok(())
            }
        }
    }
}
