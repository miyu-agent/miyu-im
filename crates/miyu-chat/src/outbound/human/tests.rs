//! 模拟真人一条分多条发送（`chat.md` 第五条「守着它的」，施工 I-26、I-49）：短的、长的、带换行的不拆；同一句每次一样；句末
//! 标点处都断、断开处不留句号逗号、每条至少三个字、至多五条；逗号照句子定、一句至多断一处，长句断得比短句多；引号、代码、网址
//! 里不断；停顿 = 想的工夫 + 照打字速度打这一条，第二条前面想得久，打字越快停得越短。

use std::time::Duration;

use super::{
    Bubble, LONG_SENTENCE, LONGEST, LONGEST_PAUSE, MAX_PIECES, MIN_CHARS, SHORTEST, THINK_FIRST,
    THINK_NEXT, bubbles,
};

/// 出厂的打字速度。
const SPEED: u32 = 80;

/// 几句短话。
const PARTS: [&str; 12] = [
    "今天天气不错",
    "我刚看了一下",
    "这个问题我也遇到过",
    "你可以先试试重启",
    "不行的话再说",
    "其实没那么复杂",
    "我翻了一下文档",
    "应该是配置写错了",
    "装完记得更新一下",
    "别急慢慢来",
    "我这边看不到图",
    "换个镜像源试试",
];

/// 用句号连起来的两三句（每句里也许有逗号）。
fn corpus() -> Vec<String> {
    let mut out = Vec::new();
    for (a, first) in PARTS.iter().enumerate() {
        for (b, second) in PARTS.iter().enumerate() {
            if a != b {
                let third = PARTS[(a * 7 + b * 5) % PARTS.len()];
                out.push(format!("{first}，{second}。{third}"));
                out.push(format!("{first}。{second}"));
            }
        }
    }
    out
}

/// 几句很短的话。
const SHORT_PARTS: [&str; 8] = [
    "好的",
    "我看看",
    "行吧",
    "在的",
    "可以啊",
    "明天吧",
    "真的假的",
    "还行",
];

/// 一句里带一个逗号（没有句末标点）：`long` 的够长句（照 [`PARTS`] 拼），不是的短句（照 [`SHORT_PARTS`] 拼）。
fn one_sentence(long: bool) -> Vec<String> {
    let parts: &[&str] = if long { &PARTS } else { &SHORT_PARTS };
    let mut out = Vec::new();
    for (a, first) in parts.iter().enumerate() {
        for (b, second) in parts.iter().enumerate() {
            let text = format!("{first}，{second}");
            if a != b && (text.chars().count() >= LONG_SENTENCE) == long {
                out.push(text);
            }
        }
    }
    out
}

#[test]
fn short_long_and_multiline_stay_whole() {
    assert!(bubbles("  ", SPEED).is_empty());
    let one = bubbles("在的", SPEED);
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].text, "在的");
    assert_eq!(one[0].pause, Duration::ZERO, "第一条当场发");
    assert!(
        one[0].typed >= Duration::from_millis(SHORTEST),
        "第一条也算好想加打的工夫"
    );
    let long = "这个问题我也遇到过。".repeat(LONGEST / 10 + 1);
    assert_eq!(bubbles(&long, SPEED).len(), 1, "超过 {LONGEST} 个字不拆");
    assert_eq!(
        bubbles("先这样。\n再那样。", SPEED).len(),
        1,
        "带换行的不拆"
    );
}

#[test]
fn the_same_text_splits_the_same_way() {
    for text in corpus() {
        assert_eq!(bubbles(&text, SPEED), bubbles(&text, SPEED), "{text}");
    }
}

#[test]
fn every_sentence_end_breaks_and_the_marks_go() {
    for text in corpus() {
        let said = bubbles(&text, SPEED);
        assert!(
            said.len() >= 2 && said.len() <= MAX_PIECES,
            "句号处都断：{said:?}"
        );
        for (index, bubble) in said.iter().enumerate() {
            assert!(!bubble.text.contains('。'), "{said:?}");
            assert!(bubble.text.chars().count() >= MIN_CHARS, "{said:?}");
            if index + 1 < said.len() {
                assert!(
                    !bubble.text.ends_with(['，', '；', '、']),
                    "断开处不留：{said:?}"
                );
            }
        }
        let joined: String = said.iter().map(|bubble| bubble.text.as_str()).collect();
        let bare =
            |s: &str| -> String { s.chars().filter(|c| !"，。；、".contains(*c)).collect() };
        assert_eq!(bare(&joined), bare(&text), "拼回去只少了断开处的标点");
    }
}

