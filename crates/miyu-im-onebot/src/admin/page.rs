//! 群管理在 im 页面上的两块（施工 Q-6，设计 02 第一节第 5、6 条、第四节第 4、5 条）：会话浮窗的「群管理」页（这个群的刷屏处理、
//! 记录、排行），面板（一个群一项：她的身份、按动作筛的记录、排行榜、清空）。写法照核心的界面块，纯逻辑；字是语言到字的表，宿主
//! 照页面的语言挑。
//!
//! 记录一条一行：做了什么、对谁（标题），谁做的、理由（说明），时刻（右边）；禁言的带状态：还在禁言中（`doing`）、已解禁（后来有
//! 对这个人的解禁，`done`）、已过期（`off`）。

use serde_json::{Value, json};

use miyu_im::toolset::clock;

use super::groups::Group;
use super::records::{Action, By, Record, Verdict};
use super::summary::{self, Period};
use super::sync::spoken;

/// 列几条记录、几个人。
pub(crate) const SHOWN: usize = 50;

/// 浮窗里列几条。
const PANEL_SHOWN: usize = 10;

/// 一句话的三种语言。
pub(crate) fn t(zh: &str, en: &str, ja: &str) -> Value {
    json!({"zh": zh, "en": en, "ja": ja})
}

/// 多久，给人看的三种语言（天、小时、分钟各一段，0 的不写，不到一分钟的照一分钟）：`[中文, 英文, 日文]`。
fn lengths(seconds: u64) -> [String; 3] {
    let minutes = seconds.div_ceil(60).max(1);
    let parts = [minutes / 1440, minutes % 1440 / 60, minutes % 60];
    let say = |units: [&str; 3], join: &str| {
        parts
            .iter()
            .zip(units)
            .filter(|(count, _)| **count > 0)
            .map(|(count, unit)| format!("{count}{unit}"))
            .collect::<Vec<_>>()
            .join(join)
    };
    [
        say([" 天", " 小时", " 分钟"], " "),
        spoken(seconds),
        say(["日", "時間", "分"], ""),
    ]
}

/// 一个人：名字，没有的写号。
fn who(name: Option<&str>, user: i64) -> String {
    name.map_or_else(|| user.to_string(), str::to_string)
}

/// 一条记录是禁言的话现在怎样：`(状态, 字)`；别的是空的。`later` 是这一条以后的记录。
fn mute_state(record: &Record, later: &[Record], now: u64) -> Option<(&'static str, Value)> {
    let seconds = record.seconds.filter(|_| record.action == Action::Mute)?;
    let lifted = later.iter().any(|after| {
        after.user == record.user && matches!(after.action, Action::Unmute | Action::Kick)
    });
    Some(match (lifted, now < record.at.saturating_add(seconds)) {
        (true, _) => ("done", t("已解禁", "Unmuted", "解除済み")),
        (false, true) => ("doing", t("禁言中", "Muted", "ミュート中")),
        (false, false) => ("off", t("已过期", "Expired", "期限切れ")),
    })
}

/// 记录 `records`（旧的在前）的第 `index` 条一行。
fn item(records: &[Record], index: usize, now: u64) -> Value {
    let record = &records[index];
    let name = who(record.name.as_deref(), record.user);
    let title = match (record.action, record.seconds) {
        (Action::Mute, Some(seconds)) => {
            let [zh, en, ja] = lengths(seconds);
            t(
                &format!("禁言 {name} {zh}"),
                &format!("Muted {name} for {en}"),
                &format!("{name} を {ja} ミュート"),
            )
        }
        (Action::Mute, None) => t(
            &format!("禁言 {name}"),
            &format!("Muted {name}"),
            &format!("{name} をミュート"),
        ),
        (Action::Unmute, _) => t(
            &format!("解禁 {name}"),
            &format!("Unmuted {name}"),
            &format!("{name} のミュートを解除"),
        ),
        (Action::Kick, _) if record.block => t(
            &format!("踢出并拉黑 {name}"),
            &format!("Removed and blocked {name}"),
            &format!("{name} を退出させ再参加を拒否"),
        ),
        (Action::Kick, _) => t(
            &format!("踢出 {name}"),
            &format!("Removed {name}"),
            &format!("{name} を退出"),
        ),
        (Action::Join, _) => t(
            &format!("入群申请 {name}"),
            &format!("Join request from {name}"),
            &format!("{name} の参加申請"),
        ),
    };
    let by = match &record.by {
        By::Ai => t("AI", "AI", "AI"),
        By::Admin { user, name } => {
            let name = who(name.as_deref(), *user);
            t(
                &format!("管理员 {name}"),
                &format!("Admin {name}"),
                &format!("管理者 {name}"),
            )
        }
        By::Spam => t("刷屏处理", "Flood rule", "連投稿の処理"),
    };
    let detail = match &record.reason {
        Some(reason) => {
            let mut detail = by.clone();
            for (_, text) in detail.as_object_mut().into_iter().flatten() {
                *text = json!(format!("{} · {reason}", text.as_str().unwrap_or_default()));
            }
            detail
        }
        None => by,
    };
    let mut item = json!({"title": title, "detail": detail, "right": clock(record.at)});
    // 入群审批（施工 Q-7）：说明是申请人写的，右边是结论。
    if let Some(verdict) = record.verdict {
        let (said, tone) = match verdict {
            Verdict::Approve => (t("通过", "Approved", "承認"), "good"),
            Verdict::Reject => (t("拒绝", "Rejected", "拒否"), "bad"),
            Verdict::Hold => (t("留待人工", "Left to a person", "保留"), "warn"),
        };
        if let Some(comment) = &record.comment {
            item["detail"] = json!(comment);
        }
        item["right"] =
            json!([{"text": said, "tone": tone}, {"text": clock(record.at), "tone": "dim"}]);
    }
    // 禁言的状态写在时刻前面，带语气（设计 02 第一节第 5 条：还在禁言中、已解禁、已过期）。
    if let Some((state, said)) = mute_state(record, &records[index + 1..], now) {
        let tone = match state {
            "doing" => "warn",
            "done" => "good",
            _ => "dim",
        };
        item["state"] = json!(state);
        item["right"] =
            json!([{"text": said, "tone": tone}, {"text": clock(record.at), "tone": "dim"}]);
    }
    item
}

