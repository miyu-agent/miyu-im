//! 宿主和驱动之间（施工 I-2 下，设计 01 第三节）：驱动是一个平台的接法（OneBot 的反向 WebSocket……）。宿主照驱动报的名字把
//! 它的配置项挑出来交给它（施工 I-3），叫它起来（[`Driver::start`]）；它从 [`Gate`] 拿宿主给的几样：它的配置、连着的号登记在哪、
//! 认出来的事交给谁、桥自己的数、说给人听的、状态报给谁、平台事件交给挂接点。宿主不依赖驱动，驱动只经这里碰宿主。

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value};
use tokio::sync::{mpsc, watch};

use crate::changed::Changed;
use crate::chat::{Bots, Cap, Event, Platform, Replier, To};
use crate::core::hooks::Hooks;
use crate::current::Current;
use crate::plugins::Builtin;
use crate::serve::{Failure, Notice};
use crate::toolset::Answers;
use crate::tuning::Tuning;

/// 驱动起来以后跑着的那一段：接连接、配置变了自己换，桥停下时随宿主的任务组一起放下。
pub type Running = Pin<Box<dyn Future<Output = ()> + Send>>;

/// 起来的结局（[`Driver::start`]）。
pub type Started = Pin<Box<dyn Future<Output = Result<Running, Failure>> + Send>>;

/// 能贴的一个表情（施工 I-7 中三下二：会话浮窗「确认主动回复后贴表情」旁边选哪个，列表由驱动给）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reaction {
    /// 编号：贴的时候交给平台的，规则 `reaction` 也写它。
    pub id: &'static str,
    /// 平台自己的叫法。
    pub name: &'static str,
    /// 画得出来的那个字（Unicode 表情）；平台自带的图标没有。
    pub glyph: Option<&'static str>,
}

/// 一个平台的接法。
pub trait Driver: Send + Sync {
    /// 平台：场所、平台上的人、命令编号照它的名字拼。
    fn platform(&self) -> Platform;

    /// 这个平台做得到的几样（施工 I-8 上）：宿主照它算插件在这个平台上能不能用。
    fn caps(&self) -> &'static [Cap];

    /// 驱动自己的配置项：清单 `[settings]` 里的名字（施工 I-3）。宿主照它挑出这几键交给驱动（[`Gate::config`]），别的键不给。
    fn settings(&self) -> &'static [&'static str];

    /// [`Driver::settings`] 里哪几项是密钥（施工 I-6 中）：页面存它们时先存成核心的密钥（`im-<平台>-<名字>`），配置里写引用。
    fn secrets(&self) -> &'static [&'static str] {
        &[]
    }

    /// 能贴的表情（施工 I-7 中三下二）：空的是这个平台不支持贴表情，页面上不出那一格。
    fn reactions(&self) -> &'static [Reaction] {
        &[]
    }

    /// 驱动给 AI 的工具（施工 T-1，[`crate::toolset::ToolSet`]；2026-10-20 项目主人定：平台的工具在驱动里，im 不提供工具）：照
    /// `bridge.json` 的参数 `tuning` 造一套；没有的是空的。
    fn toolset(&self, tuning: &Tuning) -> Option<Arc<dyn crate::toolset::ToolSet>> {
        let _ = tuning;
        None
    }

    /// 驱动自己带的 im 插件（施工 I-10 上，[`Builtin`]）：照样是插件（声明、场所开关、插件页），卸不掉；OneBot 带识人术。
    fn builtins(&self) -> Vec<Arc<dyn Builtin>> {
        Vec::new()
    }

    /// 照 [`Gate::config`] 起来（OneBot：开好端口、说听上了），交回跑着的那一段。起不来的（端口被占、听不了、配置里缺必须的
    /// 那一项）交回 [`Failure`]。
    fn start(&self, gate: Arc<Gate>) -> Started;
}

/// 驱动报的状态（施工 I-3）：后台页的 `status`、状态文件照原样并进去（OneBot 的 `listen`、`token`）。
#[derive(Debug, Default)]
pub(crate) struct Reported(Mutex<Map<String, Value>>);

impl Reported {
    /// 最新报的。
    pub(crate) fn get(&self) -> Map<String, Value> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// 换成 `fields`。
    fn set(&self, fields: Map<String, Value>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = fields;
    }
}

/// 宿主给驱动的，各个连接共用。
pub struct Gate {
    /// 桥手里最新的配置：驱动的那几项在这里（和后台页的方法共用，核心推来新的就换上，施工 O-20、I-3）。
    current: Arc<Current>,
    /// 驱动报的状态。
    reported: Arc<Reported>,
    /// 认哪几个路径、调用等多久、写队列多长（`bridge.json`）。
    tuning: Tuning,
    /// 连着的号。
    bots: Arc<Bots>,
    /// 认出来的事交给跟核心的那一头（`core/route.rs`）。
    inbound: mpsc::Sender<Event>,
    /// 连上、断开各说一行（「样子」）。
    tell: Arc<dyn Fn(Notice) + Send + Sync>,
    /// 连上、断开、认出号、问到是哪个实现时叫一声：状态文件照这一刻再写（施工 O-18）。
    changed: Arc<Changed>,
    /// 挂接点（施工 O-43 上）：平台的通知、请求原样交给挂在 `chat.event` 的包。
    hooks: Arc<Hooks>,
    /// 驱动自己的状态目录（施工 Q-3，同 `toolset::Call::store`）。
    store: PathBuf,
    /// 驱动给场所会话记事实的模板（施工 Q-3，`ToolSet::facts`）、问模型的提示词模板（施工 Q-7，`ToolSet::prompts`）；没有工具套的
    /// 是空的。
    texts: Option<Texts>,
}

