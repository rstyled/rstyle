//! Numeric negatives reuse completed core scopes; suppressed/excluded inputs never become zero facts.
use super::super::{
    local_adapter, numeric_workflow,
    owner_population::{self, Domain, Owner},
    status,
};
use super::{Snapshot, observed::Assessed};
use rstyle_core::suppress::Suppression;
use std::rc::Rc;

pub(in crate::clones) struct Row<'a> {
    pub assessed: &'a Assessed,
    pub unique: bool,
    pub snapshots: &'a [Result<Snapshot, String>],
    pub policy: &'a Rc<serde_json::Value>,
    pub metrics: &'static str,
    pub selected: bool,
    pub effect_scope_complete: bool,
    pub request: Option<&'a crate::clones::diagnostic_selection::Request>,
}

pub(in crate::clones::review) fn observations<'a>(
    advisory: &'a crate::clones::advisory::Report,
    ctx: &crate::clones::report::Context<'_>,
) -> super::super::owner_observations::Observations<Row<'a>> {
    let truncated = advisory.diagnostic_owner_scope_truncated();
    super::super::owner_observations::Observations {
        rows: advisory.diagnostic_owner_reviews(ctx),
        requested: ctx.args.compiler_advisory,
        // The complete name census establishes uniqueness for retained rows.
        // Omitted payloads remain unknown through unavailable_scope.
        truncated: false,
        unavailable_scope: truncated || !advisory.advisory_inputs_complete(),
        complete: !truncated && advisory.advisory_inputs_complete(),
    }
}

impl Owner for Row<'_> {
    fn identity(
        &self,
    ) -> Result<(&super::super::Locator, &super::super::local::BuildContext), &str> {
        if !self.unique {
            return Err("Compiler diagnostic owner is ambiguous in this target");
        }
        self.assessed
            .owner
            .as_ref()
            .map(|owner| (&owner.locator, &owner.context))
            .map_err(String::as_str)
    }
}

impl numeric_workflow::Retained for Row<'_> {
    type Snapshot = Snapshot;
    const DOMAIN: Domain = Domain::Compiler;
    const NOT_REQUESTED: &'static str = "Compiler diagnostics were not requested";
    fn family(_: &Snapshot) -> status::Family {
        status::Family::CompilerDiagnostic
    }
    fn assess(&self, previous: &Snapshot) -> owner_population::Assessment {
        if self
            .request
            .is_some_and(|request| !request.includes(&previous.rule_id))
        {
            return (
                status::State::NotRequested,
                "Compiler diagnostic rule is outside the requested rules".into(),
            );
        }
        if !self.selected {
            return (
                status::State::NotRequested,
                "Compiler diagnostic owner is outside the selected files".into(),
            );
        }

        let Some(suppression) = &self.assessed.suppression else {
            return unavailable("Compiler diagnostic owner is excluded by policy");
        };
        if let Some(index) = self.assessed.findings.get(&previous.rule_id) {
            return positive(&self.snapshots[*index], previous);
        }

        if matches!(suppression, Suppression::Invalid(_))
            || matches!(suppression, Suppression::Allow(rules) if rules.contains(&previous.rule_id))
        {
            return unavailable(
                "Suppressed or invalid-suppression scope cannot establish a numeric negative",
            );
        }
        if super::rules::is_complexity(&previous.rule_id) && !self.assessed.complexity_measured {
            return unavailable("Compiler complexity was not measured for this owner");
        }
        if previous.rule_id == "side_effect_density" && !self.effect_scope_complete {
            return unavailable(
                "Compiler effect propagation has unavailable owners or unresolved callees",
            );
        }
        negative(self, previous)
    }
}

fn positive(
    current: &Result<Snapshot, String>,
    previous: &Snapshot,
) -> owner_population::Assessment {
    match current {
        Err(reason) => unavailable(reason),
        Ok(current) if current == previous => (
            status::State::HiddenExact,
            "Exact compiler diagnostic remains current".into(),
        ),
        Ok(current) => (
            status::State::Stale,
            format!(
                "Compiler diagnostic changed: {:?}",
                super::super::exact_rules::stale_reason(previous, current)
            ),
        ),
    }
}

fn negative(current: &Row<'_>, previous: &Snapshot) -> owner_population::Assessment {
    if previous.versions != local_adapter::versions("compiler-diagnostics")
        || previous.metrics != current.metrics
    {
        return unavailable("Compiler diagnostic versions do not establish a current negative");
    }
    if current.policy != &previous.policy {
        return (
            status::State::Stale,
            "Compiler policy changed; the current negative cannot retire the retained choice"
                .into(),
        );
    }
    (
        status::State::NoFinding,
        "Core numeric rule completed under the retained policy without a finding".into(),
    )
}

fn unavailable(detail: &str) -> owner_population::Assessment {
    (status::State::Unavailable, detail.into())
}
