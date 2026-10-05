//! Line-level source helpers shared by the driver and the rules.

/// Counts lines that contain code: blank lines, `//` comment-only lines and
/// lines inside `/* */` blocks (nestable) are not counted.
pub fn code_lines(source: &str) -> usize {
    let mut depth = 0usize;
    source
        .lines()
        .filter(|line| line_has_code(line, &mut depth))
        .count()
}

/// True when any line is empty or whitespace-only. For fn source this is the
/// signature plus body, so a blank line anywhere in it counts as spacing.
pub fn has_blank_line(source: &str) -> bool {
    source.lines().any(|line| line.trim().is_empty())
}

fn line_has_code(line: &str, depth: &mut usize) -> bool {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut has_code = false;
    while index < bytes.len() {
        let rest = &bytes[index..];
        if rest.starts_with(b"/*") {
            *depth += 1;
            index += 2;
        } else if *depth > 0 && rest.starts_with(b"*/") {
            *depth -= 1;
            index += 2;
        } else if *depth == 0 && rest.starts_with(b"//") {
            break;
        } else {
            has_code |= *depth == 0 && !bytes[index].is_ascii_whitespace();
            index += 1;
        }
    }
    has_code
}

/// Picks the line a suppression comment may live on: the nearest line above
/// `fn_line_index` (0-based) that is not an attribute or doc comment.
pub fn preceding_comment_line(lines: &[&str], fn_line_index: usize) -> String {
    lines[..fn_line_index.min(lines.len())]
        .iter()
        .rev()
        .map(|line| line.trim())
        .find(|line| !line.starts_with("#[") && !line.starts_with("///"))
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_code_lines() {
        let source = "fn f() {\n\n    // note\n    /* a\n       b */\n    x();\n}";
        assert_eq!(code_lines(source), 3);
    }

    #[test]
    fn blank_line_detection() {
        assert!(has_blank_line("fn f() {\n    a();\n\n    b();\n}"));
        assert!(has_blank_line("a();\n   \nb();"));
        assert!(!has_blank_line("fn f() {\n    a();\n    // c\n    b();\n}"));
    }

    #[test]
    fn code_next_to_comments_counts() {
        assert_eq!(code_lines("x(); // tail\n/* c */ y();"), 2);
    }

    #[test]
    fn nested_block_comments() {
        assert_eq!(code_lines("/* a /* b */ still */ x();"), 1);
        assert_eq!(code_lines("/* a\n/* b */\nstill */\nx();"), 1);
    }

    #[test]
    fn suppression_line_skips_attributes_and_docs() {
        let lines = [
            "// rstyle-allow(fn_params): ffi",
            "/// docs",
            "#[inline]",
            "fn f() {}",
        ];
        assert_eq!(
            preceding_comment_line(&lines, 3),
            "// rstyle-allow(fn_params): ffi"
        );
        assert_eq!(preceding_comment_line(&lines, 0), "");
    }
}