/// 记录 `records`（旧的在前）里动作是 `action`（`all` 是都要）的，新的在前最多 `most` 条：一段 `list`；没有的一段淡字。
fn events(records: &[Record], action: &str, now: u64, most: usize) -> Value {
    let wanted = |record: &Record| match action {
        "mute" => record.action == Action::Mute,
        "unmute" => record.action == Action::Unmute,
        "kick" => record.action == Action::Kick,
        "join" => record.action == Action::Join,
        _ => true,
    };
    let items: Vec<Value> = (0..records.len())
        .rev()
        .filter(|index| wanted(&records[*index]))
        .take(most)
        .map(|index| item(records, index, now))
        .collect();
    match items.is_empty() {
        true => {
            json!({"type": "text", "text": t("还没有记录。", "No records yet.", "記録はまだありません。"), "tone": "dim"})
        }
        false => json!({"type": "list", "items": items}),
    }
}

/// 排行榜 `period` 的一段 `list`，最多 `most` 个人；没有的一段淡字。
fn ranking(records: &[Record], period: Period, now: u64, most: usize) -> Value {
    let items: Vec<Value> = summary::ranking(records, period, now)
        .iter()
        .take(most)
        .enumerate()
        .map(|(index, tally)| {
            let name = who(tally.name.as_deref(), tally.user);
            // 次数是 0 的那一段不写。
            let [zh, en, ja] = lengths(tally.seconds);
            let mut parts: [Vec<String>; 3] = Default::default();
            if tally.mutes > 0 {
                parts[0].push(format!("禁言 {} 次，共 {zh}", tally.mutes));
                parts[1].push(format!("Muted {} times, {en} in all", tally.mutes));
                parts[2].push(format!("ミュート {} 回、計 {ja}", tally.mutes));
            }
            if tally.kicks > 0 {
                parts[0].push(format!("踢出 {} 次", tally.kicks));
                parts[1].push(format!("removed {} times", tally.kicks));
                parts[2].push(format!("退出 {} 回", tally.kicks));
            }
            let detail = t(
                &parts[0].join("；"),
                &parts[1].join("; "),
                &parts[2].join("・"),
            );
            let mut item = json!({"title": format!("{}. {name}", index + 1), "detail": detail});
            if let Some(reason) = &tally.reason {
                item["right"] = json!(reason);
            }
            item
        })
        .collect();
    match items.is_empty() {
        true => {
            json!({"type": "text", "text": t("这段时间没有人被管过。", "Nobody was moderated in this period.", "この期間に管理された人はいません。"), "tone": "dim"})
        }
        false => json!({"type": "list", "items": items}),
    }
}

