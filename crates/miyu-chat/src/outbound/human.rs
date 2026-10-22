//! 照真人的样子一句分几条发（`docs/blueprint/chat.md` 第五条「怎么走」第 10 条，施工 I-26，2026-10-22 项目主人定：程序切、
//! 完全照真人、默认开）。数都照旧群 130515298 的真人记录（11.5 万条；一个人 20 秒内连着发的算一串，8.2 万串）：
//!
//! - 拆不拆、拆几条照这么长的话真人拆的比例（[`SPLIT_RATE`]、[`PIECES`]）抽；超过 [`LONGEST`] 个字的、带换行的不拆（讲解、
//!   列步骤，真人话长了也多半一口气发）。
//! - 拆就在句号、问号、叹号、分号、省略号处都断（真人一条里带句号的只有 1%），不够条数再挑逗号处补，挑法让每条差不多长，
//!   第一个逗号优先（项目主人：第一个逗号通常也可以作为切分点），加一点随机。引号、括号、代码、网址里面不断。
//! - 断开处的句号、分号、逗号、顿号去掉（真人非最后一条 94% 不带标点），问号、叹号留着。
//! - 每条前面停一会儿，照真人「下一条越长停越久」（[`pause`]）。
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
}

/// 超过这么多个字的不拆。
const LONGEST: usize = 200;

/// 一句最多拆成几条（真人 51 到 200 字的串有拆成 8 条以上的，多半是几件事连着说，照 5 收住）。
const MAX_PIECES: usize = 5;

/// 一条至少这么多个字（不算末尾去掉的标点）：真人多条的串里每条 p10 是 3 个字。
const MIN_CHARS: usize = 2;

/// 这么长以内的话拆成多条的千分比：真人 ≤10 字 9%、≤25 30%、≤50 50%、≤100 55%、≤200 42%。
const SPLIT_RATE: [(usize, u32); 5] = [(10, 90), (25, 300), (50, 500), (100, 550), (LONGEST, 420)];

/// 拆了的拆成 2、3、4、5 条的权重，同 [`SPLIT_RATE`] 分档（5 条以上的并进 5 条）。
const PIECES: [[u32; 4]; 5] = [
    [76, 19, 4, 1],
    [76, 19, 4, 1],
    [47, 30, 13, 10],
    [28, 17, 15, 40],
    [42, 10, 5, 43],
];

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

/// 照真人的样子把一句 `text` 拆成几条（见模块开头）：去掉首尾空白，空的交回空的；不拆的交回原样一条。
#[must_use]
pub fn bubbles(text: &str) -> Vec<Bubble> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let whole = || {
        vec![Bubble {
            text: text.to_string(),
            pause: Duration::ZERO,
        }]
    };
    let chars: Vec<char> = text.chars().collect();
    let Some(tier) = SPLIT_RATE.iter().position(|(top, _)| chars.len() <= *top) else {
        return whole();
    };
    if text.contains('\n') {
        return whole();
    }
    let mut rng = Rng::of(text);
    if rng.below(1000) >= SPLIT_RATE[tier].1 {
        return whole();
    }
    let wanted = pick(&PIECES[tier], &mut rng) + 2;
    let pieces = cut(&chars, wanted, &mut rng);
    if pieces.len() < 2 {
        return whole();
    }
    let last = pieces.len() - 1;
    pieces
        .into_iter()
        .enumerate()
        .map(|(index, piece)| {
            let text = match index == last {
                true => piece.trim().to_string(),
                false => trimmed(&piece).to_string(),
            };
            let pause = match index {
                0 => Duration::ZERO,
                _ => pause(text.chars().count(), &mut rng),
            };
            Bubble { text, pause }
        })
        .collect()
}

