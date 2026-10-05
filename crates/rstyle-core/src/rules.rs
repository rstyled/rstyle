//! Pure rules over `FnFacts`. No IO, no compiler types.

use crate::config::{Config, Granularity, MutationBudgetConfig};
use crate::facts::{EffectKind, FnFacts};
use crate::lines::{code_lines, has_blank_line};
use crate::report::{Detail, Diagnostic};
use std::collections::BTreeMap;

/// The measured value and its limit, kept together so rules can't swap them.
#[derive(Debug, Clone, Copy)]
pub struct Measure {
    pub actual: usize,
    pub limit: usize,
}

fn diagnostic(f: &FnFacts, rule_id: &str, message: String, measure: Measure) -> Diagnostic {
    Diagnostic {
        path: f.path.clone(),
        line: f.line,
        column: f.column,
        rule_id: rule_id.into(),
        message,
        actual: measure.actual,
        limit: measure.limit,
        details: vec![],
    }
}

pub fn fn_length(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let actual = code_lines(&f.source);
    let limit = cfg.fn_length.max_lines;
    let message = format!("function `{}` is too long", f.name);
    (actual > limit)
        .then(|| diagnostic(f, "fn_length", message, Measure { actual, limit }))
        .into_iter()
        .collect()
}

pub fn fn_params(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let counted_self = usize::from(cfg.fn_params.count_self && f.has_self);
    let actual = f.params + counted_self;
    let limit = cfg.fn_params.max_params;
    let message = format!("function `{}` has too many parameters", f.name);
    (actual > limit)
        .then(|| diagnostic(f, "fn_params", message, Measure { actual, limit }))
        .into_iter()
        .collect()
}

/// Flags fns with `min_lines`+ code lines and no blank line anywhere. Reported as
/// `actual > limit` with `limit = min_lines - 1`, like the other threshold rules.
pub fn fn_spacing(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let min_lines = cfg.fn_spacing.min_lines;
    let actual = code_lines(&f.source);
    let message = format!("function `{}` has no blank line between its phases", f.name);
    let limit = min_lines.saturating_sub(1);
    let flagged = min_lines > 0 && actual >= min_lines && !has_blank_line(&f.source);

    flagged
        .then(|| diagnostic(f, "fn_spacing", message, Measure { actual, limit }))
        .into_iter()
        .collect()
}

fn mutation_groups(f: &FnFacts, granularity: Granularity) -> BTreeMap<String, Vec<usize>> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for event in &f.mutations {
        let key = match granularity {
            Granularity::Field if !event.field_path.is_empty() => {
                format!("{}.{}", event.root, event.field_path)
            }
            _ => event.root.clone(),
        };
        groups.entry(key).or_default().push(event.line);
    }
    groups
}

fn to_details(groups: &BTreeMap<String, Vec<usize>>) -> Vec<Detail> {
    groups
        .iter()
        .map(|(label, lines)| Detail {
            label: label.clone(),
            count: lines.len(),
            lines: lines.clone(),
        })
        .collect()
}

pub fn mutation_budget(f: &FnFacts, cfg: &MutationBudgetConfig) -> Vec<Diagnostic> {
    let groups = mutation_groups(f, cfg.granularity);
    let over: BTreeMap<_, _> = groups
        .iter()
        .filter(|(_, lines)| lines.len() > cfg.max_mutations_per_target)
        .map(|(label, lines)| (label.clone(), lines.clone()))
        .collect();
    let mut out = Vec::new();
    if !over.is_empty() {
        let actual = over.values().map(Vec::len).max().unwrap_or(0);
        let measure = Measure {
            actual,
            limit: cfg.max_mutations_per_target,
        };
        let message = format!("function `{}` mutates one target too many times", f.name);
        out.push(Diagnostic {
            details: to_details(&over),
            ..diagnostic(f, "mutation_per_target", message, measure)
        });
    }
    if groups.len() > cfg.max_mutated_targets {
        let measure = Measure {
            actual: groups.len(),
            limit: cfg.max_mutated_targets,
        };
        let message = format!("function `{}` mutates too many distinct targets", f.name);
        out.push(Diagnostic {
            details: to_details(&groups),
            ..diagnostic(f, "mutated_targets", message, measure)
        });
    }
    out
}

/// One weighted thing that happens in a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub kind: EffectKind,
    pub label: String,
    pub line: usize,
    /// Set when inherited from a callee (the immediate callee's name).
    pub via: Option<String>,
}

/// All effects of a function: mutations, classified calls, and driver-decided effects.
pub fn effects_of(f: &FnFacts, cfg: &Config) -> Vec<Effect> {
    // One effect per written target: repeated writes are `mutation_per_target`'s business.
    let mut written = std::collections::BTreeSet::new();
    let mutations = f
        .mutations
        .iter()
        .filter(|m| m.escapes)
        .filter(|m| written.insert((&m.root, &m.field_path)))
        .map(|m| Effect {
            kind: EffectKind::Mutation,
            label: if m.field_path.is_empty() {
                m.root.clone()
            } else {
                format!("{}.{}", m.root, m.field_path)
            },
            line: m.line,
            via: None,
        });
    let calls = f.calls.iter().filter_map(|c| {
        let classified = cfg.side_effects.paths.classify(&c.callee);
        let kind = classified.or(c.dynamic.then_some(EffectKind::Unknown))?;
        Some(Effect {
            kind,
            label: c.callee.clone(),
            line: c.line,
            via: None,
        })
    });
    let intrinsic = f.effects.iter().map(|e| Effect {
        kind: e.kind,
        label: e.detail.clone(),
        line: e.line,
        via: None,
    });
    mutations.chain(calls).chain(intrinsic).collect()
}

