//! 清理（`docs/blueprint/chat.md` 第五条「怎么走」第 2 条）：去掉模型多吐出来的下一轮、漏进正文的工具调用；整条是空的、整条是
//! 括号旁白的丢掉。
//! 两份名单（漏进来的标记、不可见字符）是数据，从 [`Outbound`] 拿，出厂的照旧版（施工时定的第 2、10 条，第八条）。
//!
//! 漏进来的样子是旧版线上取的证：模型复读退化时把 `<tool_call><function=…>` 当正文吐出来，中转站的解析器不认，原样落进
//! 正文发到群里；流式截断的只剩开头，没有收尾。
//!
//! 多吐出来的下一轮（施工 I-214）是重制版线上取的证（2026-11-11，deepseek-v4.1-flash）：她的话说完了模型没停，接着吐一个
//! 角色词 `user`，再照给模型看的样子编出下一条用户消息——风格提示、「只回这一条」的提示、一行 `[18:31] 某某 (id=…, owner)
//! [msg=…]: …`。编号、那句话都是编的。原样发出去，群里看到的是提示词和一句没人说过的话。

use super::{OutCtx, OutStep, OutWhy, Outbound, OutboundRule, Outgoing, Target};

/// 清理这条规则。
pub(super) struct Rule;

impl OutboundRule for Rule {
    fn judge(&self, mut outgoing: Outgoing, target: Target, ctx: &OutCtx) -> OutStep {
        // 先去掉多吐出来的下一轮（它后面的都不是她的话），再去掉漏进来的工具调用。
        let cut = next_turn(&outgoing.text, &ctx.outbound.turn_marks);
        if let Some(end) = cut {
            outgoing.text.truncate(end);
        }
        let leaked = strip_leaks(&mut outgoing.text, &ctx.outbound) || cut.is_some();
        let pictured = !outgoing.images.is_empty();
        if blank(&outgoing.text, &ctx.outbound.invisible) {
            match (pictured, leaked) {
                (false, true) => return OutStep::Drop(OutWhy::Leaked),
                (false, false) => return OutStep::Drop(OutWhy::Blank),
                // 有图的照发图，空白的正文不发成一个空气泡。
                (true, _) => outgoing.text.clear(),
            }
        }
        if !pictured && aside(outgoing.text.trim()) {
            return OutStep::Drop(OutWhy::Aside);
        }
        OutStep::Continue { outgoing, target }
    }
}

/// 模型多吐出来的下一轮前面常挂着的角色词（一行只有它，或者紧贴着下一轮的开头）。
const ROLES: [&str; 5] = ["user", "assistant", "system", "human", "tool"];

/// 正文 `text` 里模型多吐出来的「下一轮」从哪开始（施工 I-214）：她的话到这里为止（字节位置），后面的整段不是她说的。没有的
/// 是空的。
///
/// 两种开头，取最早的一处：
/// - `marks` 里的一项（给模型看的块的开头，她自己不会写）；
/// - 一行照场所消息给模型看的样子写的：`[时:分] ` 开头、后面有 `[msg=`（前面可以紧贴着一个角色词）。
///
/// 开头前面挂着的角色词（单独一行，或者紧贴着开头）一起去掉；再前面的空白也去掉。
#[must_use]
pub fn next_turn(text: &str, marks: &[String]) -> Option<usize> {
    let marked = marks
        .iter()
        .filter(|mark| !mark.is_empty())
        .filter_map(|mark| text.find(mark.as_str()))
        .min();
    let mut start = 0;
    let lined = text.split_inclusive('\n').find_map(|line| {
        let at = start;
        start += line.len();
        venue_line(line).map(|skipped| at + skipped)
    });
    let at = match (marked, lined) {
        (Some(marked), Some(lined)) => marked.min(lined),
        (found, None) | (None, found) => found?,
    };
    let said = text.get(..at)?.trim_end();
    // 最后一行只有一个角色词（`user`、`User:`）：是那一轮的头，不是她的话。
    let last = said.rfind('\n').map_or(0, |line| line + 1);
    let word = said.get(last..)?.trim().trim_end_matches([':', '：']);
    match ROLES.iter().any(|role| word.eq_ignore_ascii_case(role)) {
        true => Some(said.get(..last)?.trim_end().len()),
        false => Some(said.len()),
    }
}

