//! Pure rules over `FnFacts`. No IO, no compiler types.

use crate::config::{Config, Granularity, MutationBudgetConfig};
use crate::facts::{CallEvent, EffectEvent, EffectKind, FnFacts, LibraryClass, MutationEvent};
use crate::lines::code_lines;
use crate::report::{Detail, Diagnostic};
use std::collections::BTreeMap;

/// The measured value and its limit, kept together so rules can't swap them.
#[derive(Debug, Clone, Copy)]
pub struct Measure {
    pub actual: usize,
    pub limit: usize,
}

pub(crate) fn diagnostic(
    f: &FnFacts,
    rule_id: &str,
    message: String,
    measure: Measure,
) -> Diagnostic {
    Diagnostic {
        path: f.path.clone(),
        line: f.line,
        column: f.column,
        rule_id: rule_id.into(),
        message,
        actual: measure.actual,
        limit: measure.limit,
        details: vec![],
        source: f.whole_lines(),
    }
}

pub fn fn_length(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    fn_length_with_lines(f, cfg, code_lines(&f.source))
}

pub(crate) fn fn_length_with_lines(f: &FnFacts, cfg: &Config, actual: usize) -> Vec<Diagnostic> {
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

pub fn complexity(f: &FnFacts, cfg: &Config) -> Vec<Diagnostic> {
    let Some(score) = &f.complexity else {
        return vec![];
    };
    [
        (
            "cyclomatic_complexity",
            score.cyclomatic,
            cfg.complexity.max_cyclomatic,
        ),
        (
            "cognitive_complexity",
            score.cognitive,
            cfg.complexity.max_cognitive,
        ),
    ]
    .into_iter()
    .filter_map(|(rule, actual, limit)| {
        if actual <= limit {
            return None;
        }
        let mut finding = diagnostic(
            f,
            rule,
            format!("function `{}` has high {}", f.name, rule.replace('_', " ")),
            Measure { actual, limit },
        );
        finding.details = score
            .contributions
            .iter()
            .filter_map(|point| {
                let count = if rule == "cyclomatic_complexity" {
                    point.cyclomatic
                } else {
                    point.cognitive
                };
                (count > 0).then(|| Detail {
                    label: format!("{:?} (nesting {})", point.kind, point.nesting),
                    count,
                    lines: vec![point.line],
                })
            })
            .collect();
        Some(finding)
    })
    .collect()
}

fn mutation_groups(f: &FnFacts, granularity: Granularity) -> BTreeMap<String, Vec<usize>> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for event in &f.mutations {
        if event.origin == crate::facts::MutationOrigin::ForDesugaring {
            continue;
        }
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
    effects_from_events(
        EffectEvents {
            mutations: &f.mutations,
            calls: &f.calls,
            effects: &f.effects,
        },
        cfg,
    )
}

/// Compiler-observed events, independent of authored source and numeric measurements.
pub struct EffectEvents<'a> {
    pub mutations: &'a [MutationEvent],
    pub calls: &'a [CallEvent],
    pub effects: &'a [EffectEvent],
}

/// Reuse effect classification over borrowed events without inventing function metrics.
pub fn effects_from_events(events: EffectEvents<'_>, cfg: &Config) -> Vec<Effect> {
    // One effect per written target: repeated writes are `mutation_per_target`'s business.
    let mut written = std::collections::BTreeSet::new();
    let mutations = events
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
    let calls = events.calls.iter().filter_map(|c| {
        let kind = call_effect(c, cfg)?;
        Some(Effect {
            kind,
            label: c.callee.clone(),
            line: c.line,
            via: None,
        })
    });
    let intrinsic = events.effects.iter().map(|e| Effect {
        kind: e.kind,
        label: e.detail.clone(),
        line: e.line,
        via: None,
    });

    mutations.chain(calls).chain(intrinsic).collect()
}

