//! 按需看（施工 O-33，`onebot.md` 第一条「平台工具（二）」第 4 到 7 条；2026-10-11 项目主人定；施工 T-3 从宿主挪进驱动）：她调
//! `fetch_media {msg, index?}`，才去 QQ 取那一条里的第几样。im 照会话填好场景交过来（这一轮这件工具调过几次也数好了）；这里挡一轮
//! 取满了的、认参数，要取的交给 im 另起任务，跟核心的那一头不等 QQ。
//!
//! 私聊也给（施工 O-35）：私聊里带了东西的那条，她也看得到 `[msg=…]` 和 `#n`。
//!
//! 任务里：`get_msg` 现认那一条的段（桥重启了也认得；同一套 `segments`，第几个和交给核心的 `venue.media` 一个数法），不在这个
//! 会话里的当没有（[`owned`]：人打得出假的 `[msg=…]`，编号不能拿去看别处的消息）；图存成 blob，作为工具结果的图片块交回（核心照调用它的会话属主拷 blob，核心 O-2 三补）；存不成图的（太大、
//! 不是图）和视频、文件一样写进这个会话工作区的 `qq-files/`，回路径。语音、小黄脸、`mface` 商城表情取不了。一轮最多取
//! `fetch_per_turn` 次（`bridge.json`，出厂 4，照旧 Miyu），取没取到都算一次。不设大小上限。

use std::time::Duration;

use serde_json::Value;

use miyu_im::chat::{DETAIL, Fetch, Fetched, Link, Source, To};
use miyu_im::toolset::{
    Call, Missed, Planned, QQ_FILES, Stored, UNLIMITED, file_name, kept, message, msg_id, over,
    save, take, too_large,
};

/// 她要的那一样：消息的平台编号、第几样（从 1 数；写错的是 0，取的时候照「没有这一样」答）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wanted {
    /// 平台编号：QQ 的消息编号是整数（可以是负的），写成字。
    pub(crate) msg: String,
    /// 第几样。
    pub(crate) index: usize,
}

/// 读参数（纯逻辑）：`msg` 是整数，或者写成十进制整数的字（最多 20 位，前面可以有负号），别的交回 `not-found`；`index` 不写是 1，
/// 是正整数（字也认）的照它，别的是 0。
pub(crate) fn wanted(args: &Value) -> Result<Wanted, &'static str> {
    let msg = msg_id(&args["msg"]).ok_or("not-found")?;
    let index = match &args["index"] {
        Value::Null => 1,
        Value::Number(number) => number
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(0),
        Value::String(text) => text.parse().unwrap_or(0),
        _ => 0,
    };
    Ok(Wanted { msg, index })
}

/// `fetch_media` 的一次调用（模块的说明）：这一轮已经取了 `limit` 次的答 `too-many`（取没取到都算一次），参数不对的答 `not-found`，
/// 别的交给 im 另起任务去取，最多等 `wait`。
pub(super) fn fetch_media(
    call: Call,
    limit: usize,
    (wait, image_limit): (Duration, usize),
) -> Planned {
    if call.used >= limit {
        let limit = limit.to_string();
        return Planned::Now(call.answers.say("too-many", &[("count", &limit)], true));
    }
    let wanted = match wanted(&call.args) {
        Ok(wanted) => wanted,
        Err(why) => return Planned::Now(call.answers.say(why, &[], true)),
    };
    Planned::later(async move {
        let home = std::env::home_dir();
        match fetch(&call, &wanted, (wait, image_limit), home).await {
            Some(Ok(answer)) => answer,
            Some(Err((name, fields))) => {
                let fields: Vec<(&str, &str)> = fields
                    .iter()
                    .map(|(key, value)| (*key, value.as_str()))
                    .collect();
                call.answers.say(name, &fields, true)
            }
            // 核心断开了：答了也没人收，照「够不着」答。
            None => call.answers.say("unreachable", &[], true),
        }
    })
}

/// 取回来的那一条在不在会话 `to` 里（纯逻辑，施工 O-35 核对归属；群里的 O-33 起就查）：群里是这个群的，私聊里是这个人在
/// 私聊里发的。编号是她给的，人打得出假的 `[msg=…]`：这个人在群里说的、别人的私聊、她自己发的都当没有。
pub(crate) fn owned(fetched: &Fetched, to: To) -> bool {
    match to {
        To::Group(group) => fetched.group == Some(group),
        To::Private(user) => fetched.private && fetched.user == Some(user),
    }
}

/// 没做成的：答的那一句的名字和字段。
type Undone = (&'static str, Vec<(&'static str, String)>);

