//! 读 Markdown 文档里的一节和它的表格（照核心仓库的 `xtask/src/drawing.rs`，只留登记簿用得上的几样）。文档的格式被改坏了，
//! 这里说清楚读不出来的是什么。

/// 标题里带 `title` 的那一节（`### ` 开头），到下一个同级或更高级的标题为止。
///
/// # Errors
///
/// 找不到这样的一节。
pub(crate) fn section<'a>(text: &'a str, title: &str) -> Result<Vec<&'a str>, String> {
    let mut lines = text.lines();
    lines
        .by_ref()
        .find(|line| line.starts_with("### ") && line.contains(title))
        .ok_or_else(|| format!("找不到标题里带「{title}」的一节"))?;
    Ok(lines
        .take_while(|line| !line.starts_with("### ") && !line.starts_with("## "))
        .collect())
}

/// 一张表格：表头和每一行的格子。
pub(crate) struct Table {
    pub(crate) header: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
}

/// 一节里所有的表格。表格是连续的、以 `|` 开头的行；第二行是分隔行。
pub(crate) fn tables(section: &[&str]) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in section.iter().chain(std::iter::once(&"")) {
        if line.trim_start().starts_with('|') {
            current.push(line);
        } else if !current.is_empty() {
            if current.len() >= 2 {
                tables.push(Table {
                    header: cells(current[0]),
                    rows: current[2..].iter().map(|row| cells(row)).collect(),
                });
            }
            current.clear();
        }
    }
    tables
}

fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(|c| c.trim().to_string()).collect()
}

/// 表头正好是 `header` 的那一张表。
///
/// # Errors
///
/// 没有这样的表。
pub(crate) fn find_table<'a>(tables: &'a [Table], header: &[&str]) -> Result<&'a Table, String> {
    tables
        .iter()
        .find(|t| {
            t.header
                .iter()
                .map(String::as_str)
                .eq(header.iter().copied())
        })
        .ok_or_else(|| format!("找不到表头是「{}」的表格", header.join(" | ")))
}

/// 一段文字里所有用反引号括起来的名字。
pub(crate) fn backticked(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "\
### 一、别的

| a | b |
|---|---|
| 1 | 2 |

### 二、要的

前面一段话。

| x | y |
|---|---|
| `p` | `q` 和 `r` |
| s | t |

### 三、后面的
";

    #[test]
    fn the_section_ends_at_the_next_heading() {
        let section = section(TEXT, "要的").unwrap();
        assert_eq!(section.first(), Some(&""));
        assert!(!section.iter().any(|line| line.contains("后面")));
        assert!(super::section(TEXT, "没有的").is_err());
    }

    #[test]
    fn the_table_is_found_by_its_header() {
        let section = section(TEXT, "要的").unwrap();
        let tables = tables(&section);
        let table = find_table(&tables, &["x", "y"]).unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(backticked(&table.rows[0][1]), ["q", "r"]);
        assert!(find_table(&tables, &["a", "b"]).is_err());
    }
}
