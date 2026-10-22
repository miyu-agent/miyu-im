//! 冲她来（`docs/blueprint/chat.md` 第二条「怎么走」第 11 条，`18-通讯平台.md` 第七节，施工 O-12 下）：一条消息是不是对她
//! 说的，填 [`Said::addressed`](crate::Said::addressed)。限流满了只给冲她来的回一句，主动回复判断给冲她来的加 `direct` 分。
//!
//! 照旧版（`group_trigger_text`）：触发词只认开头，不认中间。中间出现就算的话，「为什么」这类词设成触发词会误叫；旧版
//! 08-29 实测过（施工时定的第 16 条）。私聊不看触发词，一律算（施工时定的第 17 条）。

use crate::VenueKind;

/// 一条消息是不是冲她来的（「怎么走」第 11 条）：
///
/// - 私聊（`kind` 是 [`VenueKind::Private`]）：一律是。
/// - 群：@ 了她（`mentions_me`）是；正文 `text` 去掉开头的空白以后，以 `keywords` 里某个触发词开头是。引用她的消息、不 @ 不算
///   （施工 I-20，2026-10-21 项目主人定）。
///
/// `keywords` 是场所规则的触发词：区分大小写，空的不算，原样比（不去空白）。名字也算触发词，写在里面，出厂是空的；这里
/// 不自动加上人格的名字。
pub fn addressed(kind: VenueKind, mentions_me: bool, text: &str, keywords: &[String]) -> bool {
    match kind {
        VenueKind::Private => true,
        VenueKind::Group => {
            let text = text.trim_start();
            mentions_me
                || keywords
                    .iter()
                    .any(|keyword| !keyword.is_empty() && text.starts_with(keyword.as_str()))
        }
    }
}

/// 冲她来的这一条是不是只叫了她、没说别的（施工 I-29）：正文 `text` 里的触发词 `keywords` 去掉、`@` 开头的词（@ 她、@ 别人，
/// 桥写成名字）去掉以后，一个字母、数字、汉字这类字都不剩（只有名字、标点、表情）。只带了图、没有字的由外面另算。旧 Miyu
/// 主群 3 天里冲她来的四分之一是这样的：「miyu」「😭」、一张图。
pub fn hollow(text: &str, keywords: &[String]) -> bool {
    let mut rest = text.to_string();
    for keyword in keywords.iter().filter(|keyword| !keyword.is_empty()) {
        rest = rest.replace(keyword.as_str(), " ");
    }
    !rest
        .split_whitespace()
        .filter(|word| !word.starts_with('@'))
        .any(|word| word.chars().any(char::is_alphanumeric))
}

#[cfg(test)]
mod tests;
