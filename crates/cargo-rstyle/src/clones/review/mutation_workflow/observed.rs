//! Live adapters use completed counts and checked original owner identities.
use super::super::{
    Locator, local, local_adapter, mutation_rules, owner_observations, owner_population, workflow,
};
use crate::clones::{FunctionKey, analysis::mutation_reports, report};
use std::collections::BTreeMap;

pub(super) use mutation_reports::RuleReview as Rule;
type Identity = (Locator, local::BuildContext);

pub(super) struct Observations {
    pub rows: Vec<Row>,
    pub truncated: bool,
    pub unavailable_scope: bool,
    pub requested: bool,
    complete: bool,
}

pub(super) struct Row {
    pub key: FunctionKey,
    pub selected: bool,
    rules: Vec<Rule>,
    unique: bool,
    witness: Result<Witness, String>,
}

enum Witness {
    Identity(Box<Identity>),
    Findings(Vec<mutation_rules::Snapshot>),
}

impl Row {
    fn new(
        source: mutation_reports::Review<'_>,
        selected: bool,
        identity: Result<Identity, String>,
    ) -> Self {
        let witness = identity.map(|identity| {
            let snapshots: Vec<_> = source
                .rules
                .iter()
                .filter(|rule| rule.visible && rule.complete)
                .map(|rule| mutation_rules::Snapshot {
                    versions: local_adapter::versions(rule.policy.rule_id()),
                    locator: identity.0.clone(),
                    context: identity.1.clone(),
                    policy: rule.policy.clone(),
                    measurement: mutation_rules::Measurement {
                        sites_per_root: source.sites_per_root.to_vec(),
                    },
                })
                .collect();
            if snapshots.is_empty() {
                Witness::Identity(Box::new(identity))
            } else {
                Witness::Findings(snapshots)
            }
        });
        Self {
            key: source.key,
            selected,
            rules: source.rules,
            unique: true,
            witness,
        }
    }

    pub fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        match self.witness.as_ref().map_err(String::as_str)? {
            Witness::Identity(identity) => Ok((&identity.0, &identity.1)),
            Witness::Findings(snapshots) => Ok((&snapshots[0].locator, &snapshots[0].context)),
        }
    }

    pub fn rule(&self, rule_id: &str) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|rule| rule.policy.rule_id() == rule_id)
    }

    pub fn snapshot(&self, rule_id: &str) -> Result<&mutation_rules::Snapshot, String> {
        let witness = self.witness.as_ref().map_err(Clone::clone)?;
        if let Witness::Findings(snapshots) = witness
            && let Some(snapshot) = snapshots
                .iter()
                .find(|snapshot| snapshot.policy.rule_id() == rule_id)
        {
            return Ok(snapshot);
        }
        Err("Owner has no completed visible mutation finding".into())
    }

    pub fn checked(&self, rule_id: &str) -> Result<mutation_rules::LiveFinding<'_>, String> {
        let rule = self.rule(rule_id).ok_or("Mutation rule was not assessed")?;
        if !rule.complete || !rule.visible || !self.unique {
            return Err("Mutation assessment or original owner uniqueness is unavailable".into());
        }
        mutation_rules::LiveFinding::checked(self.snapshot(rule_id)?)
            .map_err(|error| format!("{error:?}"))
    }

    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter()
    }
}

impl Observations {
    pub fn complete(&self) -> bool {
        self.complete
            && !self.truncated
            && !self.unavailable_scope
            && self.rows.iter().all(|row| {
                row.rules()
                    .all(|rule| !rule.visible || row.checked(rule.policy.rule_id()).is_ok())
            })
    }
}

pub(super) fn collect(
    inputs: &workflow::Inputs<'_, '_>,
    ctx: &report::Context<'_>,
) -> Observations {
    let selected: BTreeMap<_, _> = inputs.analysis.review_selection().collect();
    let collected = owner_observations::collect(
        inputs.mutation.reviews().map(|source| (source.key, source)),
        inputs,
        ctx,
        |source, identity| {
            let selection = selected
                .get(&source.key)
                .copied()
                .unwrap_or(ctx.args.changed_files.is_none());
            Row::new(source, selection, identity)
        },
    );
    Observations {
        rows: collected.rows,
        truncated: collected.truncated,
        requested: ctx.args.mutation_rules,
        unavailable_scope: collected.unavailable_scope,
        complete: inputs.mutation.review_scope_complete(),
    }
}

impl owner_population::Owner for Row {
    fn identity(&self) -> Result<(&Locator, &local::BuildContext), &str> {
        Row::identity(self)
    }
}

impl owner_observations::Observation for Row {
    fn key(&self) -> FunctionKey {
        self.key
    }

    fn set_unique(&mut self, unique: bool) {
        self.unique = unique;
    }
}
