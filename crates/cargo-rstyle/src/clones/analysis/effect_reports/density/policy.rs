//! Numeric policy applied only to complete effect and source inventories.
use crate::clones::analysis::Outcome;
use rstyle_core::{config::SideEffectsConfig, facts::EffectKind};
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize)]
pub(super) struct Measurement {
    pub(super) outcome: Outcome,
    pub(super) actual: Option<usize>,
    pub(super) limit: usize,
    pub(super) exempt: bool,
    pub(super) reason: Option<&'static str>,
}

pub(super) struct SourceSize {
    pub statements: Option<usize>,
    pub code_lines: Option<usize>,
}

pub(super) fn evaluate(
    config: &SideEffectsConfig,
    counts: &BTreeMap<EffectKind, usize>,
    source: SourceSize,
    complete: bool,
) -> Result<Measurement, String> {
    let unavailable = |reason| Measurement {
        outcome: Outcome::NotEvaluated,
        actual: None,
        limit: config.max_density_milli,
        exempt: false,
        reason: Some(reason),
    };
    let (Some(statements), Some(code_lines)) = (source.statements, source.code_lines) else {
        return Ok(unavailable(
            "Complete effect and source inventories are required",
        ));
    };
    if !complete || counts.get(&EffectKind::Unknown).copied().unwrap_or(0) != 0 {
        return Ok(unavailable("Complete known effect events are required"));
    }

    if statements < config.min_statements || code_lines < config.min_lines {
        return Ok(Measurement {
            outcome: Outcome::EvaluatedNoFinding,
            actual: None,
            limit: config.max_density_milli,
            exempt: true,
            reason: Some("Complete function is below a configured density minimum"),
        });
    }
    let actual = density(config, counts, statements)?;
    Ok(Measurement {
        actual: Some(actual),
        limit: config.max_density_milli,
        outcome: if actual > config.max_density_milli {
            Outcome::Finding
        } else {
            Outcome::EvaluatedNoFinding
        },
        exempt: false,
        reason: None,
    })
}

fn density(
    config: &SideEffectsConfig,
    counts: &BTreeMap<EffectKind, usize>,
    statements: usize,
) -> Result<usize, String> {
    let weighted = counts.iter().try_fold(0usize, |total, (kind, count)| {
        count
            .checked_mul(config.weights.of(*kind))
            .and_then(|value| total.checked_add(value))
            .ok_or("Effect density weighted event count overflowed")
    })?;

    Ok(weighted
        .checked_mul(1000)
        .ok_or("Effect density milli scaling overflowed")?
        / statements.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SideEffectsConfig {
        SideEffectsConfig {
            min_statements: 0,
            min_lines: 0,
            ..SideEffectsConfig::default()
        }
    }

    #[test]
    fn exact_boundary_and_independent_weighted_density_are_strict() {
        let config = policy();
        let counts = BTreeMap::from([(EffectKind::Mutation, 1)]);
        let boundary = evaluate(
            &config,
            &counts,
            SourceSize {
                statements: Some(2),
                code_lines: Some(10),
            },
            true,
        )
        .unwrap();
        assert_eq!(boundary.actual, Some(500));
        assert_eq!(boundary.outcome, Outcome::EvaluatedNoFinding);
        assert!(!boundary.exempt);
        let above = evaluate(
            &config,
            &counts,
            SourceSize {
                statements: Some(1),
                code_lines: Some(10),
            },
            true,
        )
        .unwrap();
        assert_eq!(above.actual, Some(1000));
        assert_eq!(above.outcome, Outcome::Finding);
        let mixed = BTreeMap::from([(EffectKind::Mutation, 2), (EffectKind::Io, 1)]);
        let measured = evaluate(
            &config,
            &mixed,
            SourceSize {
                statements: Some(3),
                code_lines: Some(10),
            },
            true,
        )
        .unwrap();
        assert_eq!(measured.actual, Some(1666));
        assert_eq!(measured.outcome, Outcome::Finding);
    }

    #[test]
    fn all_arithmetic_overflow_stages_return_errors() {
        let mut config = policy();
        config.weights.mutation = 2;
        let term = BTreeMap::from([(EffectKind::Mutation, usize::MAX)]);
        assert!(
            evaluate(
                &config,
                &term,
                SourceSize {
                    statements: Some(1),
                    code_lines: Some(10)
                },
                true
            )
            .is_err()
        );
        config.weights.mutation = 1;
        config.weights.io = 1;
        let sum = BTreeMap::from([(EffectKind::Mutation, usize::MAX), (EffectKind::Io, 1)]);
        assert!(
            evaluate(
                &config,
                &sum,
                SourceSize {
                    statements: Some(1),
                    code_lines: Some(10)
                },
                true
            )
            .is_err()
        );
        let scale = BTreeMap::from([(EffectKind::Mutation, usize::MAX / 1000 + 1)]);
        assert!(
            evaluate(
                &config,
                &scale,
                SourceSize {
                    statements: Some(usize::MAX),
                    code_lines: Some(10)
                },
                true
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_zero_weight_and_incomplete_inputs_cannot_authorize_exemptions() {
        let mut config = SideEffectsConfig::default();
        config.weights.unknown = 0;
        let unknown = BTreeMap::from([(EffectKind::Unknown, 1)]);
        let result = evaluate(
            &config,
            &unknown,
            SourceSize {
                statements: Some(0),
                code_lines: Some(0),
            },
            true,
        )
        .unwrap();
        assert_eq!(result.outcome, Outcome::NotEvaluated);
        assert_eq!(result.actual, None);
        assert!(!result.exempt);
        let empty = BTreeMap::new();
        for (statements, lines, complete) in [
            (Some(0), Some(0), false),
            (None, Some(0), true),
            (Some(0), None, true),
        ] {
            let result = evaluate(
                &config,
                &empty,
                SourceSize {
                    statements,
                    code_lines: lines,
                },
                complete,
            )
            .unwrap();
            assert_eq!(result.outcome, Outcome::NotEvaluated);
            assert_eq!(result.actual, None);
            assert!(!result.exempt);
        }
    }

    #[test]
    fn complete_minimum_exemptions_use_or_and_skip_unneeded_arithmetic() {
        let config = SideEffectsConfig::default();
        let huge = BTreeMap::from([(EffectKind::Io, usize::MAX)]);
        for (statements, lines) in [(2, 10), (3, 9)] {
            let result = evaluate(
                &config,
                &huge,
                SourceSize {
                    statements: Some(statements),
                    code_lines: Some(lines),
                },
                true,
            )
            .unwrap();
            assert_eq!(result.outcome, Outcome::EvaluatedNoFinding);
            assert_eq!(result.actual, None);
            assert!(result.exempt);
        }
        let one = BTreeMap::from([(EffectKind::Mutation, 1)]);
        let at_minimum = evaluate(
            &config,
            &one,
            SourceSize {
                statements: Some(3),
                code_lines: Some(10),
            },
            true,
        )
        .unwrap();
        assert!(!at_minimum.exempt);
        assert_eq!(at_minimum.actual, Some(333));
        let zero = evaluate(
            &policy(),
            &one,
            SourceSize {
                statements: Some(0),
                code_lines: Some(0),
            },
            true,
        )
        .unwrap();
        assert_eq!(zero.actual, Some(1000));
    }
}
