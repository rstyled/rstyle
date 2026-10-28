use super::super::{Availability, mutations, selected_context};
use super::{Census, Limits, Witness};
use crate::{nominal_boundary::UnitContext, unit_store::UnitStore};
use std::collections::{BTreeMap, BTreeSet};

const MAX_FUNCTIONS: usize = 4096;

pub struct Session<'a> {
    unit: &'a UnitStore,
    limits: Limits,
    observe_uses: bool,
    definitions: BTreeMap<&'a str, usize>,
    observations: BTreeMap<usize, mutations::Observations>,
    contexts: BTreeMap<usize, Availability<UnitContext>>,
    targets: BTreeMap<usize, Vec<Witness>>,
    completed: Option<Census>,
    failure: Option<String>,
}

impl<'a> Session<'a> {
    pub fn new(unit: &'a UnitStore, limits: Limits) -> Result<Self, String> {
        Self::configured(unit, limits, false)
    }

    pub fn new_with_uses(unit: &'a UnitStore, limits: Limits) -> Result<Self, String> {
        Self::configured(unit, limits, true)
    }

    fn configured(unit: &'a UnitStore, limits: Limits, observe_uses: bool) -> Result<Self, String> {
        Ok(Self {
            unit,
            limits,
            observe_uses,
            definitions: definitions(unit)?,
            observations: BTreeMap::new(),
            contexts: BTreeMap::new(),
            targets: BTreeMap::new(),
            completed: None,
            failure: None,
        })
    }

    pub fn into_census(self) -> Option<Census> {
        self.completed
    }

    pub fn census(&self) -> Option<&Census> {
        self.completed.as_ref()
    }

    pub fn observations(&self, owner: usize) -> Option<&mutations::Observations> {
        self.observations.get(&owner)
    }

    pub fn witnesses(&self, target: usize) -> &[Witness] {
        self.targets.get(&target).map_or(&[], Vec::as_slice)
    }

    pub fn call(&self, witness: &Witness) -> Option<&mutations::Call> {
        self.observations(witness.caller_index)?
            .calls
            .get(witness.call_index)
    }

    fn invalidate(&mut self, error: &str) {
        self.discard_observations();
        self.failure = Some(error.into());
    }

    fn discard_observations(&mut self) {
        self.observations.clear();
        self.targets.clear();
        self.contexts.clear();
    }