#[test]
fn a_short_piece_joins_its_neighbour() {
    // 「好」不够三个字，并进后一条；句号照样去掉。
    let said = bubbles("好。我等会儿看看这个", SPEED);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].text.starts_with('好'), "{said:?}");
}

#[test]
fn commas_break_by_the_sentence_and_at_most_once() {
    let rate = |texts: &[String]| {
        let split = texts
            .iter()
            .filter(|text| bubbles(text, SPEED).len() > 1)
            .count();
        split as f64 / texts.len() as f64
    };
    let long = one_sentence(true);
    let short = one_sentence(false);
    assert!(
        long.len() > 40 && short.len() > 20,
        "{} {}",
        long.len(),
        short.len()
    );
    let (long_rate, short_rate) = (rate(&long), rate(&short));
    assert!((0.15..=0.45).contains(&long_rate), "长句断了 {long_rate}");
    assert!(short_rate <= 0.2, "短句断了 {short_rate}");
    assert!(long_rate > short_rate, "长句比短句断得多");
    for text in long.iter().chain(&short) {
        assert!(bubbles(text, SPEED).len() <= 2, "一句至多断一处：{text}");
    }
    // 三个逗号的长句断开时只断一处，留着别的逗号。
    let three = "我刚看了一下，应该是配置写错了，换个镜像源试试，不行的话再说";
    for salt in 0..60 {
        let text = format!("{three}{}", "啊".repeat(salt % 5));
        let said = bubbles(&text, SPEED);
        assert!(said.len() <= 2, "{said:?}");
        if said.len() == 2 {
            assert!(
                said.iter().any(|bubble| bubble.text.contains('，')),
                "多数条里还留着逗号：{said:?}"
            );
        }
    }
}

#[test]
fn quotes_code_and_links_are_not_cut() {
    let texts = [
        "他说「先装好，再重启。然后看日志」，我觉得没问题",
        "跑一下 `pacman -Syu。然后 reboot` 就行，别的不用管",
        "看这个 https://wiki.archlinux.org/a,b?c=1，里面写得很清楚，照做就行",
        "改成 `a != b` 试试，不行再说，我也不确定",
    ];
    for base in texts {
        for salt in 0..40 {
            let text = format!("{base}{}", "啊".repeat(salt % 3));
            for bubble in bubbles(&text, SPEED) {
                let opened = bubble.text.matches('「').count();
                assert_eq!(opened, bubble.text.matches('」').count(), "{bubble:?}");
                assert_eq!(bubble.text.matches('`').count() % 2, 0, "{bubble:?}");
                if bubble.text.contains("https://") {
                    assert!(bubble.text.contains("a,b?c=1"), "{bubble:?}");
                }
            }
        }
    }
}

#[test]
fn pauses_are_thinking_plus_typing() {
    for text in corpus() {
        let said = bubbles(&text, SPEED);
        assert_eq!(said[0].pause, Duration::ZERO);
        for (index, bubble) in said.iter().enumerate().skip(1) {
            assert_eq!(bubble.pause, bubble.typed);
            let think = if index == 1 { THINK_FIRST } else { THINK_NEXT };
            let chars = bubble.text.chars().count() as u64;
            let base = think + chars * 60_000 / u64::from(SPEED);
            let millis = bubble.pause.as_millis() as u64;
            let low = (base * 800 / 1000).clamp(SHORTEST, LONGEST_PAUSE);
            let high = (base * 1200 / 1000).clamp(SHORTEST, LONGEST_PAUSE);
            assert!((low..=high).contains(&millis), "{bubble:?} 照 {base}");
        }
    }
    // 打字越快停得越短：同一句慢的每一段都不比快的短。
    let text = "我刚看了一下。应该是配置写错了。换个镜像源试试";
    let (slow, fast) = (bubbles(text, 40), bubbles(text, 200));
    let total = |said: &[Bubble]| -> Duration { said.iter().map(|bubble| bubble.pause).sum() };
    assert!(total(&slow) > total(&fast) * 2, "{slow:?} {fast:?}");
}
