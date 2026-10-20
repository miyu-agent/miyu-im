//! 接入通讯平台的宿主 `miyu-im`（`docs/blueprint/onebot.md`，施工 O-8；施工 I-2 下和驱动 `miyu-im-onebot` 拆开）：接 QQ 这类平台的
//! 一个头，和终端界面、网页平级
//! （核心的 `docs/designs/18-通讯平台.md` 第三节）。O-8 是骨架：终端管理员的私聊、只有文字；O-22 起群消息记进场所会话（旁听，不开回合）。
//!
//! - [`settings`]：核心在握手时交来、变了推过来的配置：NapCat 的端口、令牌（施工 O-20）；没交的端口照清单的默认值。
//! - [`package`]：桥自己的包文件夹（施工 S-6 上）：核心交来的 `MIYU_PACKAGE_DIR`，没有的照资源目录里出厂的那一份；桥自己的
//!   文件在它的 `data/` 里。
//! - [`tuning`]：桥自己的数（包文件夹的 `data/bridge.json`）：路径、等回应多久、队列多长。
//! - [`serve`]：在核心亲手给的管道（标准输入输出，施工 O-18）上握手、开监听，把下面几样接起来；核心关了管道就停。
//! - [`driver`]：宿主和驱动之间（施工 I-2 下）：平台的接法（[`driver::Driver`]）、驱动从宿主拿的（[`driver::Gate`]）。
//! - [`chat`]：和平台无关的数据（施工 I-2 上）：一条消息、撤回、事件、段、带的东西、发给谁，驱动认出来交给宿主的样子。
//! - `core`：跟核心的那一头：握手（不带凭据）、`venue.session`、带 `as`、`venue` 的 `session.send`、撤回、订阅、她的回话。
//! - [`status_file`]：桥的状态文件 `state/packages/im/status.json`，`status` 读它（施工 O-18）。
//! - [`control`]：`miyu im start`、`stop`、`restart`、`status`：调核心的 `extension.*`（施工 O-18）。
//! - [`logs`]：`miyu im logs [-f]`：印运行日志和标准错误（施工 O-18）。
//! - [`texts`]：说给人听的字（包文件夹的 `data/human/`）。
//! - `current`：桥手里最新的配置：握手交来的，核心推来新的就换上，改了不用重启（施工 O-20）。
//! - `running`：桥跑着的样子：实际听的端口、连着的号、最新的配置；后台页的 `status`、状态文件照它说，推来的端口变化照它换
//!   （施工 O-28 下从桥自己的 WebUI 挪出来；桥自己的网页随 O-28 下去掉）。
//! - [`rules`]：场所规则和出厂数据（施工 O-21）：出厂的起来时读一次，系统的照群聊内核读好、合起来、套到场所上，变了下一次
//!   用就照新的。
//! - `media`：取图、视频、文件到手，视频、文件存进会话工作区的 `qq-files/`（施工 O-33）。
//! - [`venue`]：`miyu im venue show <场所>`：一个场所每一项的值和来处（施工 O-21）。
//! - `site`：im 自己的页面服务（施工 I-5 上）：只听本机，给页面的文件、交接票换登录、页面和宿主说话的通道。
//! - [`web`]：`miyu im web`：带交接票打开 im 自己的页面（施工 I-6 上；O-28 补起原来是转给网页软件的后台页）。
//! - `counters`：今天的几个数（施工 I-6 下）：收到、回复、只记录、没发出去，记在 im 自己的状态目录里。
//! - `changed`：桥手里的状态变了的计数（施工 I-6 上）：状态文件、页面的通道照它。
//! - `recent`：最近判断（施工 I-7 上补）：每个会话留最近几笔回没回、为什么，会话浮窗的「概况」照它。
//!
//! 分层照 `01-架构.md` 第九节第 5 层：只用本机传输连核心（`start` 这几样照终端的样子连，用 `miyu-client` 的 `open::Core`）；场所、平台上的人的编号经第 2
//! 层的群聊内核 `miyu-chat` 拼；不依赖核心的 crate（`miyu-core`、`miyu-endpoint`，`18-通讯平台.md` 第一节）：配置由核心交
//! （施工 O-20）。核心里别的 crate 只经门面 `miyu-client` 拿（设计 32 第九节第 1 条，施工 S-6 上）。

pub mod access;
mod changed;
pub mod chat;
pub mod control;
mod core;
mod counters;
mod current;
pub mod driver;
pub mod logs;
mod media;
pub mod package;
pub mod plugins;
mod recent;
pub mod rules;
mod running;
pub mod serve;
pub mod settings;
mod site;
pub mod status_file;
pub mod texts;
pub mod toolset;
pub mod tuning;
pub mod venue;
pub mod web;
pub mod whitelist;

/// 运行日志的目标。
pub const TARGET: &str = "miyu::im";

/// 还没有字可用时印原话的开头，也是程序的名字。
pub const PROGRAM: &str = "miyu-im";

/// 这个软件包的编号（`im/package.toml`，施工 O-18）：`extension.*` 照它找桥，状态目录、标准错误的文件照它起名。
pub const PACKAGE: &str = "im";
