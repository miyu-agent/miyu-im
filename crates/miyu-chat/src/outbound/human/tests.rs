//! 模拟真人一条分多条发送（`chat.md` 第五条「守着它的」，施工 I-26）：短的、长的、带换行的不拆；同一句每次一样；拆了的句号处
//! 都断、断开处不留句号逗号、每条至少两个字、至多五条；引号、代码、网址里不断；拆的比例照真人；停顿照字数。

use std::time::Duration;

use super::{Bubble, LONGEST, Rng, bubbles, pause};

/// 拼一批像她说的话：从几句短话里挑两到四句，用逗号或句号连起来。
fn corpus() -> Vec<String> {
    let parts = [
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
    let mut out = Vec::new();
    for a in 0..parts.len() {
        for b in 0..parts.len() {
            if a == b {
                continue;
            }
            let joint = if (a + b) % 3 == 0 { "。" } else { "，" };
            out.push(format!("{}{joint}{}", parts[a], parts[b]));
            let c = (a * 7 + b * 5) % parts.len();
            out.push(format!("{}，{}{joint}{}", parts[a], parts[b], parts[c]));
        }
    }
    out
}

/// 拆了的那些。
fn split_ones() -> Vec<(String, Vec<Bubble>)> {
    corpus()
        .into_iter()
        .map(|text| {
            let said = bubbles(&text);
            (text, said)
        })
        .filter(|(_, said)| said.len() > 1)
        .collect()
}

#[test]
fn short_long_and_multiline_stay_whole() {
    assert!(bubbles("  ").is_empty());
    let one = bubbles("在的");
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].text, "在的");
    assert_eq!(one[0].pause, Duration::ZERO);
    let long = "这个问题我也遇到过，".repeat(LONGEST / 10 + 1);
    assert_eq!(bubbles(&long).len(), 1, "超过 {LONGEST} 个字不拆");
    for text in corpus() {
        let middle = text.chars().count() / 2;
        let lines: String = text
            .chars()
            .enumerate()
            .flat_map(|(at, c)| (at == middle).then_some('\n').into_iter().chain([c]))
            .collect();
        assert_eq!(bubbles(&lines).len(), 1, "带换行的不拆：{lines}");
    }
}

#[test]
fn the_same_text_splits_the_same_way() {
    for text in corpus() {
        assert_eq!(bubbles(&text), bubbles(&text), "{text}");
    }
}

#[test]
fn split_ones_break_at_every_period_and_drop_the_marks() {
    let split = split_ones();
    assert!(!split.is_empty());
    for (text, said) in &split {
        assert!(said.len() <= 5, "{said:?}");
        for (index, bubble) in said.iter().enumerate() {
            let inner: String = bubble.text.chars().rev().skip(1).collect();
            assert!(!inner.contains('。'), "句号处都断：{said:?}");
            assert!(bubble.text.chars().count() >= 2, "{said:?}");
            if index + 1 < said.len() {
                assert!(
                    !bubble.text.ends_with(['，', '。', '；', '、']),
                    "断开处不留：{said:?}"
                );
            }
        }
        // 拼回去只少了断开处的标点。
        let joined: String = said.iter().map(|bubble| bubble.text.as_str()).collect();
        let bare =
            |s: &str| -> String { s.chars().filter(|c| !"，。；、".contains(*c)).collect() };
        assert_eq!(bare(&joined), bare(text));
    }
}

#[test]
fn quotes_code_and_links_are_not_cut() {
    let texts = [
        "他说「先装好，再重启，然后看日志」，我觉得没问题",
        "跑一下 `pacman -Syu，然后 reboot` 就行，别的不用管",
        "看这个 https://wiki.archlinux.org/a,b?c=1，里面写得很清楚，照做就行",
        "改成 `a != b` 试试，不行再说，我也不确定",
    ];
    for base in texts {
        for salt in 0..40 {
            let text = format!("{base}{}", "啊".repeat(salt % 3));
            for bubble in bubbles(&text) {
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
fn how_often_follows_people() {
    // 真人 26 到 50 个字的拆一半（0.5），10 个字以内的 9%：语料够大，差不多就行。
    let texts = corpus();
    let rate = |range: std::ops::RangeInclusive<usize>| {
        let picked: Vec<&String> = texts
            .iter()
            .filter(|text| range.contains(&text.chars().count()))
            .collect();
        let split = picked.iter().filter(|text| bubbles(text).len() > 1).count();
        split as f64 / picked.len() as f64
    };
    let middle = rate(26..=50);
    assert!(
        (0.38..=0.62).contains(&middle),
        "26 到 50 个字拆了 {middle}"
    );
    let short = rate(11..=25);
    assert!((0.18..=0.42).contains(&short), "11 到 25 个字拆了 {short}");
}

#[test]
fn pauses_follow_the_next_bubble() {
    for (_, said) in split_ones() {
        assert_eq!(said[0].pause, Duration::ZERO);
        for bubble in &said[1..] {
            let chars = bubble.text.chars().count() as u64;
            let median = (4000 + 550 * chars).min(14_000);
            let millis = bubble.pause.as_millis() as u64;
            assert!(
                millis >= median * 650 / 1000 && millis <= median * 1350 / 1000,
                "{bubble:?}"
            );
        }
    }
    // 中位照字数涨，到 14 秒为止。
    let mut rng = Rng(7);
    let long = (0..200).map(|_| pause(60, &mut rng)).max().unwrap();
    assert!(long <= Duration::from_millis(14_000 * 1350 / 1000));
    let mut rng = Rng(7);
    let short: Duration = (0..200).map(|_| pause(2, &mut rng)).sum();
    let mut rng = Rng(7);
    let longer: Duration = (0..200).map(|_| pause(20, &mut rng)).sum();
    assert!(longer > short * 2, "{short:?} {longer:?}");
}