    pub fn inspect_all(&mut self) -> Result<&Census, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.completed.is_none() {
            match self.inspect() {
                Ok(census) => self.completed = Some(census),
                Err(error) => {
                    self.invalidate(&error);
                    return Err(error);
                }
            }
        }
        Ok(self.completed.as_ref().unwrap())
    }

    fn inspect(&mut self) -> Result<Census, String> {
        let mut census = Census::new(self.unit.functions().len());
        let count = self
            .unit
            .functions()
            .len()
            .min(self.limits.functions)
            .min(MAX_FUNCTIONS);
        if count < census.expected_functions {
            census.exhausted(
                None,
                "Caller function population or observation budget exhausted",
            );
        }

        for owner in 0..count {
            if !self.observe(owner, &mut census)? {
                break;
            }
        }
        Ok(census)
    }

    fn observe(&mut self, owner: usize, census: &mut Census) -> Result<bool, String> {
        let observed = if self.observe_uses {
            mutations::inspect_calls_with_uses(&self.unit.functions()[owner])?
        } else {
            mutations::inspect_calls(&self.unit.functions()[owner])?
        };
        self.retain_observation(owner, observed, census)
    }

    fn retain_observation(
        &mut self,
        owner: usize,
        observed: mutations::Observations,
        census: &mut Census,
    ) -> Result<bool, String> {
        let arguments: usize = observed.calls.iter().map(|call| call.arguments.len()).sum();
        let uses = observed.uses.formals.len()
            + observed.uses.occurrences.len()
            + observed.uses.boundaries.len();
        if observed.calls.len() > self.limits.calls.saturating_sub(census.call_count)
            || arguments > self.limits.arguments.saturating_sub(census.argument_count)
            || uses > self.limits.uses.saturating_sub(census.use_entries)
        {
            census.exhausted(
                Some(owner),
                "Caller call, argument or formal-use retention budget exhausted",
            );
            return Ok(false);
        }

        self.publish_observation(owner, observed, census)
    }

    fn publish_observation(
        &mut self,
        owner: usize,
        observed: mutations::Observations,
        census: &mut Census,
    ) -> Result<bool, String> {
        let arguments = observed.calls.iter().map(|call| call.arguments.len()).sum();
        let uses = observed.uses.formals.len()
            + observed.uses.occurrences.len()
            + observed.uses.boundaries.len();

        count_observation(census, observed.calls.len(), arguments);
        census.use_entries += uses;
        self.inspect_observation(owner, &observed, census)?;
        self.observations.insert(owner, observed);
        Ok(true)
    }

    fn inspect_observation(
        &mut self,
        owner: usize,
        observed: &mutations::Observations,
        census: &mut Census,
    ) -> Result<(), String> {
        self.owner_coverage(owner, observed, census)?;
        self.index_calls(owner, observed, census)
    }

    fn owner_coverage(
        &mut self,
        owner: usize,
        observed: &mutations::Observations,
        census: &mut Census,
    ) -> Result<(), String> {
        selected_context::retain(&mut self.contexts, self.unit, owner)?;
        self.original_coverage(owner, observed, census);

        if observed.direct.execution == mutations::Execution::BudgetExhausted {
            census.exhausted(
                Some(owner),
                "Original caller syntax or native observation budget exhausted",
            );
        }
        if hidden_boundaries(observed)? {
            census.incomplete(
                Some(owner),
                None,
                "Original caller contains an unexamined boundary that may hide calls",
            );
        }
        Ok(())
    }

    fn original_coverage(
        &self,
        owner: usize,
        observed: &mutations::Observations,
        census: &mut Census,
    ) {
        if !matches!(self.contexts[&owner], Availability::Known(_))
            || !matches!(observed.signature, Availability::Known(()))
            || !matches!(observed.direct.availability, Availability::Known(()))
        {
            census.incomplete(
                Some(owner),
                None,
                "Original caller source, signature or selected context is unavailable",
            );
        }

        let identity = self.unit.functions()[owner]
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str);
        if identity
            .and_then(|identity| self.definitions.get(identity))
            .copied()
            != Some(owner)
        {
            census.incomplete(
                Some(owner),
                None,
                "Original caller definition identity is unavailable",
            );
        }
    }

    fn index_calls(
        &mut self,
        owner: usize,
        observed: &mutations::Observations,
        census: &mut Census,
    ) -> Result<(), String> {
        for (call_index, call) in observed.calls.iter().enumerate() {
            if let Some(target_index) = self.target(owner, call)? {
                self.targets.entry(target_index).or_default().push(Witness {
                    caller_index: owner,
                    call_index,
                    target_index,
                });
            } else {
                census.incomplete(
                    Some(owner),
                    Some(call.span),
                    "Exact original same-unit call target is unavailable",
                );
            }
            if call
                .arguments
                .iter()
                .any(|argument| !matches!(argument.value, Availability::Known(_)))
            {
                census.incomplete(Some(owner), Some(call.span), "Call arguments contain unsupported or unexamined evaluation that may hide calls");
            }
        }
        Ok(())
    }

    fn target(&mut self, owner: usize, call: &mutations::Call) -> Result<Option<usize>, String> {
        let Availability::Known(identity) = &call.target else {
            return Ok(None);
        };
        let Some(target) = self.definitions.get(identity.as_str()).copied() else {
            return Ok(None);
        };
        if call.target_path.as_deref() != Some(self.unit.functions()[target].name.as_str()) {
            return Err(
                "Caller selected definition identity contradicts its captured native path".into(),
            );
        }

        selected_context::retain(&mut self.contexts, self.unit, target)?;
        match (&self.contexts[&owner], &self.contexts[&target]) {
            (Availability::Known(left), Availability::Known(right)) if left != right => {
                Err("Caller target belongs to another selected-unit context".into())
            }
            (Availability::Known(_), Availability::Known(_)) => Ok(Some(target)),
            _ => Ok(None),
        }
    }
}

fn count_observation(census: &mut Census, calls: usize, arguments: usize) {
    census.scanned_functions += 1;
    census.call_count += calls;
    census.argument_count += arguments;
}

