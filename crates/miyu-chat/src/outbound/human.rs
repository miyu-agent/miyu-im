//! 照真人的样子一句分几条发（`docs/blueprint/chat.md` 第五条「怎么走」第 10 条，施工 I-26 起；施工 I-49 照项目主人 10-27 定的
//! 重做）：
//!
//! - 超过 [`LONGEST`] 个字的、带换行的不拆（讲解、列步骤，一口气发）。
//! - 句号、问号、叹号、分号、省略号处都断（真人一条里带句号的只有 1%）。
//! - 逗号照句子定：一句（两个句末标点之间）[`LONG_SENTENCE`] 个字以上的有 [`LONG_COMMA`]‰ 在逗号处断开，短的 [`SHORT_COMMA`]‰；
//!   断也只断一处，挑最靠中间、两边都够 [`MIN_CHARS`] 个字的那个逗号。所以多数条里还留着逗号。引号、括号、代码、网址里面不断。
//! - 不够 [`MIN_CHARS`] 个字的一条并进前一条（第一条并进后一条）；多于 [`MAX_PIECES`] 条的后面几条并成一条。
//! - 断开处的句号、分号、逗号、顿号去掉，问号、叹号留着。
//! - 每条前面停一会儿 = 想的工夫 + 这一条照打字速度打完要多久（[`typed`]）：第二条前面想得久一点（[`THINK_FIRST`]），第三条起
//!   短（[`THINK_NEXT`]）；上下抖 0.8 到 1.2 倍，夹在 [`SHORTEST`] 到 [`LONGEST_PAUSE`] 之间。打字速度是场所参数
//!   `outbound.typing_speed`（字/分钟）。第一条当场发（她想、打字的工夫就是模型生成那一会儿），它算好的工夫交给出站队列：
//!   这个会话前面还有没发完的，它也照这个工夫等。
//!
//! 随机照这句话的内容取种子：同一句拆出来、停多久都一样，桥重启补发不会拆出两个样子。

use std::time::Duration;

use sha2::{Digest, Sha256};

/// 拆出来的一条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bubble {
    /// 正文。
    pub text: String,
    /// 发它以前停多久：第一条是零。
    pub pause: Duration,
    /// 想加打这一条要多久（第一条也算）：第一条在会话前面还有没发完的时候照它等。
    pub typed: Duration,
}

/// 超过这么多个字的不拆。
pub const LONGEST: usize = 200;

/// 一句最多拆成几条。
pub const MAX_PIECES: usize = 5;

/// 一条至少这么多个字（不算末尾去掉的标点）。
pub const MIN_CHARS: usize = 3;

/// 一句有这么多个字算长句。
pub const LONG_SENTENCE: usize = 12;

/// 长句在逗号处断开的千分比。
pub const LONG_COMMA: u32 = 300;

/// 短句在逗号处断开的千分比。
pub const SHORT_COMMA: u32 = 100;

/// 第二条前面想多久，毫秒。
pub const THINK_FIRST: u64 = 1000;

/// 第三条起前面想多久，毫秒。
pub const THINK_NEXT: u64 = 400;

/// 停顿至少多久，毫秒。
pub const SHORTEST: u64 = 600;

/// 停顿至多多久，毫秒。
pub const LONGEST_PAUSE: u64 = 8000;

/// 句子到这儿完了的标点：拆就在这里断。
const STRONG: [char; 6] = ['。', '！', '？', '；', '…', '～'];

/// 半角的问号、叹号、分号：后面紧跟字母、数字、`/`、`.`、`=` 的不算（代码、路径、网址、`!=`）；半角逗号同样。
const ASCII_STRONG: [char; 3] = ['?', '!', ';'];

/// 可以断也可以不断的标点。
const WEAK: [char; 3] = ['，', '、', ','];

/// 断开以后从末尾去掉的标点。
const DROPPED: [char; 6] = ['。', '；', '，', '、', ';', ','];

/// 开的引号、括号：里面不断。
const OPEN: [char; 8] = ['「', '“', '‘', '（', '(', '《', '【', '['];

/// 收的引号、括号。
const CLOSE: [char; 8] = ['」', '”', '’', '）', ')', '》', '】', ']'];

/// 照真人的样子把一句 `text` 拆成几条，打字速度是每分钟 `speed` 个字（见模块开头）：去掉首尾空白，空的交回空的；不拆的交回
/// 原样一条（也带 `typed`）。
#[must_use]
pub fn bubbles(text: &str, speed: u32) -> Vec<Bubble> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let mut rng = Rng::of(text);
    let chars: Vec<char> = text.chars().collect();
    let pieces = if chars.len() > LONGEST || text.contains('\n') {
        vec![text.to_string()]
    } else {
        cut(&chars, &mut rng)
    };
    let last = pieces.len() - 1;
    pieces
        .into_iter()
        .enumerate()
        .map(|(index, piece)| {
            let text = match index == last {
                true => piece.trim().to_string(),
                false => trimmed(&piece).to_string(),
            };
            let typed = typed(index, text.chars().count(), speed, &mut rng);
            let pause = match index {
                0 => Duration::ZERO,
                _ => typed,
            };
            Bubble { text, pause, typed }
        })
        .collect()
}

/// 第 `index` 条（从 0 数）有 `chars` 个字，每分钟打 `speed` 个字：想的工夫（第一、二条 [`THINK_FIRST`]，往后
/// [`THINK_NEXT`]）加打字的工夫，上下抖 0.8 到 1.2 倍，夹在 [`SHORTEST`] 到 [`LONGEST_PAUSE`] 之间。
fn typed(index: usize, chars: usize, speed: u32, rng: &mut Rng) -> Duration {
    let think = if index <= 1 { THINK_FIRST } else { THINK_NEXT };
    let chars = u64::try_from(chars).unwrap_or(u64::MAX);
    let typing = chars.saturating_mul(60_000) / u64::from(speed.max(1));
    let factor = 800 + u64::from(rng.below(401));
    let millis = think.saturating_add(typing).saturating_mul(factor) / 1000;
    Duration::from_millis(millis.clamp(SHORTEST, LONGEST_PAUSE))
}

