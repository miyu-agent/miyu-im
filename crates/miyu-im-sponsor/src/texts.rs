//! 给模型看的字（包文件夹 `miyu-im-sponsor/data/`，登记在仓库的 `docs/ledger.md`）：工具说明和答的几句，编进程序里。字段写 `{名字}`。

/// 工具说明：`description`、`parameters`。
pub const TOOL: &str = include_str!("../../../miyu-im-sponsor/data/tools/sponsor.json");

/// 答的几句：名字到模板。
const ANSWERS: [(&str, &str); 9] = [
    (
        "recorded",
        include_str!("../../../miyu-im-sponsor/data/tool-results/recorded.txt"),
    ),
    (
        "converted",
        include_str!("../../../miyu-im-sponsor/data/tool-results/converted.txt"),
    ),
    (
        "unconverted",
        include_str!("../../../miyu-im-sponsor/data/tool-results/unconverted.txt"),
    ),
    (
        "admin-only",
        include_str!("../../../miyu-im-sponsor/data/tool-results/admin-only.txt"),
    ),
    (
        "person",
        include_str!("../../../miyu-im-sponsor/data/tool-results/person.txt"),
    ),
    (
        "ranking",
        include_str!("../../../miyu-im-sponsor/data/tool-results/ranking.txt"),
    ),
    (
        "none",
        include_str!("../../../miyu-im-sponsor/data/tool-results/none.txt"),
    ),
    (
        "not-done",
        include_str!("../../../miyu-im-sponsor/data/tool-results/not-done.txt"),
    ),
    (
        "unreachable",
        include_str!("../../../miyu-im-sponsor/data/tool-results/unreachable.txt"),
    ),
];

/// 那一句 `name`，字段照 `fields` 换（一遍换完，字段里写的 `{…}` 不再换），去掉行尾的换行（纯逻辑）。字段里的换行写成空格：
/// 伪造不出别的行。没有这一句的是空的（照说不会：名字都写在这个文件里）。
pub fn say(name: &str, fields: &[(&str, &str)]) -> String {
    let template = ANSWERS
        .iter()
        .find(|(known, _)| *known == name)
        .map_or("", |(_, template)| template)
        .trim_end_matches(['\n', '\r']);
    let mut text = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        text.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let field = after.find('}').and_then(|close| {
            let key = &after[..close];
            let value = fields.iter().find(|(name, _)| *name == key)?.1;
            Some((value, close))
        });
        match field {
            Some((value, close)) => {
                text.push_str(&value.replace(['\n', '\r'], " "));
                rest = &after[close + 1..];
            }
            None => {
                text.push('{');
                rest = after;
            }
        }
    }
    text.push_str(rest);
    text
}

#[cfg(test)]
mod tests;