/// 发下一条以前停多久：下一条 `chars` 个字。真人两条之间的中位是下一条 1 到 3 个字 5 秒、4 到 7 个字 7 秒、8 到 12 个字
/// 10 秒、13 到 20 个字 12 秒、更长的 14 秒，约是 4 秒加每字 0.55 秒、最多 14 秒；上下抖到中位的 0.65 到 1.35 倍（真人
/// p25 到 p75 大致是这一圈）。
fn pause(chars: usize, rng: &mut Rng) -> Duration {
    let chars = u64::try_from(chars).unwrap_or(u64::MAX);
    let median = chars.saturating_mul(550).saturating_add(4000).min(14_000);
    let factor = 650 + u64::from(rng.below(701));
    Duration::from_millis(median * factor / 1000)
}

/// 照权重 `weights` 抽一个下标。
fn pick(weights: &[u32], rng: &mut Rng) -> usize {
    let total: u32 = weights.iter().sum();
    let mut left = rng.below(total.max(1));
    for (index, weight) in weights.iter().enumerate() {
        if left < *weight {
            return index;
        }
        left -= weight;
    }
    weights.len() - 1
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

/// 照想要的条数 `wanted` 挑断的地方：句子完了的都断（加起来超过 [`MAX_PIECES`] 条的不硬断），不够的挑逗号补；每条
/// 尽量一样长，逗号比句号多一点代价，第一个逗号少一点，再加一点随机。断不出至少两条的交回一条。
fn cut(chars: &[char], wanted: usize, rng: &mut Rng) -> Vec<String> {
    let cuts = cuts(chars);
    let strong = cuts.iter().filter(|(_, strong)| *strong).count();
    let wanted = wanted.max(strong + 1).min(cuts.len() + 1).min(MAX_PIECES);
    if wanted < 2 {
        return vec![chars.iter().collect()];
    }
    let forced = strong < MAX_PIECES;
    let first_weak = cuts.iter().position(|(_, strong)| !*strong);
    let noise: Vec<f64> = cuts.iter().map(|_| rng.unit() * 2.0).collect();
    // 位置：开头、每个能断的地方、结尾。
    let mut at = vec![0];
    at.extend(cuts.iter().map(|(after, _)| *after));
    at.push(chars.len());
    let points = at.len();
    let target = chars.len() as f64 / wanted as f64;
    let cost = |point: usize| -> f64 {
        if point == points - 1 {
            return 0.0;
        }
        let (_, strong) = cuts[point - 1];
        let weak = if strong { 0.0 } else { 1.5 };
        let first = if first_weak == Some(point - 1) {
            1.0
        } else {
            0.0
        };
        weak - first + noise[point - 1]
    };
    let must = |point: usize| point > 0 && point < points - 1 && forced && cuts[point - 1].1;
    // best[j][c]：在第 j 个位置断、已经断成 c 条的最小代价，back 记前一个位置。
    let mut best = vec![vec![f64::INFINITY; wanted + 1]; points];
    let mut back = vec![vec![usize::MAX; wanted + 1]; points];
    best[0][0] = 0.0;
    for j in 1..points {
        let here = cost(j);
        for i in 0..j {
            if (i + 1..j).any(must) {
                continue;
            }
            let piece: String = chars[at[i]..at[j]].iter().collect();
            let length = trimmed(&piece).chars().count();
            if length < MIN_CHARS {
                continue;
            }
            let spread = (length as f64 - target).powi(2) / target;
            for c in 1..=wanted {
                let before = best[i][c - 1];
                if before.is_finite() && before + spread + here < best[j][c] {
                    best[j][c] = before + spread + here;
                    back[j][c] = i;
                }
            }
        }
    }
    let last = points - 1;
    let Some(mut count) = (1..=wanted).rev().find(|&c| best[last][c].is_finite()) else {
        return vec![chars.iter().collect()];
    };
    let mut ends = Vec::new();
    let mut j = last;
    while j > 0 {
        ends.push(j);
        j = back[j][count];
        count -= 1;
    }
    ends.reverse();
    let mut start = 0;
    ends.iter()
        .map(|&end| {
            let piece = chars[at[start]..at[end]].iter().collect();
            start = end;
            piece
        })
        .collect()
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

    /// 0 到 1。
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests;
