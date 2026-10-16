//! 行数门禁：每个 `.rs` 文件不超过 [`MAX_LINES`] 行，测试也算（同核心：核心仓库 `01-架构.md` 第九节规则 4，设计 32 第五节）。

/// 一个 `.rs` 文件最多几行。
pub const MAX_LINES: usize = 500;

/// 超了就说一句；没超返回 `None`。
pub fn check(label: &str, text: &str, max: usize) -> Option<String> {
    let lines = text.lines().count();
    (lines > max).then(|| format!("{label} 有 {lines} 行，上限是 {max} 行：把它拆开"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_over_limit_is_rejected() {
        let text = "x\n".repeat(501);
        let problem = check("big.rs", &text, 500).unwrap();
        assert!(problem.contains("501 行"), "{problem}");
    }

    #[test]
    fn file_at_limit_passes() {
        assert_eq!(check("ok.rs", &"x\n".repeat(500), 500), None);
    }
}
