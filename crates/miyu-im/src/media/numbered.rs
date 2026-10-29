//! 合并转发里带的东西的编号（施工 I-75，`onebot.md` 第一条「群消息」第 2 条、「平台工具（二）」第 4 条）：展开时交给核心的格带上
//! `index`，近况里写成 `[image #<编号>: …]`；她照这条消息的 `msg` 和这个编号 `fetch_media`，按需取时照同一个数法找回那一样。
//!
//! - 接着这条消息自己带的数：自己的是 1 到 n（`venue.media`），转发里的从 n + 1 起。
//! - 照展开的先后数：一个转发里一条条往下，每条先数它带的（编号合写法的，同 [`crate::toolset::kept`]），再数套在它里面的转发。
//!   名字没有、超了限额没放进去的那几条也数：编号不跟着限额变。
//! - 打不开的转发（没内联又取不到的，展开时照限额没去取的）到这里为止，它和后面的都不编。展开时和按需取时读到的一样，编出的号就一样；
//!   按需取时多打开了的那几个，编号只会排在给她看过的后面。

use futures_util::future::BoxFuture;

use crate::chat::{Forwarded, Link, Media, Node};
use crate::toolset::kept;

/// 编到哪了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Numbering {
    /// 已经编了几样（连这条消息自己带的）。
    done: usize,
    /// 碰到打不开的转发了：后面的都不编。
    stopped: bool,
}

impl Numbering {
    /// 自己带了 `own` 样的那条消息：转发里的从 `own + 1` 数。
    pub fn after(own: usize) -> Numbering {
        Numbering {
            done: own,
            stopped: false,
        }
    }

    /// 下一样的编号；停了的是空的。
    pub fn take(&mut self) -> Option<usize> {
        if self.stopped {
            return None;
        }
        self.done += 1;
        Some(self.done)
    }

    /// 碰到打不开的转发：后面的都不编。
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// 停了没有。
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// 编了几样（连自己带的），停了的照停的那一刻。
    pub fn done(&self) -> usize {
        self.done
    }

    /// 只数不展开的几条 `nodes`：每条带的，再往里数套着的转发（[`Numbering::skip`]）。
    pub fn count(&mut self, nodes: &[Node]) {
        for node in nodes {
            for _ in kept(&node.segments.media) {
                if self.take().is_none() {
                    return;
                }
            }
            self.skip(&node.segments.forwards);
        }
    }

    /// 不去取的几个转发：内联了的照它数，没内联的停。
    pub fn skip(&mut self, forwards: &[Forwarded]) {
        for forwarded in forwards {
            if self.stopped {
                return;
            }
            match &forwarded.content {
                Some(nodes) => self.count(nodes),
                None => self.stop(),
            }
        }
    }
}

/// 照编号 `index` 找这条消息的合并转发 `forwards` 里的那一样（`numbering` 数过了这条消息自己带的）：没内联的转发经 `link` 调
/// `get_forward_msg` 取，取不到的停。找不到的（编到头了、停了）交回空的，`numbering` 停在数到的地方。
pub fn find<'a>(
    link: &'a dyn Link,
    forwards: &'a [Forwarded],
    index: usize,
    numbering: &'a mut Numbering,
) -> BoxFuture<'a, Option<Media>> {
    Box::pin(async move {
        for forwarded in forwards {
            let fetched;
            let nodes = match (&forwarded.content, &forwarded.id) {
                (Some(nodes), _) => nodes,
                (None, Some(id)) => match link.forwarded(id).await {
                    Ok(Some(nodes)) => {
                        fetched = nodes;
                        &fetched
                    }
                    Ok(None) | Err(_) => {
                        numbering.stop();
                        return None;
                    }
                },
                (None, None) => {
                    numbering.stop();
                    return None;
                }
            };
            for node in nodes {
                for item in kept(&node.segments.media) {
                    match numbering.take() {
                        Some(number) if number == index => return Some(item),
                        Some(_) => {}
                        None => return None,
                    }
                }
                if let Some(found) = find(link, &node.segments.forwards, index, numbering).await {
                    return Some(found);
                }
                if numbering.stopped() {
                    return None;
                }
            }
        }
        None
    })
}

#[cfg(test)]
mod tests;
