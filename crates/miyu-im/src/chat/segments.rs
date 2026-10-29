//! 一条消息认出来的段（施工 I-2 上从 `im/segments.rs` 挪来）：正文、引用、@全体、带的东西、合并转发。怎么认是驱动的事，
//! 认出来的样子各个平台一样。

/// 合并转发的占位：照核心渲染带的东西的写法 `[种类]`（「施工时定的」第 61 条）。展开的内容另交（施工 O-34），占位照旧写：
/// O-34 以前造的会话只看得到它。
pub const FORWARD: &str = "[forward]";

/// 卡片（`json`、`xml` 段）的占位，同 [`FORWARD`]。
pub const CARD: &str = "[card]";

/// 桥写进正文的占位：只有它们的消息没有字（[`Segments::wordless`]，施工 O-37）。
const PLACEHOLDERS: [&str; 2] = [FORWARD, CARD];

/// 一条消息认出来的段。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Segments {
    /// 正文，照段的先后：字（占位也是字）和 @ 了谁。
    pub pieces: Vec<Piece>,
    /// 引用的那一条的平台编号：几段引用的只认第一段。
    pub reply_to: Option<String>,
    /// @ 了全体成员（`at` 段的 `qq` 是 `all`）：正文里不写（「施工时定的」第 60 条）。
    pub mentions_all: bool,
    /// 带的东西，照段的先后。
    pub media: Vec<Media>,
    /// 合并转发，照段的先后（施工 O-34）：正文里各有一个占位。
    pub forwards: Vec<Forwarded>,
}

/// 一段合并转发（施工 O-34，`onebot.md`「群消息」第 2 条）：NapCat 的 `forward` 段。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Forwarded {
    /// `data.id`：交 `get_forward_msg` 取里面的消息；没有的、空的是空的。
    pub id: Option<String>,
    /// 段里直接带着的里面的消息（NapCat 开了解析合并转发的，套在里面的同样带着）：驱动认好了的节点；没带的是空的。
    pub content: Option<Vec<super::Node>>,
}

/// 正文的一块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// 字，照原样；合并转发、卡片的占位也是这一种。
    Text(String),
    /// @ 了号是它的人：交给核心以前写成 `@名字`（「群消息」第 5 条）。
    At(super::Id),
}

/// 带的一样东西。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
    /// 什么种类。
    pub kind: MediaKind,
    /// 平台的编号：不空。
    pub id: String,
    /// 文件名、表情的字：有的话，去掉首尾空白不空的才有。
    pub name: Option<String>,
    /// 多少字节（施工 O-33）：段里的 `file_size`，非负整数（NapCat 给的是字，整数也认）；没有的、读不出的是空的。
    pub size: Option<u64>,
    /// 怎么去 QQ 取（施工 O-33，`fetch_media`、冲她来的那条的图）：取不了的（语音、小黄脸、`mface` 商城表情）是空的。
    pub fetch: Option<Fetch>,
    /// 段里写的下载地址（施工 I-75）：`url` 是 `http`、`https` 的才有。合并转发里的那几样 `get_image` 不一定认（NapCat 照编号找的是
    /// 外面那一条消息），按需取时先下它；转发里的段是取的时候现读的，地址不会过期。
    pub url: Option<String>,
}

/// 带的东西怎么取（施工 O-33，`onebot.md`「平台工具（二）」）：照 NapCat 的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetch {
    /// 图片段（图、表情包、NapCat 发成图片的商城表情）：`get_image`。
    Image,
    /// 视频、文件：`get_file`。
    File,
}

/// 带的东西的种类，和核心认的五种一样（`venues.md`「场所的格」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// 图片。
    Image,
    /// 文件。
    File,
    /// 语音（`record` 段）。
    Voice,
    /// 视频。
    Video,
    /// 表情：表情包、QQ 的小黄脸、商城表情。
    Sticker,
}

impl MediaKind {
    /// 交给核心的写法。
    pub fn as_str(self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::File => "file",
            MediaKind::Voice => "voice",
            MediaKind::Video => "video",
            MediaKind::Sticker => "sticker",
        }
    }
}

impl Segments {
    /// 没有字（施工 O-37，`onebot.md`「施工时定的」第 79 条）：正文的每一块都是空白或者占位（`[forward]`、`[card]`），没有 @；
    /// 带没带东西、带的什么都不看。没有字的不算刚说过话（`core/route/called.rs` 填进 `Facts::textless`）。
    ///
    /// 占位照「一段字正好是它」认：人自己打的一段字正好是 `[card]` 的也当没有字，分不开，也很少见（「施工时定的」第 221 条）。
    pub fn wordless(&self) -> bool {
        self.pieces.iter().all(|piece| match piece {
            Piece::Text(text) => text.trim().is_empty() || PLACEHOLDERS.contains(&text.as_str()),
            Piece::At(_) => false,
        })
    }

    /// @ 了的号，照先后、不重。
    pub fn at(&self) -> Vec<super::Id> {
        let mut all = Vec::new();
        for piece in &self.pieces {
            if let Piece::At(user) = piece
                && !all.contains(user)
            {
                all.push(user.clone());
            }
        }
        all
    }
}