/// 会话浮窗的「群管理」页（群 `group`，设置 `settings`，记录 `records`，成员汇总开没开 `summary`，此刻 `now`）。
pub(crate) fn panel(
    group: i64,
    settings: &Group,
    records: &[Record],
    summary: bool,
    now: u64,
) -> Value {
    let spam = settings.spam;
    let mut blocks = vec![
        json!({"type": "heading", "text": t("刷屏处理", "Flood handling", "連投稿の処理")}),
        json!({"type": "form", "id": format!("spam-{group}"), "fields": [
            {"key": "on", "label": t("开启刷屏处理", "Handle floods", "連投稿を処理する"), "hint": t("同一个人连着发三次同样的话就撤回重复的、禁言。", "When someone sends the same words three times in a row, recall the repeats and mute them.", "同じ人が同じ言葉を三回続けたら、重複を取り消してミュートします。"), "control": "toggle", "value": spam.on},
            {"key": "window", "label": t("几秒以内", "Within seconds", "何秒以内"), "control": "number", "value": spam.window, "min": 10, "max": 3600, "integer": true},
            {"key": "mute", "label": t("禁言几分钟", "Mute minutes", "ミュートする分"), "control": "number", "value": spam.mute / 60, "min": 1, "max": 43200, "integer": true}
        ], "submit": {"label": t("保存", "Save", "保存"), "call": "admin.spam", "params": {"group": group}}}),
        json!({"type": "heading", "text": t("入群审批", "Join requests", "参加審査")}),
        json!({"type": "form", "id": format!("approval-{group}"), "fields": [
            {"key": "rule", "label": t("通过条件", "Rule to pass", "承認の条件"), "hint": t("写能接受的答案，原样交给 AI 审；群设了入群问题的，问题也一起带上。空着的不审，留给人工。", "Write the answers you accept; the AI reviews requests against it as written, with the group's join question. Leave it empty to review by hand.", "受け入れる答えを書きます。AI がそのまま照らし合わせて審査します（グループの参加質問も一緒に）。空なら手動で審査します。"), "control": "textarea", "value": settings.approval.rule}
        ], "submit": {"label": t("保存", "Save", "保存"), "call": "admin.approval", "params": {"group": group}}}),
        json!({"type": "heading", "text": t("最近的管理记录", "Recent moderation", "最近の管理記録")}),
        events(records, "all", now, PANEL_SHOWN),
    ];
    if summary {
        blocks.push(json!({"type": "heading", "text": t("本周排行", "This week's ranking", "今週のランキング")}));
        blocks.push(ranking(records, Period::Week, now, 5));
    }
    Value::Array(blocks)
}

/// 面板上一个群（号 `group`、名字 `name`，她在这个群是不是管理员 `admin`，问不到的是空的）的那一项。
pub(crate) fn section(
    (group, name): (i64, Option<&str>),
    admin: Option<bool>,
    (settings, records): (&Group, &[Record]),
    (summary, now): (bool, u64),
) -> Value {
    let (badge, state) = match admin {
        Some(true) => (
            json!({"text": t("管理员", "Admin", "管理者"), "tone": "good"}),
            "good",
        ),
        Some(false) => (
            json!({"text": t("不是管理员", "Not an admin", "管理者ではない"), "tone": "warn"}),
            "warn",
        ),
        None => (
            json!({"text": t("问不到身份", "Role unknown", "役割不明"), "tone": "dim"}),
            "off",
        ),
    };
    let view = &settings.view;
    let period = Period::read(&view.period).unwrap_or(Period::Week);
    let option = |value: &str, label: Value| json!({"value": value, "label": label});
    let mut blocks = vec![
        json!({"type": "form", "id": format!("view-{group}"), "fields": [
            {"key": "action", "label": t("看哪一种", "Show", "表示"), "control": "select", "value": view.action, "options": [
                option("all", t("全部", "All", "すべて")), option("mute", t("禁言", "Mutes", "ミュート")),
                option("unmute", t("解禁", "Unmutes", "解除")), option("kick", t("踢人", "Removals", "退出")),
                option("join", t("入群审批", "Join requests", "参加審査"))
            ]},
            {"key": "period", "label": t("排行榜看多久", "Ranking period", "ランキングの期間"), "control": "select", "value": period.name(), "options": [
                option("week", t("一周", "Week", "一週間")), option("month", t("一个月", "Month", "一か月")), option("all", t("全部", "All", "すべて"))
            ]}
        ], "submit": {"label": t("看", "Show", "表示"), "call": "admin.view", "params": {"group": group}}}),
        json!({"type": "heading", "text": t("管理记录", "Moderation records", "管理記録")}),
        events(records, &view.action, now, SHOWN),
    ];
    if summary {
        blocks.push(json!({"type": "heading", "text": t("排行榜", "Ranking", "ランキング")}));
        blocks.push(ranking(records, period, now, SHOWN));
    }
    let asked = settings
        .clearing
        .is_some_and(|at| now < at.saturating_add(60));
    let clear = match asked {
        true => t(
            "确认清空（一分钟内再点一次）",
            "Confirm clearing (click again within a minute)",
            "消去を確定（一分以内にもう一度）",
        ),
        false => t(
            "清空这个群的记录",
            "Clear this group's records",
            "このグループの記録を消去",
        ),
    };
    blocks.push(json!({"type": "buttons", "items": [{"label": clear, "action": {"call": "admin.clear", "params": {"group": group}}}]}));
    json!({
        "id": group.to_string(),
        "title": name.map_or_else(|| group.to_string(), str::to_string),
        "subtitle": group.to_string(),
        "badge": badge,
        "state": state,
        "blocks": blocks,
    })
}

#[cfg(test)]
mod tests;
