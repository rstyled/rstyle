use super::{Outcome, mutations};
use crate::clones::Args;
use rstyle_core::config::{Granularity, MutationBudgetConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(in crate::clones) struct Settings {
    max_mutations_per_target: usize,
    max_mutated_targets: usize,
    granularity: &'static str,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Configuration {
    mutation_budget: MutationBudgetConfig,
}

pub(super) fn configuration(args: &Args) -> Result<Option<Settings>, String> {
    if !args.mutation_rules {
        return Ok(None);
    }
    let policy = match &args.mutation_config {
        None => Configuration::default(),
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?
        }
    }
    .mutation_budget;
    if !matches!(policy.granularity, Granularity::Root) {
        return Err("Mutation analysis currently supports root granularity only".into());
    }
    Ok(Some(Settings {
        max_mutations_per_target: policy.max_mutations_per_target,
        max_mutated_targets: policy.max_mutated_targets,
        granularity: "root",
    }))
}

#[derive(Serialize)]
pub(super) struct Evidence {
    sites: Vec<mutations::WriteSite>,
    boundaries: Vec<mutations::Boundary>,
    write_sites: Count,
    mutated_roots: Count,
    escaped_write_sites: Count,
    max_sites_per_root: Count,
}

#[derive(Serialize)]
struct Count {
    kind: &'static str,
    value: usize,
}

#[derive(Serialize)]
pub(super) struct Rule {
    pub rule_id: &'static str,
    pub outcome: Outcome,
    pub actual: Option<usize>,
    pub limit: usize,
}

impl Evidence {
    pub(super) fn from_assessment(
        assessment: mutations::Assessment,
        complete: bool,
    ) -> Result<Self, String> {
        let counts = root_counts(&assessment.sites)?;
        let kind = if complete { "exact" } else { "lower_bound" };
        Ok(Self {
            write_sites: Count {
                kind,
                value: assessment.sites.len(),
            },
            escaped_write_sites: Count {
                kind,
                value: assessment.sites.iter().filter(|s| s.caller_visible).count(),
            },
            mutated_roots: Count {
                kind,
                value: counts.len(),
            },
            max_sites_per_root: Count {
                kind,
                value: counts.values().map(|(_, count)| *count).max().unwrap_or(0),
            },
            sites: assessment.sites,
            boundaries: assessment.boundaries,
        })
    }
}

type RootCounts<'a> = BTreeMap<(u32, u32, u32), (&'a mutations::Root, usize)>;

fn root_counts(sites: &[mutations::WriteSite]) -> Result<RootCounts<'_>, String> {
    let mut counts = BTreeMap::new();
    for site in sites {
        let root = &site.root;
        let key = (root.native.krate, root.native.owner, root.native.local);
        let (existing, count) = counts.entry(key).or_insert((root, 0));
        if *existing != root {
            return Err("Contradictory mutation roots share one native binding".into());
        }
        *count += 1;
    }
    Ok(counts)
}

impl Settings {
    pub(super) fn evaluate(&self, evidence: Option<&Evidence>, complete: bool) -> Vec<Rule> {
        vec![
            rule(
                "mutation_per_target",
                evidence.map(|e| e.max_sites_per_root.value),
                self.max_mutations_per_target,
                complete,
            ),
            rule(
                "mutated_targets",
                evidence.map(|e| e.mutated_roots.value),
                self.max_mutated_targets,
                complete,
            ),
        ]
    }
}

fn rule(rule_id: &'static str, actual: Option<usize>, limit: usize, complete: bool) -> Rule {
    let outcome = match actual {
        Some(value) if value > limit => Outcome::Finding,
        Some(_) if complete => Outcome::EvaluatedNoFinding,
        _ => Outcome::NotEvaluated,
    };
    Rule {
        rule_id,
        outcome,
        actual,
        limit,
    }
}

pub(super) fn outcome(rules: &[Rule]) -> Outcome {
    if rules.iter().any(|r| r.outcome == Outcome::Finding) {
        Outcome::Finding
    } else if rules.iter().any(|r| r.outcome == Outcome::NotEvaluated) {
        Outcome::NotEvaluated
    } else {
        Outcome::EvaluatedNoFinding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_bounds_only_decide_strict_threshold_breaches() {
        assert_eq!(
            rule("mutation_per_target", Some(3), 2, false).outcome,
            Outcome::Finding
        );
        assert_eq!(
            rule("mutation_per_target", Some(2), 2, false).outcome,
            Outcome::NotEvaluated
        );
        assert_eq!(
            rule("mutation_per_target", Some(2), 2, true).outcome,
            Outcome::EvaluatedNoFinding
        );
        assert_eq!(
            rule("mutation_per_target", None, 0, false).outcome,
            Outcome::NotEvaluated
        );
    }
    fn site(local: u32, declaration: u32) -> mutations::WriteSite {
        mutations::WriteSite {
            span: rstyle_pdg::ByteSpan { start: 40, end: 45 },
            root: mutations::Root {
                declaration_span: rstyle_pdg::ByteSpan {
                    start: declaration,
                    end: declaration + 1,
                },
                scope_span: rstyle_pdg::ByteSpan { start: 0, end: 60 },
                native: rstyle_pdg::HirRef {
                    krate: 0,
                    owner: 1,
                    local,
                },
                formal: None,
                provenance: mutations::Provenance::LocalOwned,
            },
            operation: mutations::Operation::Assignment,
            caller_visible: false,
        }
    }

    #[test]
    fn roots_remain_distinct_and_contradictory_native_associations_fail() {
        let sites = [site(1, 20), site(1, 20), site(2, 30)];
        let roots = root_counts(&sites).unwrap();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[&(0, 1, 1)].1, 2);
        assert!(root_counts(&[site(1, 20), site(1, 30)]).is_err());
    }

    #[test]
    fn policy_rejects_unknown_fields_instead_of_silently_changing_thresholds() {
        assert!(
            toml::from_str::<Configuration>("[mutation_budget]\nmax_mutatons_per_target=2")
                .is_err()
        );
        assert!(toml::from_str::<Configuration>("max_mutations_per_target=2").is_err());
    }
}