/// 一行是不是照场所消息给模型看的样子写的：去掉行首的空白、紧贴着的一个角色词以后，`[时:分] ` 开头、后面有 `[msg=`。是的
/// 交回这一行里它从哪开始（跳过的空白的字节数；角色词留给调的一方照行尾去掉）。
fn venue_line(line: &str) -> Option<usize> {
    let skipped = line.len() - line.trim_start().len();
    let rest = line.trim_start();
    let glued = ROLES
        .iter()
        .find_map(|role| {
            let head = rest.get(..role.len())?;
            head.eq_ignore_ascii_case(role)
                .then(|| rest.get(role.len()..))?
        })
        .unwrap_or(rest);
    let stamp = glued.as_bytes();
    let timed = stamp.len() > 8
        && stamp[0] == b'['
        && stamp[1].is_ascii_digit()
        && stamp[2].is_ascii_digit()
        && stamp[3] == b':'
        && stamp[4].is_ascii_digit()
        && stamp[5].is_ascii_digit()
        && stamp[6] == b']'
        && stamp[7] == b' ';
    (timed && glued.contains("[msg=")).then_some(skipped)
}

/// 一段一段去掉漏进来的工具调用，直到没有：开头、收尾照位置一一对上（[`Outbound`] 的 `leak_open`、`leak_close`）；每次去
/// 最早开头的一段，从开头到它后面的第一个收尾，找不到收尾的去到末尾。去掉过东西的是 `true`。
///
/// 收尾从开头的后面找起：收尾可能是开头的一段（开头 ` ```tool `、收尾 ` ``` `），从开头处找只去掉半个开头（施工时定的
/// 第 11 条）。开头都不空（声明守着），每一轮至少去掉一个开头，停得下来。
fn strip_leaks(text: &mut String, outbound: &Outbound) -> bool {
    let mut stripped = false;
    while let Some((start, open, close)) = outbound
        .leak_open
        .iter()
        .zip(&outbound.leak_close)
        .filter_map(|(open, close)| text.find(open.as_str()).map(|at| (at, open, close)))
        .min_by_key(|(at, ..)| *at)
    {
        let after = start + open.len();
        let end = text
            .get(after..)
            .and_then(|rest| rest.find(close.as_str()))
            .map_or(text.len(), |at| after + at + close.len());
        text.replace_range(start..end, "");
        stripped = true;
    }
    stripped
}

/// 看起来是空的：只有空白和 `invisible` 里的字。模型「什么都不想说」时常吐一个零宽空格，`trim` 不认它，发出去是一个空
/// 气泡。只拿来判空，不拿来改正文：零宽连接符夹在表情里是有意义的。
fn blank(text: &str, invisible: &[char]) -> bool {
    text.chars()
        .all(|c| c.is_whitespace() || invisible.contains(&c))
}

/// 整条是一对中文括号括起来的旁白：以 `（` 开头、和它配对的 `）` 在末尾，里面可以再套括号，括号外没有别的字。`text`
/// 交进来前已经去掉首尾的空白。
fn aside(text: &str) -> bool {
    let mut depth = 0_u32;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '（' => depth += 1,
            '）' if depth == 0 => return false,
            '）' => {
                depth -= 1;
                // 最外面那一对关上了，后面还有字：括号外有字。
                if depth == 0 && chars.peek().is_some() {
                    return false;
                }
            }
            _ if depth == 0 => return false,
            _ => {}
        }
    }
    // 空的不算；没关上的不算。
    !text.is_empty() && depth == 0
}
