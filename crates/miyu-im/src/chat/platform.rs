//! 平台的名字和照它拼的编号（施工 I-2 下从 `onebot.rs` 挪来，`onebot.md` 第一条「怎么走」第 7、8 条）：场所、平台上的人、命令
//! 编号的头一段都是平台的名字。名字由驱动给（OneBot 是 `qq`），宿主不写死。场所、平台上的人经群聊内核拼，不手拼（`chat.md`
//! 第七条第 1 条）。号照原样写（施工 I-79：号是字）。

use miyu_chat::{Venue, VenueKind};
use miyu_client::events::FormatError;
use miyu_client::ids::{ExternalId, VenueId};

use super::{Id, To};

/// 一个平台：名字是驱动给的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Platform(&'static str);

impl Platform {
    /// 名字是 `name` 的平台。
    pub const fn new(name: &'static str) -> Platform {
        Platform(name)
    }

    /// 平台的名字。
    pub fn name(self) -> &'static str {
        self.0
    }

    /// 机器人号 `bot` 和号 `user` 的私聊这个场所：`<平台>:<机器人号>:private:<号>`。
    ///
    /// # Errors
    ///
    /// 群聊内核拼不出来，报它的 [`FormatError`]。号已经查过，照说不会；用的一方照实记下，不 `unwrap`。
    pub fn private_venue(self, bot: &Id, user: &Id) -> Result<VenueId, FormatError> {
        Ok(self.venue(bot, VenueKind::Private, user)?.id().clone())
    }

    /// 机器人号 `bot` 收发的、种类是 `kind`、号是 `number`（私聊的对方、群号）的场所（施工 O-22：套场所规则要 [`Venue`]，编号是
    /// 它的 [`Venue::id`]；施工 I-81：编号带机器人号，同一个平台几个号各是各的场所）。
    ///
    /// # Errors
    ///
    /// 同 [`Platform::private_venue`]。
    pub fn venue(self, bot: &Id, kind: VenueKind, number: &Id) -> Result<Venue, FormatError> {
        Venue::with_bot(self.0, bot.as_str(), kind, number.as_str())
    }

    /// 平台上号是 `user` 的那个人：`<平台>:<号>`（`venue.session` 的 `peer`、`session.send` 的 `as.external`）。
    ///
    /// # Errors
    ///
    /// 同 [`Platform::private_venue`]。
    pub fn person(self, user: &Id) -> Result<ExternalId, FormatError> {
        miyu_chat::person(self.0, user.as_str())
    }

    /// 机器人号 `bot` 收进来的第 `message_id` 条消息、平台给的时刻是 `time`，发进去用的命令编号：
    /// `<平台>:<机器人的号>:<消息编号>:<时刻>`。带上时刻：NapCat 重置本地库以后消息编号会重号，同一个编号、不同的时刻是两条，
    /// 新消息不会被当成重发吞掉。斜杠命令交 `command.run` 也用它（O-19）：一条消息要么是命令、要么是话，编号只用一次。
    pub fn command_id(self, bot: &Id, message_id: i64, time: i64) -> String {
        format!("{}:{bot}:{message_id}:{time}", self.0)
    }

    /// 命令编号 `id` 是哪个机器人号收进来的：[`Platform::command_id`] 拼的第二段（施工 O-32：桥起来时照日志里最近一条人话认）。
    /// 后面加了 `/decided` 这类的也认。别的平台、不是这样拼的（自己编的）是空的。
    pub fn bot_of(self, id: &str) -> Option<Id> {
        let mut parts = id.split(':');
        if parts.next()? != self.0 {
            return None;
        }
        Id::new(parts.next()?)
    }

    /// 场所 `venue` 发到哪（施工 O-32）：这个平台的，私聊发给对方、群发进群；别的平台、号不是好好的号的是空的。
    pub fn to_of(self, venue: &Venue) -> Option<To> {
        if venue.platform() != self.0 {
            return None;
        }
        let number = Id::new(venue.number())?;
        Some(match venue.kind() {
            VenueKind::Private => To::Private(number),
            VenueKind::Group => To::Group(number),
        })
    }
}
