//! 驱动带的工具（施工 T-1，2026-10-20 项目主人定：不同平台的工具最终调的接口可能完全不一样，平台的工具在驱动里，im 不提供工具）：
//! 驱动交一套工具（[`ToolSet`]），im 照它读说明、登记给核心、把核心转来的调用交给它。
//!
//! - 说明照包文件夹 `data/<目录>/tools/<名字>.json`、答的话照 `data/<目录>/tool-results/<名字>.txt`（目录是 [`ToolSet::dir`]，例如
//!   `onebot`），和出厂的别的数据一起读、查过（`crate::rules::Factory::load`），写坏了是打包的错，桥起不来。
//! - 一次调用（[`Call`]）：im 照会话填好场景（发到哪、收进这个会话的机器人号、谁叫的、工作区、那个机器人号的连接），交给
//!   [`ToolSet::plan`]：当场就答得了的交回 [`Planned::Now`]，要去平台上办的交回 [`Planned::Later`]，im 另起任务等它办完再答，跟核心的
//!   那一头不等。

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use miyu_client::ids::ExternalId;
use miyu_client::protocol::Template;
use miyu_client::time::{Timestamp, UtcOffset};
use miyu_client::tooltext::{Access, load};
use serde_json::{Map, Value, json};

use crate::chat::{Link, Media, Platform, To};
use crate::core::Caller;
use crate::core::blobs::{Unstored, put};

/// 驱动的工具常用的几样，im 自己也用：照来源拿字节、存进工作区的 `qq-files/`、照字节认扩展名、照编号取回一条消息（施工 T-2：撤回
/// 先核对）。
pub use crate::media::{Missed, QQ_FILES, bytes, extension, file_name, message, save, take};

/// 叫她做的那一条（施工 T-2，原来路由的 `platform::Origin`）：群里是投影里调用所在那条线这一轮她回的那一条，私聊是最近一条人说的话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// 发它的人。
    pub sender: ExternalId,
    /// 它引用的那一条；没引用的是空的。
    pub quote: Option<Quote>,
    /// 它 @ 了谁（不含她：群消息记的时候就去掉了），照先后。
    pub mentions: Vec<ExternalId>,
}

/// 被引用的那一条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    /// 平台编号，照引用里的原样。
    pub msg: String,
    /// 发它的人：投影里认得出的；她的、桥不认识的是空的。
    pub sender: Option<ExternalId>,
}

/// 她给的消息编号 `value`：整数，或者写成十进制整数的字（最多 20 位，前面可以有负号），交回写成字的；别的是空的（取图、撤回的
/// `msg` 照它读）。
pub fn msg_id(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) if number.is_i64() => Some(number.to_string()),
        Value::String(text) if integer(text) => Some(text.clone()),
        _ => None,
    }
}

/// 是不是十进制整数：可以有一个负号，1 到 20 位数字。
fn integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    (1..=20).contains(&digits.len()) && digits.bytes().all(|byte| byte.is_ascii_digit())
}

/// 主动发消息一次最多多少个字符：拆段以前。
const SEND_MOST: usize = 4000;

/// 读 `chat_send`、`chat.send` 的参数（纯逻辑）：发到哪（`user`、`group` 写正好一个，正整数，写成字的也认），发什么（去掉首尾
/// 空白不空、不超过 4000 个字符）。写错的是空的。
pub fn send_request(args: &Value) -> Option<(To, String)> {
    let number = |value: &Value| match value {
        Value::Null => Some(None),
        Value::Number(number) => number.as_i64().filter(|n| *n > 0).map(Some),
        Value::String(text) => text.trim().parse::<i64>().ok().filter(|n| *n > 0).map(Some),
        _ => None,
    };
    let to = match (number(&args["user"])?, number(&args["group"])?) {
        (Some(user), None) => To::Private(user),
        (None, Some(group)) => To::Group(group),
        _ => return None,
    };
    let text = args["text"].as_str()?.trim();
    let fits = !text.is_empty() && text.chars().count() <= SEND_MOST;
    fits.then(|| (to, text.to_string()))
}