/// 挑断的地方（见模块开头）：句子完了的都断，逗号照句子定断不断、断哪一个；再把太短的并掉、太多的收住。断不出两条的交回一条。
fn cut(chars: &[char], rng: &mut Rng) -> Vec<String> {
    let cuts = cuts(chars);
    let mut points: Vec<usize> = cuts
        .iter()
        .filter(|(_, strong)| *strong)
        .map(|(after, _)| *after)
        .collect();
    let mut sentences = vec![0];
    sentences.extend(points.iter().copied());
    sentences.push(chars.len());
    for pair in sentences.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let rate = if end - start >= LONG_SENTENCE {
            LONG_COMMA
        } else {
            SHORT_COMMA
        };
        if rng.below(1000) >= rate {
            continue;
        }
        let middle = (start + end) / 2;
        let comma = cuts
            .iter()
            .filter(|(after, strong)| !*strong && *after > start && *after < end)
            .map(|(after, _)| *after)
            .filter(|&after| long_enough(chars, start, after) && long_enough(chars, after, end))
            .min_by_key(|&after| after.abs_diff(middle));
        if let Some(comma) = comma {
            points.push(comma);
        }
    }
    points.sort_unstable();
    let mut pieces: Vec<String> = Vec::new();
    let mut start = 0;
    for end in points.into_iter().chain([chars.len()]) {
        pieces.push(chars[start..end].iter().collect());
        start = end;
    }
    merged(pieces)
}

/// `chars` 里 `start` 到 `end` 那一段去掉断开处的标点以后够不够 [`MIN_CHARS`] 个字。
fn long_enough(chars: &[char], start: usize, end: usize) -> bool {
    let piece: String = chars[start..end].iter().collect();
    trimmed(&piece).chars().count() >= MIN_CHARS
}

/// 不够 [`MIN_CHARS`] 个字的并进前一条（第一条并进后一条），多于 [`MAX_PIECES`] 条的后面几条并成一条。
fn merged(pieces: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in pieces {
        let short = trimmed(&piece).chars().count() < MIN_CHARS;
        match out.last_mut() {
            Some(last) if short || trimmed(last).chars().count() < MIN_CHARS => {
                last.push_str(&piece)
            }
            _ => out.push(piece),
        }
    }
    if out.len() > MAX_PIECES {
        let tail: String = out.drain(MAX_PIECES - 1..).collect();
        out.push(tail);
    }
    out
}

/// 能断的地方：断在第几个字后面、是不是句子完了的标点。引号、括号、代码、网址里面的不算；最后一个字后面不算。
fn cuts(chars: &[char]) -> Vec<(usize, bool)> {
    let urls = urls(chars);
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut code = false;
    for (index, &c) in chars.iter().enumerate() {
        if c == '`' {
            code = !code;
        }
        if OPEN.contains(&c) {
            depth += 1;
        } else if CLOSE.contains(&c) {
            depth = depth.saturating_sub(1);
        }
        let Some(&next) = chars.get(index + 1) else {
            break;
        };
        if depth > 0 || code || urls.iter().any(|url| url.contains(&index)) {
            continue;
        }
        let glued = next.is_ascii_alphanumeric() || "/.=".contains(next);
        let strong = STRONG.contains(&c) || (ASCII_STRONG.contains(&c) && !glued);
        let weak = (WEAK.contains(&c) && !(c == ',' && glued)) && !strong;
        let run = STRONG.contains(&next) || ASCII_STRONG.contains(&next);
        if strong && !run {
            out.push((index + 1, true));
        } else if weak {
            out.push((index + 1, false));
        }
    }
    out
}

/// 网址占的字（`xxx://` 起，到空白或不是 ASCII 的字为止）。
fn urls(chars: &[char]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut index = 0;
    while index + 2 < chars.len() {
        if chars[index..index + 3] != [':', '/', '/'] {
            index += 1;
            continue;
        }
        let start = (0..index)
            .rev()
            .take_while(|&at| chars[at].is_ascii_alphanumeric() || "+.-".contains(chars[at]))
            .last()
            .unwrap_or(index);
        let end = (index..chars.len())
            .find(|&at| !chars[at].is_ascii() || chars[at].is_whitespace())
            .unwrap_or(chars.len());
        out.push(start..end);
        index = end;
    }
    out
}

/// 去掉首尾空白和末尾一个断开处不留的标点（[`DROPPED`]）。
fn trimmed(piece: &str) -> &str {
    let piece = piece.trim();
    piece
        .strip_suffix(|c: char| DROPPED.contains(&c))
        .unwrap_or(piece)
        .trim_end()
}

/// 照这句话取种子的随机数（xorshift64）：同一句每次一样。
struct Rng(u64);

impl Rng {
    /// 种子是这句话的 SHA-256 的前 8 个字节；碰上 0 的换成 1（xorshift 从 0 出不来）。
    fn of(text: &str) -> Rng {
        let digest = Sha256::digest(text.as_bytes());
        let mut seed = [0u8; 8];
        seed.copy_from_slice(&digest[..8]);
        Rng(u64::from_le_bytes(seed).max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// 0 到 `bound - 1`。
    fn below(&mut self, bound: u32) -> u32 {
        u32::try_from(self.next() % u64::from(bound.max(1))).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests;