/// A call's effect: the config's say first, as the user's own, then the library contract's.
/// A call neither classifies is an unknown effect only when its target is unknown.
fn call_effect(call: &CallEvent, cfg: &Config) -> Option<EffectKind> {
    if let Some(kind) = cfg.side_effects.paths.classify(&call.callee) {
        return Some(kind);
    }
    match call.library {
        Some(LibraryClass::Pure) => None,
        Some(LibraryClass::Effect(kind)) => Some(kind),
        None => call.dynamic.then_some(EffectKind::Unknown),
    }
}

/// The callee of a library call nothing says the effect of, neither the config nor a contract, so
/// what it does can't be told from here. Density is right not to count one: nobody classified it.
/// A comparison can't call it clean, though. A call with an unknown target is its own matter.
pub fn unclassified_callee<'c>(call: &'c CallEvent, cfg: &Config) -> Option<&'c str> {
    let callee = call.library_callee.as_deref().filter(|_| !call.dynamic)?;
    let classified = match call_effect(call, cfg) {
        Some(kind) => kind != EffectKind::Unknown,
        None => call.library.is_some(),
    };
    (!classified).then_some(callee)
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
    side_effect_density_with_lines(f, effects, cfg, || code_lines(&f.source))
}

pub(crate) fn side_effect_density_with_lines(
    f: &FnFacts,
    effects: &[Effect],
    cfg: &Config,
    source_lines: impl FnOnce() -> usize,
) -> Vec<Diagnostic> {
    let actual = density_milli(f, effects, cfg);
    let limit = cfg.side_effects.max_density_milli;
    let small = f.statements < cfg.side_effects.min_statements
        || source_lines() < cfg.side_effects.min_lines;

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
            origin: crate::facts::MutationOrigin::Source,
            root: root.into(),
            field_path: field.into(),
            line,
            escapes: true,
            delegated: false,
        }
    }

    #[test]
    fn borrowed_events_preserve_written_targets_and_exclude_owned_or_delegated_writes() {
        let mutations = [
            mutation("out", "first", 1),
            mutation("out", "first", 2),
            mutation("out", "second", 3),
            MutationEvent {
                escapes: false,
                ..mutation("local", "", 4)
            },
            MutationEvent {
                escapes: false,
                delegated: true,
                ..mutation("self", "", 5)
            },
        ];
        let effects = effects_from_events(
            EffectEvents {
                mutations: &mutations,
                calls: &[],
                effects: &[],
            },
            &Config::default(),
        );
        let observed: Vec<_> = effects
            .iter()
            .map(|e| (e.kind, e.label.as_str(), e.line))
            .collect();
        assert_eq!(
            observed,
            [
                (EffectKind::Mutation, "out.first", 1),
                (EffectKind::Mutation, "out.second", 3),
            ]
        );
        assert!(effects.iter().all(|effect| effect.via.is_none()));
    }

    #[test]
    fn borrowed_events_preserve_call_classification_intrinsics_and_wrapper_results() {
        let mut facts = sample("f");
        facts.calls = vec![
            call("std::fs::write"),
            CallEvent {
                callee: "<fn pointer>".into(),
                line: 8,
                dynamic: true,
                library: None,
                library_callee: None,
            },
            call("unclassified::ordinary"),
            CallEvent {
                dynamic: true,
                ..call("std::fs::read")
            },
        ];
        facts.effects = vec![EffectEvent {
            kind: EffectKind::Global,
            detail: "static mut STATE".into(),
            line: 9,
        }];
        let cfg = Config::default();
        let effects = effects_from_events(
            EffectEvents {
                mutations: &facts.mutations,
                calls: &facts.calls,
                effects: &facts.effects,
            },
            &cfg,
        );
        assert_eq!(effects, effects_of(&facts, &cfg));
        let observed: Vec<_> = effects.iter().map(|e| (e.kind, e.label.as_str())).collect();
        assert_eq!(
            observed,
            [
                (EffectKind::Io, "std::fs::write"),
                (EffectKind::Unknown, "<fn pointer>"),
                (EffectKind::Io, "std::fs::read"),
                (EffectKind::Global, "static mut STATE"),
            ]
        );
        assert_eq!((effects[1].line, effects[3].line), (8, 9));
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
    fn lowered_iterators_do_not_inflate_source_budgets_or_erase_escaping_effects() {
        let mut f = sample("f");
        let mut source = mutation("iter", "", 1);
        source.escapes = false;
        f.mutations.push(source);
        for line in 2..6 {
            let mut generated = mutation("iter", "", line);
            generated.origin = crate::facts::MutationOrigin::ForDesugaring;
            f.mutations.push(generated);
        }
        let cfg = MutationBudgetConfig {
            max_mutated_targets: 0,
            max_mutations_per_target: 0,
            ..MutationBudgetConfig::default()
        };
        let out = mutation_budget(&f, &cfg);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|diagnostic| diagnostic.actual == 1));
        assert!(
            out.iter()
                .all(|diagnostic| diagnostic.details[0].count == 1)
        );
        let effects = effects_of(&f, &Config::default());
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].kind, EffectKind::Mutation);
        assert_eq!(effects[0].line, 2);
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

    #[test]
    fn a_library_call_nothing_classifies_is_named_and_any_other_is_not() {
        let cfg = Config::default();
        let library = |library| CallEvent {
            callee: "lib::call".into(),
            line: 1,
            dynamic: false,
            library,
            library_callee: Some("lib::call#1".into()),
        };

        let unlisted = library(None);
        assert_eq!(unclassified_callee(&unlisted, &cfg), Some("lib::call#1"));
        let unknown = library(Some(LibraryClass::Effect(EffectKind::Unknown)));
        assert_eq!(unclassified_callee(&unknown, &cfg), Some("lib::call#1"));

        assert_eq!(
            unclassified_callee(&library(Some(LibraryClass::Pure)), &cfg),
            None
        );
        let io = library(Some(LibraryClass::Effect(EffectKind::Io)));
        assert_eq!(unclassified_callee(&io, &cfg), None);

        let dynamic = CallEvent {
            dynamic: true,
            ..library(None)
        };
        assert_eq!(unclassified_callee(&dynamic, &cfg), None, "its own matter");

        let workspace = CallEvent {
            library_callee: None,
            ..library(None)
        };
        assert_eq!(unclassified_callee(&workspace, &cfg), None, "readable here");
    }

    #[test]
    fn the_config_classifying_a_library_call_is_enough() {
        let mut cfg = Config::default();
        let write = CallEvent {
            callee: "tokio::fs::write".into(),
            line: 1,
            dynamic: false,
            library: None,
            library_callee: Some("tokio::fs::write".into()),
        };
        assert_eq!(unclassified_callee(&write, &cfg), Some("tokio::fs::write"));

        cfg.side_effects.paths.io.push("tokio::fs::".into());
        assert_eq!(unclassified_callee(&write, &cfg), None);
    }

    fn call(callee: &str) -> CallEvent {
        CallEvent {
            callee: callee.into(),
            line: 1,
            dynamic: false,
            library: None,
            library_callee: None,
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
    fn a_library_class_decides_a_call_unless_the_config_names_it() {
        let classified = |library| CallEvent {
            dynamic: true,
            library,
            ..call("example.com/lib.Do")
        };
        let mut f = sample("f");
        let mut config = Config::default();

        f.calls = vec![classified(Some(LibraryClass::Effect(EffectKind::Io)))];
        let effects = effects_of(&f, &config);
        assert_eq!(effects.len(), 1, "a class is one effect, not one more");
        assert_eq!(effects[0].kind, EffectKind::Io);

        f.calls = vec![classified(Some(LibraryClass::Pure))];
        assert!(
            effects_of(&f, &config).is_empty(),
            "pure wins over a dynamic target"
        );

        config.side_effects.paths.network = vec!["example.com/lib.".into()];
        for class in [
            LibraryClass::Pure,
            LibraryClass::Effect(EffectKind::Unknown),
        ] {
            f.calls = vec![classified(Some(class))];
            let effects = effects_of(&f, &config);
            assert_eq!(effects.len(), 1, "the config replaces the class");
            assert_eq!(effects[0].kind, EffectKind::Network);
        }
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
                origin: crate::facts::MutationOrigin::Source,
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
