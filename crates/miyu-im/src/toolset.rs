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
use std::pin::Pin;
use std::sync::Arc;

use miyu_client::ids::ExternalId;
use miyu_client::protocol::Template;
use miyu_client::tooltext::{Access, load};
use serde_json::{Value, json};

use crate::chat::{Link, Platform, To};

/// 驱动的工具常用的几样，im 自己也用：照来源拿字节、存进工作区的 `qq-files/`、照字节认扩展名、照编号取回一条消息（施工 T-2：撤回
/// 先核对）。
pub use crate::media::{Missed, QQ_FILES, bytes, extension, message, save};

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

/// 交给 im 等着的那一段：办完交回 `tool.call` 回应的 `result`（`{blocks, error}`）。
pub type Pending = Pin<Box<dyn Future<Output = Value> + Send>>;

/// 一次调用怎么办（[`ToolSet::plan`]）。
pub enum Planned {
    /// 当场答：`tool.call` 回应的 `result`。
    Now(Value),
    /// 去平台上办：im 另起任务等它办完再答。
    Later(Pending),
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
    /// 那个机器人号这一刻的连接；断了的是空的。
    pub link: Option<Arc<dyn Link>>,
    /// 答的话（[`ToolSet::answers`] 里的那几句）。
    pub answers: Arc<Answers>,
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
        let results = format!("{dir}/tool-results");
        let mut templates = HashMap::new();
        let mut broken = Vec::new();
        for (name, fields) in wanted {
            match load::text_in(data, &results, name, fields) {
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
        let template = self
            .templates
            .get(name)
            .expect("答的话在 ToolSet::answers 里");
        json!({"blocks": [{"type": "text", "text": load::say(template, fields)}], "error": error})
    }
}
