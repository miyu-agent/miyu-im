//! 给她看的事实的模板（施工 O-25 下，`onebot.md` 第一条「退信」第 3 条、「场所规则和出厂数据」第 1、2 条；O-36 多「另一条线正在
//! 回」，「群里怎么叫她」第 7 条；O-46 多「文件没发出去」，「展示」第 4 条；O-49 多群会话 system 尾部的群名、群号，「群名」；O-50 多禁言、解禁，「禁言」；O-53 多冒充提醒，「冒充提醒」；O-56 多她先找人，「主动发消息」）：包文件夹的 `data/facts/` 里的两份（给模型看的字，登记在 `26-提示词.md` 第十节），桥起来
//! 时和别的出厂数据一起读，交内核的 [`Template::parse`] 查。没有系统那一份：给模型看的字随包走。
//!
//! 字段只认各自的那几个：多要了别的，填的时候换不出来，起来时就当打包的错（「施工时定的」第 129 条）。少要几个照收。

use std::path::Path;

use miyu_chat::{Problem, Source};
use miyu_client::protocol::Template;
use miyu_client::settings::problem::Code;

use super::{files, required};

/// 退信的模板在出厂目录里的位置；问题里的文件名也照它写。
const UNDELIVERED: &str = "facts/undelivered.txt";

/// 退信的模板认的字段：为什么、NapCat 说的原因、她那一段的开头（「退信」第 3 条）。
const UNDELIVERED_FIELDS: &[&str] = &["why", "detail", "text"];

/// 「另一条线正在回」的模板在出厂目录里的位置（施工 O-36）。
const ANOTHER_LINE: &str = "facts/another-line.txt";

/// 「另一条线正在回」认的字段：别的线在回的那一条、这条线回的那一条的平台编号（「群里怎么叫她」第 7 条「事实」）。
const ANOTHER_LINE_FIELDS: &[&str] = &["msg", "aim"];

/// 「文件没发出去」的模板在出厂目录里的位置（施工 O-46）。
const UNSHOWN: &str = "facts/unshown.txt";

/// 「文件没发出去」认的字段：为什么、NapCat 说的原因、文件名（「展示」第 4 条）。
const UNSHOWN_FIELDS: &[&str] = &["why", "detail", "name"];

/// 她被禁言了的事实（施工 O-50，「禁言」第 1 条）在出厂目录里的位置。
const MUTED: &str = "facts/muted.txt";

/// 禁言的事实认的字段：谁禁的、禁多久。
const MUTED_FIELDS: &[&str] = &["by", "duration"];

/// 她被解禁了的事实（施工 O-50，「禁言」第 1 条）在出厂目录里的位置。
const UNMUTED: &str = "facts/unmuted.txt";

/// 解禁的事实认的字段：怎么解的（`lifted`、`expired`）、谁解的。
const UNMUTED_FIELDS: &[&str] = &["why", "by"];

/// 她先找人的事实（施工 O-56，「主动发消息」）在出厂目录里的位置。
const REACHED: &str = "facts/reached.txt";

/// 她先找人的事实认的字段：她那句话的开头。
const REACHED_FIELDS: &[&str] = &["text"];

/// 冒充提醒的事实（施工 O-53，「冒充提醒」）在出厂目录里的位置。
const IMPOSTOR: &str = "facts/impostor.txt";

/// 冒充提醒认的字段：哪一条（平台编号）、借的是谁的名字。
const IMPOSTOR_FIELDS: &[&str] = &["msg", "name"];

/// 群名、群号那一段（施工 O-49，放在群会话的 system 尾部）在出厂目录里的位置。
const GROUP: &str = "system/group.txt";

/// 群名、群号那一段认的字段（「群名」第 2 条）。
const GROUP_FIELDS: &[&str] = &["name", "id"];

/// 查过的几份模板。
#[derive(Debug)]
pub(super) struct Facts {
    /// 退信（施工 O-25 下）。
    pub(super) undelivered: Template,
    /// 另一条线正在回（施工 O-36）。
    pub(super) another_line: Template,
    /// 她给人看的文件没发出去（施工 O-46）。
    pub(super) unshown: Template,
    /// 群名、群号（施工 O-49）。
    pub(super) group: Template,
    /// 她被禁言了（施工 O-50）。
    pub(super) muted: Template,
    /// 她被解禁了（施工 O-50）。
    pub(super) unmuted: Template,
    /// 有人借了名字（施工 O-53）。
    pub(super) impostor: Template,
    /// 她先找了这个场所的人（施工 O-56）。
    pub(super) reached: Template,
}

/// 读出厂目录 `dir`（包文件夹的 `data/`）里的几份模板，查过交回。问题照退信、另一条线的先后记；有问题的交回空的。
pub(super) fn load(dir: &Path, problems: &mut Vec<Problem>) -> Option<Facts> {
    let undelivered = template(dir, UNDELIVERED, UNDELIVERED_FIELDS, problems);
    let another_line = template(dir, ANOTHER_LINE, ANOTHER_LINE_FIELDS, problems);
    let unshown = template(dir, UNSHOWN, UNSHOWN_FIELDS, problems);
    let group = template(dir, GROUP, GROUP_FIELDS, problems);
    let muted = template(dir, MUTED, MUTED_FIELDS, problems);
    let unmuted = template(dir, UNMUTED, UNMUTED_FIELDS, problems);
    let impostor = template(dir, IMPOSTOR, IMPOSTOR_FIELDS, problems);
    let reached = template(dir, REACHED, REACHED_FIELDS, problems);
    Some(Facts {
        undelivered: undelivered?,
        another_line: another_line?,
        unshown: unshown?,
        group: group?,
        muted: muted?,
        unmuted: unmuted?,
        impostor: impostor?,
        reached: reached?,
    })
}

/// 读出厂目录 `dir` 里的模板 `name`，查过交回。不在、读不成的记一条问题；写坏了、要了 `fields` 以外的字段的记一条 `bad_format`，
/// 原话说错在哪。有问题的交回空的。
fn template(
    dir: &Path,
    name: &str,
    fields: &[&str],
    problems: &mut Vec<Problem>,
) -> Option<Template> {
    let text = required(&dir.join(name), name, problems)?;
    let checked = Template::parse(&text)
        .map_err(|error| error.to_string())
        .and_then(|template| {
            match template
                .fields()
                .iter()
                .find(|field| !fields.contains(field))
            {
                Some(field) => Err(format!("unknown field {{{field}}}: only {fields:?}")),
                None => Ok(template),
            }
        });
    match checked {
        Ok(template) => Some(template),
        Err(why) => {
            problems.push(files::whole(
                Code::BadFormat,
                Source::Factory,
                name,
                Some(why),
            ));
            None
        }
    }
}
