//! 这个群里她刚说过的话（施工 I-217，`nearby.rs`）：别的线刚说过的交出来，同一条线的、过了时候的不交；只留最近几句。

use super::Projection;
use super::nearby::WINDOW_MILLIS;
use super::tests::{LINE, at};

/// 支线的会话编号。
const LANE: &str = "lane-1";

#[test]
fn what_another_line_just_said_is_handed_to_the_other_lines_only() {
    let mut group = Projection::new(LINE, 0);
    assert!(group.nearby(at(0), LANE).is_empty(), "还没说过话");
    // 主线说了一句；2 秒以后支线要说话：看得到它。
    group.said_nearby(at(0), LINE, "这个也不说");
    assert_eq!(group.nearby(at(2), LANE), ["这个也不说"]);
    // 主线自己（这一轮、下一轮）都不比：一轮接一轮回的是不同的人，两句「在」都该发。
    assert!(group.nearby(at(2), LINE).is_empty());
    // 过了时候的不算：正好到头的还算，多一秒的不算。
    let window = WINDOW_MILLIS / 1000;
    assert_eq!(group.nearby(at(window), LANE).len(), 1);
    assert!(group.nearby(at(window + 1), LANE).is_empty());
    // 钟往回拨了的（比她说的时候还早）不算。
    assert!(group.nearby(at(-5), LANE).is_empty());
}

#[test]
fn only_the_last_few_lines_are_kept() {
    let mut group = Projection::new(LINE, 0);
    for n in 0..12 {
        group.said_nearby(at(0), LINE, &format!("第 {n} 句"));
    }
    let kept = group.nearby(at(1), LANE);
    assert_eq!(kept.len(), 8);
    assert_eq!(kept.first().map(String::as_str), Some("第 4 句"));
    assert_eq!(kept.last().map(String::as_str), Some("第 11 句"));
}
