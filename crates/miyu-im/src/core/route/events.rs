//! NapCat 那头读出来的事交给哪一段办（从 `route.rs` 挪来：那个文件到了行数上限）。

use super::{Route, request};
use crate::chat::{Event, Platform};
use crate::core::Gone;

impl Route {
    /// 平台 `platform` 的驱动认出来的一件事（施工 I-70 上：拼场所、平台上的人、命令编号，找号都照这件事的平台）。
    pub(super) async fn event(&mut self, platform: Platform, event: Event) -> Result<(), Gone> {
        match event {
            Event::Private(posted) => self.private(platform, posted).await,
            Event::Group { group, posted } => self.group(platform, group, posted).await,
            Event::Backfilled { group, posted } => self.backfilled(platform, group, posted).await,
            Event::Recalled(recall) => self.recalled(platform, recall).await,
            Event::Muted {
                bot,
                group,
                seconds,
                operator,
            } => {
                self.muted(platform, (bot, group), Some(seconds), operator)
                    .await
            }
            Event::Unmuted {
                bot,
                group,
                operator,
            } => self.muted(platform, (bot, group), None, operator).await,
            // 号连上了：排着的照先后发（施工 O-25 中，「出站队列」第 3 条）。
            Event::Connected { .. } => self.pump().await,
            Event::Befriend { bot, user, flag } => self.befriend(platform, bot, user, flag).await,
            Event::Noted {
                bot,
                to,
                kind,
                text,
            } => self.note_built_in((platform, bot), to, kind, &text).await,
            Event::Asked { to, request, reply } => {
                self.ask_model(platform, to, request, reply);
                Ok(())
            }
            Event::Happened(happened) => {
                self.group_happened(platform, happened);
                Ok(())
            }
            Event::Renamed {
                group, user, card, ..
            } => {
                self.renamed(platform, group, user, card);
                Ok(())
            }
            Event::Invited { bot, group, user } => {
                request::invited(bot, group, user);
                Ok(())
            }
        }
    }
}
