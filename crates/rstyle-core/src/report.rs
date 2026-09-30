use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Detail {
    pub label: String,
    pub count: usize,
    pub lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub rule_id: String,
    pub message: String,
    pub actual: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<Detail>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub functions: usize,
    pub violations: usize,
    /// Mean of per-function `density_milli`, rounded down.
    pub mean_density_milli: usize,
    pub max_density_milli: usize,
    pub effect_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub summary: Summary,
}

pub fn sort(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|a, b| {
        (&a.path, a.line, a.column, &a.rule_id).cmp(&(&b.path, b.line, b.column, &b.rule_id))
    });
}

fn diagnostic_line(d: &Diagnostic) -> String {
    format!(
        "{}:{}:{}: {}: {} (actual {}, limit {})",
        d.path, d.line, d.column, d.rule_id, d.message, d.actual, d.limit
    )
}

pub fn text(report: &Report) -> String {
    let s = &report.summary;
    let mut lines: Vec<String> = report.diagnostics.iter().map(diagnostic_line).collect();
    lines.push(format!(
        "rstyle: {} functions, {} violations, mean density {}, max density {}",
        s.functions, s.violations, s.mean_density_milli, s.max_density_milli
    ));
    lines.join("\n")
}

pub fn json(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(path: &str, line: usize) -> Diagnostic {
        Diagnostic {
            path: path.into(),
            line,
            column: 1,
            rule_id: "x".into(),
            message: "message".into(),
            actual: 2,
            limit: 1,
            details: vec![],
        }
    }

    #[test]
    fn sort_is_by_path_then_line() {
        let mut values = vec![
            diagnostic("b.rs", 1),
            diagnostic("a.rs", 9),
            diagnostic("a.rs", 2),
        ];
        sort(&mut values);
        let order: Vec<_> = values.iter().map(|d| (d.path.as_str(), d.line)).collect();
        assert_eq!(order, [("a.rs", 2), ("a.rs", 9), ("b.rs", 1)]);
    }

    #[test]
    fn text_has_one_line_per_diagnostic_plus_footer() {
        let report = Report {
            diagnostics: vec![diagnostic("a.rs", 1)],
            summary: Summary::default(),
        };
        let rendered = text(&report);
        assert_eq!(rendered.lines().count(), 2);
        assert!(rendered.starts_with("a.rs:1:1: x: message (actual 2, limit 1)"));
    }

    #[test]
    fn json_snapshot() {
        let report = Report {
            diagnostics: vec![Diagnostic {
                details: vec![Detail {
                    label: "self".into(),
                    count: 4,
                    lines: vec![3, 4, 5, 6],
                }],
                ..diagnostic("a.rs", 1)
            }],
            summary: Summary {
                functions: 1,
                violations: 1,
                ..Summary::default()
            },
        };
        insta::assert_snapshot!(json(&report).unwrap());
    }
}