/// Unix 秒 `at` 照本机此刻的时区写到分钟：`2026-10-20 14:03`（施工 Q-4：管理记录的时刻；时区照核心给会话钉时区的算法）。读不成的
/// 照 UTC。
pub fn clock(at: u64) -> String {
    let minutes = jiff::Zoned::now().offset().seconds() / 60;
    let offset = UtcOffset::from_minutes(minutes).unwrap_or(UtcOffset::UTC);
    i64::try_from(at)
        .ok()
        .and_then(|at| at.checked_mul(1000))
        .and_then(Timestamp::from_unix_millis)
        .map(|at| at.local_minute(offset))
        .unwrap_or_default()
}

/// 多久，照人的单位写：天、时、分、秒各一段，`1d 2h 30m 5s`，0 的一段不写（禁言、她自己被禁言的事实都照它）。
pub fn duration(seconds: u64) -> String {
    let parts = [
        (seconds / 86_400, "d"),
        (seconds % 86_400 / 3_600, "h"),
        (seconds % 3_600 / 60, "m"),
        (seconds % 60, "s"),
    ];
    let written: Vec<String> = parts
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, unit)| format!("{count}{unit}"))
        .collect();
    written.join(" ")
}

/// 发的人的名字最长几个字符（`name`）。
pub const NAME: usize = 64;

/// 平台的编号最长几个字符（`venues.md`「场所的格」的 `msg`、`reply_to`，带的东西的 `id`）。
const ID: usize = 128;

/// 一个平台的编号合不合核心的写法：不空，最多 128 个字符，没有控制字符。
pub fn fits(id: &str) -> bool {
    !id.is_empty() && id.chars().count() <= ID && !id.chars().any(char::is_control)
}

/// 一条消息带的东西里看得到的那几样（和交给核心的 `venue.media` 一样洗过：编号合写法的才算），第几个照它数（施工 T-3：取图片文件
/// 的 `index` 也照它）。
pub fn kept(media: &[Media]) -> Vec<Media> {
    media
        .iter()
        .filter(|item| fits(&item.id))
        .cloned()
        .collect()
}

/// 存 blob 的一头（施工 T-3：取图片文件把图交成工具结果里的图片块）：经桥跟核心的连接分块传，存在管理员名下，核心照会话的属主拷。
#[derive(Clone)]
pub struct Blobs(pub(crate) Caller);

/// 存了一张图的结果。
#[derive(Debug, Clone, PartialEq)]
pub enum Stored {
    /// 存成了图：工具结果里的图片块（`type`、`blob`、`name`、`media_type`、`width`、`height`）。
    Image(Value),
    /// 存不成图（太大、不是图、核心拒了）：照文件存。
    NotImage,
    /// 核心断开了：不用答了。
    Gone,
}

impl Blobs {
    /// 把 `bytes` 存成名字是 `name` 的 blob，看它是不是图（模块的说明）。
    pub async fn image(&self, name: &str, bytes: &[u8]) -> Stored {
        match put(&self.0, name, bytes).await {
            Ok(stored) if stored["kind"] == "image" => {
                let mut block = json!({"type": "image"});
                for key in ["blob", "name", "media_type", "width", "height"] {
                    block[key] = stored[key].clone();
                }
                Stored::Image(block)
            }
            Ok(_) | Err(Unstored::Refused(_)) => Stored::NotImage,
            Err(Unstored::Gone) => Stored::Gone,
        }
    }
}

/// 洗一个名字：去掉控制字符，截到 `most` 个字符（不截断一个字）；去掉首尾空白是空的交回空的。
pub fn clean(name: &str, most: usize) -> Option<String> {
    let cleaned: String = name
        .chars()
        .filter(|char| !char.is_control())
        .take(most)
        .collect();
    (!cleaned.trim().is_empty()).then_some(cleaned)
}

/// 一件工具：名字、访问类别、给哪种会话（`providers.md` 的 `venues`：`private`、`group`）。
#[derive(Debug, Clone)]
pub struct Decl {
    /// 名字（给模型看的，同说明文件的名字）。
    pub name: &'static str,
    /// 访问类别。
    pub access: Access,
    /// 给哪种会话。
    pub venues: &'static [&'static str],
}

/// 交给 im 等着的那一段：办完交回 [`Done`]。
pub type Pending = Pin<Box<dyn Future<Output = Done> + Send>>;