/// 驱动报的事实模板、提示词模板（[`Gate`]）。
type Texts = (Arc<Answers>, Arc<Answers>);

impl Gate {
    /// 拼一份：几样都是 `serve` 起来时备好的。
    pub(crate) fn new(
        (current, reported, tuning): (Arc<Current>, Arc<Reported>, Tuning),
        bots: Arc<Bots>,
        inbound: mpsc::Sender<Event>,
        tell: Arc<dyn Fn(Notice) + Send + Sync>,
        changed: Arc<Changed>,
        (hooks, store, texts): (Arc<Hooks>, PathBuf, Option<Texts>),
    ) -> Gate {
        Gate {
            current,
            reported,
            tuning,
            bots,
            inbound,
            tell,
            changed,
            hooks,
            store,
            texts,
        }
    }

    /// 桥自己的数。
    pub fn tuning(&self) -> &Tuning {
        &self.tuning
    }

    /// 连着的号。
    pub fn bots(&self) -> &Bots {
        &self.bots
    }

    /// 驱动的配置项（[`Driver::settings`]）最新的值，照名字：没交的、`null` 的已经照清单的默认值补上，默认值也没有的不放。拿着的
    /// 随时读到最新的，核心推来新的收到通知。
    pub fn config(&self) -> watch::Receiver<Map<String, Value>> {
        self.current.driver()
    }

    /// 驱动的配置项 `name` 清单里的默认值：推来的值读不懂时照它（施工 I-3）。没写默认值的是空的。
    pub fn default(&self, name: &str) -> Option<Value> {
        self.current.driver_default(name)
    }

    /// 报状态：后台页的 `status`、状态文件里驱动的那几格换成 `fields`，状态文件照这一刻再写。
    pub fn report(&self, fields: Map<String, Value>) {
        self.reported.set(fields);
        self.changed();
    }

    /// 认出来的一件事交给跟核心的那一头；那一头不收了（桥在停）交回假。
    pub async fn inbound(&self, event: Event) -> bool {
        self.inbound.send(event).await.is_ok()
    }

    /// 说给人听的一句。
    pub fn tell(&self, notice: Notice) {
        (self.tell)(notice);
    }

    /// 状态变了：状态文件照这一刻再写。
    pub fn changed(&self) {
        self.changed.notify();
    }

    /// 机器人号 `bot` 收到的一件平台事件 `event` 原样交给挂在 `chat.event` 的包，不等；`to` 是驱动认出的场所（施工 I-8 上：
    /// 只问在那个场所开着的插件），认不出的是空的。
    pub fn event(&self, bot: i64, to: Option<To>, event: &Value) {
        self.hooks.event(bot, to, event);
    }

    /// 驱动自己的状态目录（施工 Q-3）：im 的状态目录下照平台的名字。
    pub fn store(&self) -> &Path {
        &self.store
    }

    /// 驱动带的插件 `id` 在场所 `to` 开着没有（施工 Q-3：总开关、场所开关，同 `toolset::Call::plugins`）。
    pub fn active(&self, id: &str, to: To) -> bool {
        self.hooks.active(to).contains(&id)
    }

    /// 驱动带的插件给场所 `to` 记一句（施工 Q-3）：模板 `name`（`ToolSet::facts` 里的）字段照 `fields` 换、去掉首尾空白，种类是 `kind`，交给跟
    /// 核心的那一头套上外壳记进那个场所的主会话（机器人号 `bot` 收进的）。那一头不收了（桥在停）交回假。
    ///
    /// # Panics
    ///
    /// `name` 不是 `ToolSet::facts` 里的、少了它要的字段：是 bug。
    pub async fn note(
        &self,
        (bot, to): (i64, To),
        kind: &'static str,
        name: &str,
        fields: &[(&str, &str)],
    ) -> bool {
        let (facts, _) = self.texts.as_ref().expect("驱动报了事实的模板");
        let text = facts.text(name, fields).trim().to_string();
        self.inbound(Event::Noted {
            bot,
            to,
            kind,
            text,
        })
        .await
    }

    /// 驱动带的插件的提示词模板 `name`（`ToolSet::prompts` 里的）字段照 `fields` 换（施工 Q-7）。
    ///
    /// # Panics
    ///
    /// `name` 不是 `ToolSet::prompts` 里的、少了它要的字段：是 bug。
    pub fn prompt(&self, name: &str, fields: &[(&str, &str)]) -> String {
        let (_, prompts) = self.texts.as_ref().expect("驱动报了提示词模板");
        prompts.text(name, fields)
    }

    /// 请 im 替驱动带的插件问一次模型（施工 Q-7）：场所 `to`，`model.call` 的参数 `request`（`model` 由 im 照那个场所的判断模型
    /// 填）。交回模型回的字。
    ///
    /// # Errors
    ///
    /// 核心拒了的交回它的原因（`no_model` 这类），桥在停的是 `stopping`。
    pub async fn ask(&self, to: To, request: Value) -> Result<String, String> {
        let (reply, answered) = Replier::pair();
        if !self.inbound(Event::Asked { to, request, reply }).await {
            return Err("stopping".to_string());
        }
        answered
            .await
            .unwrap_or_else(|_| Err("stopping".to_string()))
    }
}