fn hidden_boundaries(observed: &mutations::Observations) -> Result<bool, String> {
    let mut checked = BTreeSet::new();
    for call in &observed.calls {
        let boundary = observed
            .direct
            .boundaries
            .get(call.boundary_index)
            .ok_or("Caller boundary ordinal is outside its original inventory")?;
        if boundary.span != call.span || !checked.insert(call.boundary_index) {
            return Err(
                "Caller boundary span or identity contradicts its original occurrence".into(),
            );
        }
    }
    Ok(observed
        .direct
        .boundaries
        .iter()
        .enumerate()
        .any(|(index, _)| !checked.contains(&index))
        || (observed.direct.coverage == mutations::Coverage::Incomplete
            && observed.direct.boundaries.is_empty()))
}

fn definitions(unit: &UnitStore) -> Result<BTreeMap<&str, usize>, String> {
    let mut definitions = BTreeMap::new();
    for (index, facts) in unit.functions().iter().take(MAX_FUNCTIONS).enumerate() {
        if let Some(value) = facts.build.frontend_options.get("definition_identity") {
            let identity = value
                .as_str()
                .filter(|identity| !identity.is_empty())
                .ok_or("Caller owning-unit definition identity is malformed")?;
            if definitions.insert(identity, index).is_some() {
                return Err("Caller owning-unit definition inventory is ambiguous".into());
            }
        }
    }
    Ok(definitions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteSpan, HirFnFacts, HirRef, nominal_boundary};

    fn unit() -> UnitStore {
        let context = nominal_boundary::tests::context();
        let functions = (0..2)
            .map(|index| {
                let mut facts: HirFnFacts = serde_json::from_str(include_str!(
                    "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
                ))
                .unwrap();
                facts.name = format!("sample::function_{index}");
                facts.build.frontend_options = Default::default();
                facts.build.frontend_options.insert(
                    "definition_identity".into(),
                    serde_json::json!(format!("definition-{index}")),
                );
                facts.build.frontend_options.insert(
                    nominal_boundary::CONTEXT_KEY.into(),
                    serde_json::to_value(&context).unwrap(),
                );
                facts
            })
            .collect();
        UnitStore::capture(functions).unwrap()
    }

    fn observed() -> mutations::Observations {
        let span = ByteSpan { start: 0, end: 1 };
        mutations::Observations {
            uses: mutations::UseInventory::unrequested(),
            signature: Availability::Known(()),
            direct: mutations::Assessment {
                availability: Availability::Known(()),
                coverage: mutations::Coverage::Incomplete,
                execution: mutations::Execution::Completed,
                sites: Vec::new(),
                boundaries: vec![mutations::Boundary {
                    availability: mutations::BoundaryAvailability::Unknown,
                    span,
                    reason: "Unknown call boundary".into(),
                }],
            },
            calls: vec![mutations::Call {
                span,
                native: HirRef {
                    krate: 0,
                    owner: 1,
                    local: 1,
                },
                boundary_index: 0,
                target: Availability::Known("definition-1".into()),
                resolved_callee: Availability::Unknown {
                    reason: "Unrequested native callee metadata".into(),
                },
                target_path: Some("sample::function_1".into()),
                arguments: vec![mutations::Argument {
                    unchanged_formal: Availability::Known(Some(0)),
                    span,
                    value: Availability::Known(mutations::ArgumentValue::OwnedU32),
                    formal: Availability::Known(Some(0)),
                }],
            }],
        }
    }

    #[test]
    fn exact_call_index_does_not_require_complete_effects_or_supported_arguments() {
        let unit = unit();
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        let mut observed = observed();
        observed.signature = Availability::Unsupported {
            reason: "Unbounded signature".into(),
        };
        observed.calls[0].arguments[0].value = Availability::Unsupported {
            reason: "Nested evaluation".into(),
        };
        let mut census = Census::new(2);
        session.owner_coverage(0, &observed, &mut census).unwrap();
        session.index_calls(0, &observed, &mut census).unwrap();
        session.observations.insert(0, observed);
        assert_eq!(census.coverage, mutations::Coverage::Incomplete);
        let witnesses = session.witnesses(1);
        assert_eq!(
            witnesses,
            &[Witness {
                caller_index: 0,
                call_index: 0,
                target_index: 1
            }]
        );
        assert_eq!(
            session.call(&witnesses[0]).unwrap().arguments[0].formal,
            Availability::Known(Some(0))
        );
    }

    #[test]
    fn unmatched_original_boundary_cannot_become_complete_no_callers() {
        let unit = unit();
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        let mut observed = observed();
        observed.calls.clear();
        let mut census = Census::new(2);
        session.owner_coverage(0, &observed, &mut census).unwrap();
        assert_eq!(census.coverage, mutations::Coverage::Incomplete);
        assert!(!census.reasons.is_empty());
        assert!(session.witnesses(1).is_empty());
    }

    #[test]
    fn definition_path_and_selected_context_contradictions_error() {
        let unit = unit();
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        let mut observed = observed();
        let mut census = Census::new(2);
        session.owner_coverage(0, &observed, &mut census).unwrap();
        observed.calls[0].target_path = Some("sample::wrong".into());
        assert!(session.index_calls(0, &observed, &mut census).is_err());
        observed.calls[0].target_path = Some("sample::function_1".into());
        let mut other = nominal_boundary::tests::context();
        other.crate_name = "other".into();
        session.contexts.insert(1, Availability::Known(other));
        assert!(session.index_calls(0, &observed, &mut census).is_err());
    }

    #[test]
    fn global_census_is_retained_once_and_budgets_remain_incomplete() {
        let unit = unit();
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        let census = session.inspect_all().unwrap();
        assert_eq!(census.scanned_functions, 2);
        assert_eq!(census.expected_functions, 2);
        assert_eq!(census.coverage, mutations::Coverage::Incomplete);
        let retained = census as *const Census;
        assert_eq!(session.inspect_all().unwrap() as *const Census, retained);
        let mut session = Session::new(
            &unit,
            Limits {
                functions: 0,
                ..Limits::default()
            },
        )
        .unwrap();
        let census = session.inspect_all().unwrap();
        assert_eq!(census.scanned_functions, 0);
        assert_eq!(census.execution, mutations::Execution::BudgetExhausted);
        assert_eq!(census.coverage, mutations::Coverage::Incomplete);
    }

    #[test]
    fn total_use_retention_cannot_multiply_the_allowance_per_owner() {
        let unit = unit();
        let mut session = Session::new_with_uses(
            &unit,
            Limits {
                uses: 1,
                ..Limits::default()
            },
        )
        .unwrap();
        let mut census = Census::new(2);
        let mut first = observed();
        first.uses.boundaries.push(mutations::Boundary {
            availability: mutations::BoundaryAvailability::Unknown,
            span: ByteSpan { start: 0, end: 1 },
            reason: "First owner inventory gap".into(),
        });
        assert!(session.retain_observation(0, first, &mut census).unwrap());
        let mut second = observed();
        second.uses.boundaries.push(mutations::Boundary {
            availability: mutations::BoundaryAvailability::Unknown,
            span: ByteSpan { start: 0, end: 1 },
            reason: "Second owner inventory gap".into(),
        });
        assert!(!session.retain_observation(1, second, &mut census).unwrap());
        assert_eq!(census.use_entries, 1);
        assert_eq!(census.scanned_functions, 1);
        assert_eq!(census.execution, mutations::Execution::BudgetExhausted);
        assert_eq!(census.coverage, mutations::Coverage::Incomplete);
        assert!(session.observations(0).is_some());
        assert!(session.observations(1).is_none());
    }

    #[test]
    fn boundary_ownership_and_ambiguous_original_definitions_reject() {
        let unit = unit();
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        let mut observed = observed();
        observed.calls[0].boundary_index = 1;
        assert!(
            session
                .owner_coverage(0, &observed, &mut Census::new(2))
                .is_err()
        );
        let mut facts = unit.functions().to_vec();
        facts[1].build.frontend_options.insert(
            "definition_identity".into(),
            serde_json::json!("definition-0"),
        );
        // The immutable unit rejects this contradiction before a caller session exists.
        assert!(UnitStore::capture(facts).is_err());
    }
}