/// 办完的：`tool.call` 回应的 `result`（`{blocks, error}`），另交给 im 做的一件事（施工 T-4：im 先做它，再答）。
#[derive(Debug, Clone, PartialEq)]
pub struct Done {
    /// 答的。
    pub answer: Value,
    /// 交给 im 做的；没有的是空的。
    pub effect: Option<Effect>,
}

impl From<Value> for Done {
    fn from(answer: Value) -> Done {
        Done {
            answer,
            effect: None,
        }
    }
}

/// 要 im 做的事（施工 T-4）：路由手里的、各个平台都一样的那几样。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// 这条线这一轮她的下一句第一段前面 @ 这几个号（`mention`）。
    Mention(Vec<i64>),
    /// 以她的名义发到 `to`（`chat_send`）：im 照对方场所的出站链发、找对方的会话记一句，发成了答 `sent`，没成的照原因答（`failed`、
    /// `unreachable`、`unanswered`、`no-target`，用这一套的答的话）。
    Send {
        /// 发到哪。
        to: To,
        /// 发什么（去掉首尾空白、不空）。
        text: String,
    },
}

/// 一次调用怎么办（[`ToolSet::plan`]）。
pub enum Planned {
    /// 当场答：`tool.call` 回应的 `result`。
    Now(Value),
    /// 去平台上办：im 另起任务等它办完，先做交回的那件事，再答。
    Later(Pending),
    /// 交给 im 当场做（施工 T-4）：做完 im 自己答。
    Host(Effect),
}

impl Planned {
    /// 去平台上办，办完只答话（没有要 im 做的事）。
    pub fn later(pending: impl Future<Output = Value> + Send + 'static) -> Planned {
        Planned::Later(Box::pin(async move { Done::from(pending.await) }))
    }
}

/// 一次调用和它的场景（im 照会话填好）。
#[derive(Clone)]
pub struct Call {
    /// 工具名。
    pub tool: String,
    /// 参数（原样）。
    pub args: Value,
    /// 场所的编号（`qq:group:777`），记运行日志用。
    pub venue: String,
    /// 发到哪。
    pub to: To,
    /// 收进这个会话的机器人号。
    pub bot: i64,
    /// 叫她的人的号：外部身份的照 `by.id`，私聊里认出的终端管理员照 `by.via`；私聊里没有的是对方；本机的人、没有触发的是空的。
    pub caller: Option<i64>,
    /// 叫她的人在平台上的身份（同上，施工 T-2）：私聊里没有的照空的，不补对方。
    pub by: Option<ExternalId>,
    /// 叫她做的那一条（施工 T-2）；认不出的是空的。
    pub origin: Option<Origin>,
    /// 她自己（收进这个会话的机器人号在平台上的身份）。
    pub me: Option<ExternalId>,
    /// 私聊的对方；群的是空的。
    pub other: Option<ExternalId>,
    /// 平台：号照它拼成平台上的人。
    pub platform: Platform,
    /// 这个群缓存着的成员名字（号 → 名字，洗过）；私聊的是空的。
    pub names: HashMap<i64, String>,
    /// 这一轮的工作目录（`tool.call` 的 `cwd`）：存东西照它。
    pub cwd: String,
    /// 调用所在那条线（会话编号，主线或者支线，施工 T-3）。
    pub line: String,
    /// 这条线这一轮这件工具在这一次以前调过几次（施工 T-3：一轮最多几次由驱动照它挡；群照投影里这条线的回合，私聊照推来的事件的
    /// 回合，换了回合从头数）。
    pub used: usize,
    /// 存 blob 的一头（施工 T-3）。
    pub blobs: Blobs,
    /// 那个机器人号这一刻的连接；断了的是空的。
    pub link: Option<Arc<dyn Link>>,
    /// 答的话（[`ToolSet::answers`] 里的那几句）。
    pub answers: Arc<Answers>,
    /// 在这个场所开着的驱动带的插件的编号（施工 Q-1：总开关、场所开关都开着的，`plugins::Builtin`）。
    pub plugins: Vec<&'static str>,
    /// 驱动这一刻的配置（施工 Q-1：没写的照清单的默认补上了）。
    pub config: Map<String, Value>,
    /// 驱动自己的状态目录（施工 Q-2：im 的状态目录下照平台的名字，`qq/`）：驱动要记的东西放这里，没建的自己建。
    pub store: PathBuf,
}

