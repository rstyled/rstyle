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

/// One violating function inside a fix group.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GroupFunction {
    pub path: String,
    pub line: usize,
    pub name: String,
    pub rules: Vec<String>,
}

/// A soft link to other groups: not a reason to stack, but fixes may touch the same thing.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Hint {
    /// `shared_type` (a mutated workspace type) or `shared_caller` (a caller that
    /// receives moved effects, or is itself being fixed in another group).
    pub kind: String,
    pub item: String,
    pub other_groups: usize,
}

/// A parameter as written.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ParamRef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// Parameters that callers hand to a re-signed function unchanged, several at once:
/// a candidate params struct.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Bundle {
    pub params: Vec<ParamRef>,
    /// Call sites that forward all of `params`.
    pub sites: usize,
    /// Functions on either side of those call sites, sorted.
    pub functions: Vec<String>,
}

/// One edit in a parameter refactor.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PlanStep {
    pub path: String,
    pub line: usize,
    pub name: String,
    /// The function's own signature changes (it violates `fn_params`); otherwise
    /// only its call sites do.
    pub signature: bool,
    /// Call sites in the workspace that call this function.
    pub incoming_sites: usize,
    /// Call sites in this function's body that call a re-signed function.
    pub outgoing_sites: usize,
    /// Parameters only ever forwarded to re-signed functions: the fix belongs in the
    /// callee or an upper layer, and they can leave this signature.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pass_through: Vec<String>,
}

/// Ordered parameter refactor for a group: callee signatures first, then callers.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StackPlan {
    pub steps: Vec<PlanStep>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bundles: Vec<Bundle>,
}

/// Violations whose fixes overlap: land them in one PR, or stack them in `functions`
/// order (callees first). Different groups can be fixed independently, in parallel.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FixGroup {
    pub id: usize,
    pub functions: Vec<GroupFunction>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<Hint>,
    /// Present when a member violates `fn_params`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<StackPlan>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub summary: Summary,
    pub fix_groups: Vec<FixGroup>,
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

fn group_lines(group: &FixGroup) -> Vec<String> {
    let head = format!(
        "  group {} ({} functions; land together or stack in this order):",
        group.id,
        group.functions.len()
    );
    let plan = group.plan.iter().flat_map(plan_lines);
    let members = group.functions.iter().map(|f| {
        format!(
            "    {}:{} {} [{}]",
            f.path,
            f.line,
            f.name,
            f.rules.join(", ")
        )
    });
    std::iter::once(head).chain(members).chain(plan).collect()
}

fn plan_lines(plan: &StackPlan) -> Vec<String> {
    let steps = plan.steps.iter().enumerate().map(|(i, s)| {
        let what = if s.signature {
            "signature"
        } else {
            "call sites"
        };
        let pass = if s.pass_through.is_empty() {
            String::new()
        } else {
            format!("; pass-through: {}", s.pass_through.join(", "))
        };
        format!(
            "      {}. {} {} ({}:{}; {} incoming, {} outgoing sites{pass})",
            i + 1,
            what,
            s.name,
            s.path,
            s.line,
            s.incoming_sites,
            s.outgoing_sites
        )
    });
    let bundles = plan.bundles.iter().map(|b| {
        let params: Vec<String> = b
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.ty))
            .collect();
        format!("      bundle ({} sites): {}", b.sites, params.join(", "))
    });
    steps.chain(bundles).collect()
}

/// Groups worth printing: several functions, or a parameter plan that edits callers.
fn listed(group: &FixGroup) -> bool {
    group.functions.len() > 1 || group.plan.as_ref().is_some_and(|p| p.steps.len() > 1)
}

/// Listed groups are printed; the rest are only counted.
fn groups_text(groups: &[FixGroup]) -> Vec<String> {
    let stacked: Vec<&FixGroup> = groups.iter().filter(|g| listed(g)).collect();
    let mut lines = vec![format!(
        "fix groups: {} ({} stacked, {} independent)",
        groups.len(),
        stacked.len(),
        groups.len() - stacked.len()
    )];
    lines.extend(stacked.iter().flat_map(|g| group_lines(g)));
    lines
}

pub fn text(report: &Report) -> String {
    let s = &report.summary;
    let mut lines: Vec<String> = report.diagnostics.iter().map(diagnostic_line).collect();
    if !report.fix_groups.is_empty() {
        lines.extend(groups_text(&report.fix_groups));
    }
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
            fix_groups: vec![],
        };
        let rendered = text(&report);
        assert_eq!(rendered.lines().count(), 2);
        assert!(rendered.starts_with("a.rs:1:1: x: message (actual 2, limit 1)"));
    }

    #[test]
    fn plan_renders_steps_and_bundles_under_its_group() {
        let step = |name: &str, signature: bool| PlanStep {
            path: "a.rs".into(),
            line: 1,
            name: name.into(),
            signature,
            incoming_sites: 2,
            outgoing_sites: 0,
            pass_through: vec!["root".into()],
        };
        let plan = StackPlan {
            steps: vec![step("callee", true), step("caller", false)],
            bundles: vec![Bundle {
                params: vec![ParamRef {
                    name: "root".into(),
                    ty: "&str".into(),
                }],
                sites: 3,
                functions: vec![],
            }],
        };
        let lines = plan_lines(&plan);
        assert_eq!(
            lines[0],
            "      1. signature callee (a.rs:1; 2 incoming, 0 outgoing sites; pass-through: root)"
        );
        assert!(lines[1].contains("2. call sites caller"));
        assert_eq!(lines[2], "      bundle (3 sites): root: &str");
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
            fix_groups: vec![],
        };
        insta::assert_snapshot!(json(&report).unwrap());
    }
}