/// Weighted effect sum per statement, in thousandths (integer math only).
pub fn density_milli(f: &FnFacts, effects: &[Effect], cfg: &Config) -> usize {
    let total: usize = effects
        .iter()
        .map(|e| cfg.side_effects.weights.of(e.kind))
        .sum();
    1000 * total / f.statements.max(1)
}

fn effect_details(effects: &[Effect], cfg: &Config) -> Vec<Detail> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for e in effects
        .iter()
        .filter(|e| cfg.side_effects.weights.of(e.kind) > 0)
    {
        let via = e
            .via
            .as_ref()
            .map_or(String::new(), |v| format!(" (via {v})"));
        groups
            .entry(format!("{}: {}{via}", e.kind.name(), e.label))
            .or_default()
            .push(e.line);
    }
    to_details(&groups)
}

/// `effects` is the function's full effect list (direct plus inherited, see `graph`).
pub fn side_effect_density(f: &FnFacts, effects: &[Effect], cfg: &Config) -> Vec<Diagnostic> {
    let actual = density_milli(f, effects, cfg);
    let limit = cfg.side_effects.max_density_milli;
    let small = f.statements < cfg.side_effects.min_statements
        || code_lines(&f.source) < cfg.side_effects.min_lines;
    if small || actual <= limit {
        return Vec::new();
    }
    let message = format!("function `{}` has a high side-effect density", f.name);
    let details = effect_details(effects, cfg);
    vec![Diagnostic {
        details,
        ..diagnostic(f, "side_effect_density", message, Measure { actual, limit })
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::{CallEvent, EffectEvent, MutationEvent, sample};

    fn mutation(root: &str, field: &str, line: usize) -> MutationEvent {
        MutationEvent {
            root: root.into(),
            field_path: field.into(),
            line,
            escapes: true,
            delegated: false,
        }
    }

    fn density_diags(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
        side_effect_density(f, &effects_of(f, cfg), cfg)
    }

    fn body_of(lines: usize) -> String {
        format!("fn f() {{\n{}}}", "x();\n".repeat(lines))
    }

    #[test]
    fn length_boundary() {
        let mut f = sample("f");
        f.source = body_of(48); // 50 code lines
        assert!(fn_length(&f, &Config::default()).is_empty());
        f.source = body_of(49);
        assert_eq!(fn_length(&f, &Config::default())[0].actual, 51);
    }

    #[test]
    fn spacing_boundary_and_blank_line() {
        let mut f = sample("f");
        let cfg = Config::default();
        f.source = body_of(3); // 5 code lines: below 6
        assert!(fn_spacing(&f, &cfg).is_empty());
        f.source = body_of(4); // 6 code lines: checked
        let out = fn_spacing(&f, &cfg);
        assert_eq!((out[0].actual, out[0].limit), (6, 5));
        f.source = "fn f() {\nx();\nx();\nx();\n\nx();\n}".into();
        assert!(fn_spacing(&f, &cfg).is_empty());
    }

    #[test]
    fn spacing_zero_disables() {
        let mut f = sample("f");
        f.source = body_of(20);
        let mut cfg = Config::default();
        cfg.fn_spacing.min_lines = 0;
        assert!(fn_spacing(&f, &cfg).is_empty());
        cfg.fn_spacing.min_lines = 10;
        assert_eq!(fn_spacing(&f, &cfg).len(), 1);
    }

    #[test]
    fn params_boundary_and_self() {
        let mut f = sample("f");
        f.params = 4;
        f.has_self = true;
        let mut cfg = Config::default();
        assert!(fn_params(&f, &cfg).is_empty());
        cfg.fn_params.count_self = true;
        assert_eq!(fn_params(&f, &cfg)[0].actual, 5);
        f.params = 5;
        cfg.fn_params.count_self = false;
        assert_eq!(fn_params(&f, &cfg)[0].actual, 5);
    }

    #[test]
    fn four_mutations_of_one_root_are_flagged() {
        let mut f = sample("f");
        f.mutations = (0..4)
            .map(|i| mutation("self", &format!("f{i}"), i + 1))
            .collect();
        let out = mutation_budget(&f, &MutationBudgetConfig::default());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id, "mutation_per_target");
        assert_eq!(
            (out[0].actual, out[0].details[0].label.as_str()),
            (4, "self")
        );
    }

    #[test]
    fn field_granularity_splits_targets() {
        let mut f = sample("f");
        f.mutations = (0..4)
            .map(|i| mutation("self", &format!("f{i}"), i + 1))
            .collect();
        let cfg = MutationBudgetConfig {
            granularity: Granularity::Field,
            max_mutated_targets: 4,
            ..MutationBudgetConfig::default()
        };
        assert!(mutation_budget(&f, &cfg).is_empty());
    }

    #[test]
    fn four_distinct_roots_are_flagged() {
        let mut f = sample("f");
        f.mutations = ["a", "b", "c", "d"]
            .iter()
            .map(|r| mutation(r, "", 1))
            .collect();
        let out = mutation_budget(&f, &MutationBudgetConfig::default());
        assert_eq!(out[0].rule_id, "mutated_targets");
        assert_eq!(out[0].actual, 4);
    }

    fn call(callee: &str) -> CallEvent {
        CallEvent {
            callee: callee.into(),
            line: 1,
            dynamic: false,
        }
    }

    #[test]
    fn unclassified_dynamic_calls_are_unknown_effects() {
        let mut f = sample("f");
        f.calls = vec![CallEvent {
            dynamic: true,
            ..call("<fn pointer>")
        }];
        let effects = effects_of(&f, &Config::default());
        assert_eq!(effects[0].kind, EffectKind::Unknown);
        // A dynamic call that still matches a known path keeps that classification.
        f.calls[0].callee = "std::io::Write::write".into();
        let effects = effects_of(&f, &Config::default());
        assert_eq!(effects[0].kind, EffectKind::Io);
    }

    #[test]
    fn density_uses_integer_weights() {
        let mut f = sample("f");
        f.statements = 4;
        f.calls = vec![
            call("std::fs::write"),
            call("std::io::_print"),
            call("alloc::vec::Vec::len"),
        ];
        let cfg = Config::default();
        assert_eq!(density_milli(&f, &effects_of(&f, &cfg), &cfg), 1500);
    }

    #[test]
    fn density_counts_mutations_and_intrinsics() {
        let mut f = sample("f");
        f.statements = 2;
        f.mutations = vec![mutation("v", "", 1)];
        f.effects = vec![EffectEvent {
            kind: EffectKind::Unsafe,
            detail: "unsafe block".into(),
            line: 2,
        }];
        let cfg = Config::default();
        assert_eq!(density_milli(&f, &effects_of(&f, &cfg), &cfg), 2500);
    }

    #[test]
    fn local_mutations_count_for_budget_but_not_density() {
        let mut f = sample("f");
        f.statements = 4;
        f.mutations = (1..=4)
            .map(|line| MutationEvent {
                escapes: false,
                ..mutation("acc", "", line)
            })
            .collect();
        let cfg = Config::default();
        assert_eq!(density_milli(&f, &effects_of(&f, &cfg), &cfg), 0);
        let budget = mutation_budget(&f, &MutationBudgetConfig::default());
        assert_eq!(budget[0].rule_id, "mutation_per_target");
    }

    #[test]
    fn small_functions_are_never_flagged() {
        let mut f = sample("f");
        f.statements = 2;
        f.calls = vec![call("std::fs::write")];
        assert!(density_diags(&f, &Config::default()).is_empty());
    }

    #[test]
    fn zero_statements_does_not_divide_by_zero() {
        let mut f = sample("f");
        f.statements = 0;
        let cfg = Config::default();
        assert_eq!(density_milli(&f, &[], &cfg), 0);
    }

    #[test]
    fn custom_weight_changes_score_and_details() {
        let mut f = sample("f");
        f.statements = 4;
        f.calls = vec![call("std::fs::write")];
        let mut cfg = Config::default();
        cfg.side_effects.min_lines = 0; // `sample` source is tiny
        let out = density_diags(&f, &cfg); // 3 * 1000 / 4 = 750 > 500
        assert_eq!(out[0].actual, 750);
        assert_eq!(out[0].details[0].label, "io: std::fs::write");
        cfg.side_effects.weights.io = 1; // 250
        assert!(density_diags(&f, &cfg).is_empty());
    }

    #[test]
    fn small_functions_skip_density_until_min_lines_lowered() {
        let mut f = sample("f");
        f.statements = 4;
        f.calls = vec![call("std::fs::write")];
        f.source = body_of(7); // 9 code lines < 10
        let mut cfg = Config::default();
        assert!(density_diags(&f, &cfg).is_empty());
        f.source = body_of(8); // 10 code lines: exactly at the threshold is checked
        assert_eq!(density_diags(&f, &cfg).len(), 1);
        f.source = body_of(7);
        cfg.side_effects.min_lines = 0;
        assert_eq!(density_diags(&f, &cfg).len(), 1);
    }

    #[test]
    fn repeated_writes_to_one_target_count_once_for_density() {
        let mut f = sample("f");
        f.statements = 10;
        f.mutations = vec![
            mutation("out", "", 1),
            mutation("out", "", 2),
            mutation("out", "", 3),
        ];
        let effects = effects_of(&f, &Config::default());
        assert_eq!(effects.len(), 1);
        // The budget rule still sees all three writes.
        assert_eq!(mutation_groups(&f, Granularity::Root)["out"].len(), 3);
    }
}