/// 驱动交的一套工具。
pub trait ToolSet: Send + Sync {
    /// 说明、答的话在包文件夹 `data/` 下的哪个目录。
    fn dir(&self) -> &'static str;

    /// 有哪几件，照登记的先后。
    fn tools(&self) -> &'static [Decl];

    /// 答的话：名字（`tool-results/<名字>.txt`）和认的字段。
    fn answers(&self) -> &'static [(&'static str, &'static [&'static str])];

    /// 认一次调用：当场答的、交 im 等着的（模块的说明）。在跟核心的那一头当场调，不许等平台。
    fn plan(&self, call: Call) -> Planned;

    /// 驱动带的插件给场所会话记一句时用的模板（施工 Q-3）：名字（`facts/<名字>.txt`）和认的字段。经 `Gate::note` 换好交给 im 记。
    fn facts(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &[]
    }

    /// 驱动带的插件问模型时用的提示词模板（施工 Q-7）：名字（`prompts/<名字>.txt`）和认的字段。经 `Gate::prompt` 换。
    fn prompts(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &[]
    }

    /// 驱动的配置是 `config` 时登记不登记 `tool`（施工 Q-1：单件工具的开关）。配置变了 im 照它重新登记，登记的变了的会话下一个回合
    /// 换工具表。都登记的不用写。
    fn offers(&self, tool: &str, config: &Map<String, Value>) -> bool {
        let _ = (tool, config);
        true
    }

    /// 哪一件是「这一轮不说话」（施工 T-4，`providers.md`「提供者和不说话」）：读的一头当场答它（答的话 `skipped`，带 `end_turn`），
    /// 她的回复里有这件的调用块的，这一轮写的都不发。没有的是空的。
    fn silence(&self) -> Option<&'static str> {
        None
    }
}

/// 读好、查过的答的话。
#[derive(Debug)]
pub struct Answers {
    templates: HashMap<&'static str, Template>,
}

impl Answers {
    /// 读包文件夹的数据目录 `data` 下 `dir/tool-results/` 里 `wanted` 那几句：读不成的交回（文件相对 `data/` 的位置、原因）。
    ///
    /// # Errors
    ///
    /// 读不成、写坏了、要了别的字段的那几份。
    pub fn load(
        data: &std::path::Path,
        dir: &str,
        wanted: &[(&'static str, &'static [&'static str])],
    ) -> Result<Answers, Vec<(String, String)>> {
        Answers::load_in(data, &format!("{dir}/tool-results"), wanted)
    }

    /// 同 [`Answers::load`]，读的是 `data/<results>/` 下的（施工 Q-3：驱动的事实模板在 `<目录>/facts/`）。
    ///
    /// # Errors
    ///
    /// 读不成、写坏了、要了别的字段的那几份。
    pub fn load_in(
        data: &std::path::Path,
        results: &str,
        wanted: &[(&'static str, &'static [&'static str])],
    ) -> Result<Answers, Vec<(String, String)>> {
        let mut templates = HashMap::new();
        let mut broken = Vec::new();
        for (name, fields) in wanted {
            match load::text_in(data, results, name, fields) {
                Ok(template) => {
                    templates.insert(*name, template);
                }
                Err(error) => broken.push((format!("{results}/{name}.txt"), error.why)),
            }
        }
        match broken.is_empty() {
            true => Ok(Answers { templates }),
            false => Err(broken),
        }
    }

    /// 答的话 `name`，字段照 `fields` 换，`error` 照写：`tool.call` 回应的 `result`。
    ///
    /// # Panics
    ///
    /// `name` 不是 [`ToolSet::answers`] 里的、少了它要的字段：是 bug（名字、字段都写在驱动的代码里，测试走得到）。
    pub fn say(&self, name: &str, fields: &[(&str, &str)], error: bool) -> Value {
        json!({"blocks": [{"type": "text", "text": self.text(name, fields)}], "error": error})
    }

    /// 那一句 `name` 的字，字段照 `fields` 换（施工 Q-3：事实的模板也照它换）。
    ///
    /// # Panics
    ///
    /// 同 [`Answers::say`]。
    pub fn text(&self, name: &str, fields: &[(&str, &str)]) -> String {
        let template = self.templates.get(name).expect("那一句在驱动报的名字里");
        load::say(template, fields)
    }
}