/// 取那一样：成了的交回答核心的 `result`；没成的交回那一句；核心断开了的是空的。`home` 是系统的家目录（换工作目录的 `~`）。
/// 图最多 `image_limit` 字节（施工 I-23）：段里写了大小、超了的不下，答 `unfetched`；视频、文件不设上限。
async fn fetch(
    call: &Call,
    wanted: &Wanted,
    (wait, image_limit): (Duration, usize),
    home: Option<std::path::PathBuf>,
) -> Option<Result<Value, Undone>> {
    let Some(link) = &call.link else {
        return Some(Err(("unreachable", Vec::new())));
    };
    let msg = &wanted.msg;
    let fetched = match message(link.as_ref(), msg).await {
        Ok(fetched) => fetched,
        Err(Missed::Failed(_)) => return Some(Err(("not-found", Vec::new()))),
        Err(missed) => return Some(Err(undone(missed))),
    };
    if !owned(&fetched, call.to) {
        return Some(Err(("not-found", Vec::new())));
    }
    let media = kept(&fetched.segments.media);
    let index = wanted.index;
    let Some(item) = index.checked_sub(1).and_then(|at| media.get(at)) else {
        return Some(Err(("no-item", vec![("count", media.len().to_string())])));
    };
    let Some(how) = item.fetch else {
        return Some(Err(("not-fetchable", Vec::new())));
    };
    let limit = match how {
        Fetch::Image => image_limit,
        _ => UNLIMITED,
    };
    if too_large(item, limit) {
        return Some(Err(undone(Missed::Unfetched(over(limit)))));
    }
    let taken = match (how, call.to) {
        // 群里的文件（施工 I-16）：`get_file` 只认缓存的图片、语音，照群文件的下载地址取；取不到的再照 `get_file` 试。
        (Fetch::File, To::Group(group)) => {
            match group_file(link.as_ref(), group, &item.id, wait).await {
                Some(bytes) => Ok((bytes, None)),
                None => take(link.as_ref(), how, &item.id, (wait, limit)).await,
            }
        }
        _ => take(link.as_ref(), how, &item.id, (wait, limit)).await,
    };
    let (bytes, named) = match taken {
        Ok(taken) => taken,
        Err(missed) => return Some(Err(undone(missed))),
    };
    if how == Fetch::Image {
        match call
            .blobs
            .image(&format!("msg-{msg}-{index}"), &bytes)
            .await
        {
            Stored::Image(block) => {
                return Some(Ok(serde_json::json!({"blocks": [block], "error": false})));
            }
            // 存不成图的（太大、不是图）照文件存。
            Stored::NotImage => {}
            Stored::Gone => return None,
        }
    }
    let name = item
        .name
        .as_deref()
        .or(named.as_deref())
        .or(Some(item.id.as_str()));
    let name = file_name(msg, index, name);
    let cwd = call.cwd.clone();
    let saved = tokio::task::spawn_blocking(move || save(&cwd, home.as_deref(), &name, &bytes))
        .await
        .map_err(|error| error.to_string())
        .and_then(|saved| saved);
    Some(match saved {
        // 答相对工作区的路径、用 `/` 隔开：她 `read` 照工作区解析相对路径；落到的真实路径在 Windows 上带 `\\?\` 前缀、
        // 反斜杠进模板还要转义，给她看的字三个平台就不一样了（CI 的 Windows 撞出来的，「施工时定的」第 204 条）。
        Ok(path) => {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            let shown = format!("{QQ_FILES}/{}", name.unwrap_or_default());
            Ok(call.answers.say("saved", &[("path", &shown)], false))
        }
        Err(detail) => Err(("not-saved", vec![("detail", cut(&detail))])),
    })
}

/// 群 `group` 里编号是 `id` 的群文件的字节（施工 I-16）；拿不到下载地址、下不下来的是空的。
async fn group_file(link: &dyn Link, group: i64, id: &str, wait: Duration) -> Option<Vec<u8>> {
    let url = link.group_file_url(group, id).await.ok()?;
    miyu_im::toolset::bytes(&[Source::Url(url)], wait, UNLIMITED)
        .await
        .ok()
}

/// 没取到的那一句：NapCat 回失败的 `failed`（它说的），拿不到手的 `unfetched`（为什么，截到 [`DETAIL`] 个字符），等不到、没连着的
/// 同平台工具（一）。
fn undone(missed: Missed) -> Undone {
    let name = missed.name();
    match missed {
        Missed::Failed(detail) | Missed::Unfetched(detail) => {
            (name, vec![("detail", cut(&detail))])
        }
        Missed::Unanswered | Missed::Unreachable => (name, Vec::new()),
    }
}

/// 交给她的原话最多 [`DETAIL`] 个字符（同平台工具（一）的 `failed`）。
fn cut(detail: &str) -> String {
    detail.chars().take(DETAIL).collect()
}

#[cfg(test)]
mod tests;
