mod summaries;
use super::{
    super::{Availability, formal_types, mutations, selected_context},
    model::*,
};
use crate::{HirFnFacts, nominal_boundary::UnitContext, unit_store::UnitStore};
use std::collections::{BTreeMap, BTreeSet};

const MAX_FUNCTION_POPULATION: usize = 4096;

pub struct Session<'a> {
    unit: &'a UnitStore,
    limits: Limits,
    definitions: BTreeMap<&'a str, usize>,
    observations: BTreeMap<usize, Result<Option<mutations::Observations>, String>>,
    contexts: BTreeMap<usize, Availability<UnitContext>>,
    summaries: Vec<Summary>,
    visible_sites: Vec<Vec<usize>>,
    completed: BTreeMap<(usize, QueryKind), Completed>,
    work: Work,
}

#[derive(Default)]
struct Work {
    statistics: Statistics,
    retained_sites: usize,
    retained_calls: usize,
}

#[derive(Clone)]
struct Prepared {
    summary: Option<usize>,
    budget: bool,
    reason: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum QueryKind {
    Assessment,
    Summary(u8),
}

#[derive(Clone)]
enum Completed {
    Assessment(Assessment),
    Summary(Prepared),
}

impl Completed {
    fn cacheable(&self) -> bool {
        match self {
            Self::Assessment(assessment) => {
                assessment.execution != mutations::Execution::BudgetExhausted
            }
            Self::Summary(prepared) => !prepared.budget,
        }
    }
}

impl TryFrom<Completed> for Assessment {
    type Error = String;

    fn try_from(completed: Completed) -> Result<Self, Self::Error> {
        match completed {
            Completed::Assessment(assessment) => Ok(assessment),
            Completed::Summary(_) => Err("Effect assessment query returned a leaf outcome".into()),
        }
    }
}

impl TryFrom<Completed> for Prepared {
    type Error = String;

    fn try_from(completed: Completed) -> Result<Self, Self::Error> {
        match completed {
            Completed::Summary(prepared) => Ok(prepared),
            Completed::Assessment(_) => {
                Err("Effect leaf query returned an assessment outcome".into())
            }
        }
    }
}

impl QueryKind {
    fn budget(self, index: usize) -> Completed {
        match self {
            Self::Assessment => Completed::Assessment(budget_assessment(index)),
            Self::Summary(_) => Completed::Summary(Prepared::unavailable(
                "Effect owner observation budget exhausted",
                true,
            )),
        }
    }
}

impl<'a> Session<'a> {
    pub fn new(unit: &'a UnitStore, limits: Limits) -> Result<Self, String> {
        Ok(Self {
            unit,
            limits,
            definitions: definitions(unit)?,
            observations: BTreeMap::new(),
            contexts: BTreeMap::new(),
            summaries: Vec::new(),
            visible_sites: Vec::new(),
            completed: BTreeMap::new(),
            work: Work::default(),
        })
    }

    pub fn observations(&self, index: usize) -> Option<&mutations::Observations> {
        self.observations.get(&index)?.as_ref().ok()?.as_ref()
    }

    pub fn summaries(&self) -> &[Summary] {
        &self.summaries
    }

    pub fn statistics(&self) -> Statistics {
        self.work.statistics
    }

    pub fn assess(&mut self, index: usize) -> Result<Assessment, String> {
        self.query_result(index, QueryKind::Assessment)
    }

    fn query_result<T>(&mut self, index: usize, kind: QueryKind) -> Result<T, String>
    where
        T: TryFrom<Completed, Error = String>,
    {
        self.query(index, kind)?.try_into()
    }

    fn query(&mut self, index: usize, kind: QueryKind) -> Result<Completed, String> {
        if let Some(completed) = self.completed.get(&(index, kind)) {
            return Ok(completed.clone());
        }
        if !self.observe(index)? {
            return Ok(kind.budget(index));
        }

        let completed = self.compute(index, kind)?;
        if completed.cacheable() {
            self.completed.insert((index, kind), completed.clone());
        }
        Ok(completed)
    }

    fn compute(&mut self, index: usize, kind: QueryKind) -> Result<Completed, String> {
        match kind {
            QueryKind::Assessment => self.assess_observed(index, 1).map(Completed::Assessment),
            QueryKind::Summary(depth) => self.retain_summary(index, depth).map(Completed::Summary),
        }
    }

    fn assess_observed(&mut self, index: usize, depth: u8) -> Result<Assessment, String> {
        let count = self.observations(index).unwrap().calls.len();
        let prepared = (0..count)
            .map(|call| self.prepare(index, call, depth))
            .collect::<Result<Vec<_>, _>>()?;

        self.compose(index, &prepared)
    }

    fn observe(&mut self, index: usize) -> Result<bool, String> {
        if let Some(result) = self.observations.get(&index) {
            return result.as_ref().map(Option::is_some).map_err(Clone::clone);
        }
        let facts = self
            .unit
            .functions()
            .get(index)
            .ok_or("Effect function ordinal is outside its owning unit")?;
        if self.unit.functions().len() > MAX_FUNCTION_POPULATION
            || self.work.statistics.observations >= self.limits.functions
        {
            return Ok(false);
        }

        self.work.statistics.observations += 1;
        let result = mutations::inspect_effect_calls(facts).map(|observed| {
            if self.work.retain(&observed, self.limits) {
                Some(observed)
            } else {
                None
            }
        });
        self.observations.insert(index, result);
        self.observations[&index]
            .as_ref()
            .map(Option::is_some)
            .map_err(Clone::clone)
    }

    fn prepare(&mut self, caller: usize, call_index: usize, depth: u8) -> Result<Prepared, String> {
        if self.work.statistics.calls >= self.limits.calls {
            return Ok(Prepared::unavailable(
                "Effect call work budget exhausted",
                true,
            ));
        }

        self.work.statistics.calls += 1;
        let call = &self.observations(caller).unwrap().calls[call_index];
        let target = match &call.target {
            Availability::Known(target) => target,
            _ => {
                return Ok(Prepared::unavailable(
                    "Exact same-unit call target is unavailable",
                    false,
                ));
            }
        };
        let Some(callee) = self.definitions.get(target.as_str()).copied() else {
            return Ok(Prepared::unavailable(
                "Exact callee body is absent from the bounded owning-unit roster",
                self.unit.functions().len() > MAX_FUNCTION_POPULATION,
            ));
        };
        if call.target_path.as_deref() != Some(self.unit.functions()[callee].name.as_str()) {
            return Err(
                "Effect selected callee identity contradicts its captured native path".into(),
            );
        }
        if !self.check_contexts(caller, callee)? {
            return Ok(Prepared::unavailable(
                "Call selected-unit context is unavailable",
                false,
            ));
        }
        self.query_result(callee, QueryKind::Summary(depth))
    }

    fn check_contexts(&mut self, caller: usize, callee: usize) -> Result<bool, String> {
        selected_context::retain(&mut self.contexts, self.unit, caller)?;
        selected_context::retain(&mut self.contexts, self.unit, callee)?;
        match (&self.contexts[&caller], &self.contexts[&callee]) {
            (Availability::Known(left), Availability::Known(right)) if left != right => {
                Err("Effect call target belongs to another selected-unit context".into())
            }
            (Availability::Known(_), Availability::Known(_)) => Ok(true),
            _ => Ok(false),
        }
    }

    fn leaf_summary(&mut self, index: usize) -> Result<Prepared, String> {
        self.query_result(index, QueryKind::Summary(0))
    }

    fn retain_leaf(&mut self, index: usize) -> Result<Prepared, String> {
        self.work.statistics.leaf_checks += 1;
        self.complete_leaf(index)
    }

    fn complete_leaf(&mut self, index: usize) -> Result<Prepared, String> {
        let observed = self.observations(index).unwrap();
        if !leaf_complete(observed) {
            return Ok(Prepared::unavailable(
                "Callee is not a complete bounded call-free scalar leaf",
                observed.direct.execution == mutations::Execution::BudgetExhausted,
            ));
        }
        if !matches!(self.contexts.get(&index), Some(Availability::Known(_))) {
            return Ok(Prepared::unavailable(
                "Callee selected-unit context is unavailable",
                false,
            ));
        }

        let Some(parameters) = parameters(&self.unit.functions()[index])? else {
            return Ok(Prepared::unavailable(
                "Callee formal type authority is unavailable",
                false,
            ));
        };
        let count = observed.direct.sites.len();
        if !self.summary_fits(count) {
            return Ok(Prepared::unavailable(
                "Effect summary retention budget exhausted",
                true,
            ));
        }

        let sites = self.direct_witnesses(index);
        Ok(self.publish_summary(
            Summary {
                function_index: index,
                parameters,
                sites,
            },
            count,
        ))
    }
}

impl Work {
    fn retain(&mut self, observed: &mutations::Observations, limits: Limits) -> bool {
        let sites = self
            .retained_sites
            .saturating_add(observed.direct.sites.len());
        let calls = self.retained_calls.saturating_add(observed.calls.len());
        if sites.saturating_add(self.statistics.mapped_sites) > limits.sites || calls > limits.calls
        {
            return false;
        }

        self.retained_sites = sites;
        self.retained_calls = calls;
        true
    }
}

impl Prepared {
    fn unavailable(reason: &str, budget: bool) -> Self {
        Self {
            summary: None,
            budget,
            reason: Some(reason.into()),
        }
    }
}

fn definitions(unit: &UnitStore) -> Result<BTreeMap<&str, usize>, String> {
    if unit.functions().len() > MAX_FUNCTION_POPULATION {
        return Ok(BTreeMap::new());
    }
    let mut definitions = BTreeMap::new();
    for (index, facts) in unit
        .functions()
        .iter()
        .take(MAX_FUNCTION_POPULATION)
        .enumerate()
    {
        if let Some(identity) = facts
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            && (identity.is_empty() || definitions.insert(identity, index).is_some())
        {
            return Err("Effect owning-unit definition inventory is empty or ambiguous".into());
        }
    }
    Ok(definitions)
}

pub(super) fn leaf_complete(observed: &mutations::Observations) -> bool {
    matches!(observed.signature, Availability::Known(()))
        && matches!(observed.direct.availability, Availability::Known(()))
        && observed.direct.coverage == mutations::Coverage::Complete
        && observed.direct.execution == mutations::Execution::Completed
        && observed.direct.boundaries.is_empty()
        && observed.calls.is_empty()
}

fn parameters(facts: &HirFnFacts) -> Result<Option<Vec<Parameter>>, String> {
    if facts.signature.params.len() > 64 {
        return Ok(None);
    }
    facts
        .signature
        .params
        .iter()
        .map(|parameter| {
            if parameter.ty.normalized.as_deref() == Some("u32") {
                Ok(Some(Parameter::OwnedU32))
            } else {
                formal_types::mutable_u32(facts, parameter.position, &parameter.pat.hir)
                    .map(|known| known.then_some(Parameter::MutableU32))
            }
        })
        .collect::<Result<Option<Vec<_>>, _>>()
}

pub(super) struct ResolvedCall {
    pub(super) event: Option<CallEvent>,
    pub(super) reason: Option<String>,
    pub(super) budget: bool,
}

#[derive(Default)]
struct Composition {
    outcomes: Vec<ResolvedCall>,
    mapped_sites: usize,
}

/// The checked original occurrence travels with its position in the call roster.
struct CallOccurrence<'a> {
    index: usize,
    call: &'a mutations::Call,
}

impl Composition {
    fn accept(&mut self, outcome: ResolvedCall) {
        self.mapped_sites += outcome
            .event
            .as_ref()
            .map_or(0, |event| event.mappings.len());
        self.outcomes.push(outcome);
    }
}

impl Session<'_> {
    fn compose(&mut self, index: usize, prepared: &[Prepared]) -> Result<Assessment, String> {
        let observed = self.observations(index).unwrap();
        validate_calls(observed)?;
        let remaining = self
            .limits
            .sites
            .saturating_sub(self.work.retained_sites + self.work.statistics.mapped_sites);
        let mut composition = Composition::default();
        for (call_index, prepared) in prepared.iter().enumerate() {
            let outcome = self.resolve_call(
                CallOccurrence {
                    index: call_index,
                    call: &observed.calls[call_index],
                },
                prepared,
                remaining - composition.mapped_sites,
            )?;
            composition.accept(outcome);
        }

        let assessment = assessment(index, observed, composition.outcomes)?;
        self.work.statistics.mapped_sites += composition.mapped_sites;
        Ok(assessment)
    }

    fn resolve_call(
        &self,
        occurrence: CallOccurrence<'_>,
        prepared: &Prepared,
        remaining: usize,
    ) -> Result<ResolvedCall, String> {
        let CallOccurrence {
            index: call_index,
            call,
        } = occurrence;
        let Some(summary_index) = prepared.summary else {
            return Ok(ResolvedCall {
                event: None,
                reason: prepared.reason.clone(),
                budget: prepared.budget,
            });
        };
        let summary = &self.summaries[summary_index];
        let visible = &self.visible_sites[summary_index];
        if visible.len() > remaining {
            return Ok(ResolvedCall::unavailable(
                "Effect mapped-site work budget exhausted",
                true,
            ));
        }

        let Some(mappings) = mappings(call, summary, visible)? else {
            return Ok(ResolvedCall::unavailable(
                "Exact call arguments do not satisfy the leaf formal mapping",
                false,
            ));
        };
        Ok(ResolvedCall {
            event: Some(CallEvent {
                call_index,
                span: call.span,
                native: call.native.clone(),
                summary_index,
                mappings,
            }),
            reason: None,
            budget: false,
        })
    }
}

impl ResolvedCall {
    fn unavailable(reason: &str, budget: bool) -> Self {
        Self {
            event: None,
            reason: Some(reason.into()),
            budget,
        }
    }
}

fn validate_calls(observed: &mutations::Observations) -> Result<(), String> {
    let mut boundaries = BTreeSet::new();
    for call in &observed.calls {
        let boundary = observed
            .direct
            .boundaries
            .get(call.boundary_index)
            .ok_or("Effect call boundary ordinal is outside its original inventory")?;
        if boundary.span != call.span || !boundaries.insert(call.boundary_index) {
            return Err("Effect call boundary ownership is contradictory or duplicated".into());
        }
    }
    Ok(())
}

pub(super) fn mappings(
    call: &mutations::Call,
    summary: &Summary,
    visible: &[usize],
) -> Result<Option<Vec<Mapping>>, String> {
    if summary.parameters.len() > 64
        || call.arguments.len() > 64
        || call.arguments.len() != summary.parameters.len()
        || !call
            .arguments
            .iter()
            .zip(&summary.parameters)
            .all(argument_matches)
    {
        return Ok(None);
    }
    visible
        .iter()
        .map(|index| site_mapping(*index, summary.sites.get(*index), call))
        .collect::<Result<Option<Vec<_>>, _>>()
}

fn argument_matches((argument, parameter): (&mutations::Argument, &Parameter)) -> bool {
    match (&argument.value, parameter) {
        (Availability::Known(mutations::ArgumentValue::OwnedU32), Parameter::OwnedU32) => true,
        (
            Availability::Known(mutations::ArgumentValue::MutableFormal(root)),
            Parameter::MutableU32,
        ) => root.provenance == mutations::Provenance::FormalBorrowed && root.formal.is_some(),
        _ => false,
    }
}

fn site_mapping(
    index: usize,
    site: Option<&SiteWitness>,
    call: &mutations::Call,
) -> Result<Option<Mapping>, String> {
    let site = site.ok_or("Effect summary site ordinal is outside its original callee")?;
    if site.root.provenance != mutations::Provenance::FormalBorrowed {
        return Err("Effect caller-visible site does not name a borrowed formal".into());
    }
    let Some(formal) = site.root.formal else {
        return Err("Effect borrowed site has no formal ordinal".into());
    };

    let Some(argument) = call.arguments.get(formal as usize) else {
        return Ok(None);
    };
    let Availability::Known(mutations::ArgumentValue::MutableFormal(root)) = &argument.value else {
        return Ok(None);
    };
    Ok(Some(Mapping {
        summary_site_index: index,
        caller_root: root.clone(),
    }))
}

pub(super) fn assessment(
    index: usize,
    observed: &mutations::Observations,
    outcomes: Vec<ResolvedCall>,
) -> Result<Assessment, String> {
    let resolved: BTreeSet<_> = outcomes
        .iter()
        .filter_map(|outcome| outcome.event.as_ref())
        .map(|event| observed.calls[event.call_index].boundary_index)
        .collect();
    let unresolved_boundaries: Vec<_> = (0..observed.direct.boundaries.len())
        .filter(|index| !resolved.contains(index))
        .collect();
    let budget = observed.direct.execution == mutations::Execution::BudgetExhausted
        || outcomes.iter().any(|outcome| outcome.budget);
    let complete = complete_coverage(observed, &unresolved_boundaries, budget);
    let reasons = outcomes
        .iter()
        .filter_map(|outcome| outcome.reason.clone())
        .collect();
    let calls: Vec<_> = outcomes
        .into_iter()
        .filter_map(|outcome| outcome.event)
        .collect();
    let roots = roots(&observed.direct.sites, &calls)?;

    Ok(Assessment {
        function_index: index,
        availability: observed.direct.availability.clone(),
        signature: observed.signature.clone(),
        direct_site_count: observed.direct.sites.len(),
        direct_coverage: observed.direct.coverage,
        coverage: if complete {
            mutations::Coverage::Complete
        } else {
            mutations::Coverage::Incomplete
        },
        execution: if budget {
            mutations::Execution::BudgetExhausted
        } else {
            mutations::Execution::Completed
        },
        caller_visible_mutation: visible(&roots, complete),
        roots,
        calls,
        unresolved_boundaries,
        reasons,
    })
}

fn complete_coverage(
    observed: &mutations::Observations,
    unresolved: &[usize],
    budget: bool,
) -> bool {
    matches!(observed.direct.availability, Availability::Known(()))
        && matches!(observed.signature, Availability::Known(()))
        && (observed.direct.coverage == mutations::Coverage::Complete
            || !observed.direct.boundaries.is_empty())
        && unresolved.is_empty()
        && !budget
}

fn roots(
    direct: &[mutations::WriteSite],
    calls: &[CallEvent],
) -> Result<Vec<mutations::Root>, String> {
    let mut roots = BTreeMap::new();
    for root in direct.iter().map(|site| &site.root).chain(
        calls
            .iter()
            .flat_map(|call| &call.mappings)
            .map(|mapping| &mapping.caller_root),
    ) {
        let key = (root.native.krate, root.native.owner, root.native.local);
        if let Some(previous) = roots.insert(key, root)
            && previous != root
        {
            return Err(
                "Effect root metadata contradicts another witness for the same native binding"
                    .into(),
            );
        }
    }
    Ok(roots.into_values().cloned().collect())
}

pub(super) fn visible(roots: &[mutations::Root], complete: bool) -> CallerVisibleMutation {
    match (
        roots
            .iter()
            .any(|root| root.provenance == mutations::Provenance::FormalBorrowed),
        complete,
    ) {
        (true, true) => CallerVisibleMutation::WritesFormal,
        (true, false) => CallerVisibleMutation::WitnessedWritesFormalWithUnknownRemainder,
        (false, true) => CallerVisibleMutation::NoneProven,
        (false, false) => CallerVisibleMutation::Unknown,
    }
}

fn budget_assessment(index: usize) -> Assessment {
    Assessment {
        function_index: index,
        availability: Availability::Unknown {
            reason: "Effect owner observation or retention budget exhausted".into(),
        },
        signature: Availability::Unknown {
            reason: "Effect owner signature was not inspected".into(),
        },
        direct_site_count: 0,
        direct_coverage: mutations::Coverage::Incomplete,
        coverage: mutations::Coverage::Incomplete,
        execution: mutations::Execution::BudgetExhausted,
        roots: Vec::new(),
        calls: Vec::new(),
        unresolved_boundaries: Vec::new(),
        reasons: vec!["Effect owner observation or retention budget exhausted".into()],
        caller_visible_mutation: CallerVisibleMutation::Unknown,
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use crate::nominal_boundary;

    fn root() -> mutations::Root {
        mutations::Root {
            declaration_span: span(),
            scope_span: span(),
            native: crate::HirRef {
                krate: 0,
                owner: 1,
                local: 1,
            },
            formal: Some(0),
            provenance: mutations::Provenance::FormalBorrowed,
        }
    }

    fn span() -> crate::ByteSpan {
        crate::ByteSpan { start: 0, end: 1 }
    }

    fn original_sites() -> mutations::Assessment {
        mutations::Assessment {
            availability: Availability::Known(()),
            coverage: mutations::Coverage::Complete,
            execution: mutations::Execution::Completed,
            sites: (0..4096)
                .map(|_| mutations::WriteSite {
                    span: span(),
                    root: root(),
                    operation: mutations::Operation::Assignment,
                    caller_visible: true,
                })
                .collect(),
            boundaries: Vec::new(),
        }
    }

    fn call(boundary_index: usize) -> mutations::Call {
        mutations::Call {
            span: span(),
            native: root().native,
            boundary_index,
            target: Availability::Known("typed-test-leaf".into()),
            resolved_callee: Availability::Unknown {
                reason: "Unrequested native callee metadata".into(),
            },
            target_path: Some("typed-test-leaf".into()),
            arguments: vec![mutations::Argument {
                unchanged_formal: Availability::Known(None),
                formal: Availability::Known(Some(0)),
                span: span(),
                value: Availability::Known(mutations::ArgumentValue::MutableFormal(root())),
            }],
        }
    }

    fn mapping_session(unit: &UnitStore) -> Session<'_> {
        let mut session = Session::new(unit, Limits::default()).unwrap();
        session.observations.insert(
            0,
            Ok(Some(mutations::Observations {
                uses: mutations::UseInventory::unrequested(),
                direct: original_sites(),
                calls: (0..3).map(call).collect(),
                signature: Availability::Known(()),
            })),
        );
        session.summaries.push(Summary {
            function_index: 0,
            sites: (0..4096)
                .map(|site_index| SiteWitness {
                    function_index: 0,
                    site_index,
                    call_path: Vec::new(),
                    root: root(),
                })
                .collect(),
            parameters: vec![Parameter::MutableU32],
        });
        session.visible_sites.push((0..4096).collect());
        session
    }

    #[test]
    fn repeated_large_leaf_checks_budget_before_visiting_original_sites() {
        let unit = super::super::tests::unavailable_unit();
        let mut session = mapping_session(&unit);
        let prepared = Prepared {
            summary: Some(0),
            budget: false,
            reason: None,
        };
        let first = session
            .resolve_call(
                CallOccurrence {
                    index: 0,
                    call: &session.observations(0).unwrap().calls[0],
                },
                &prepared,
                4096,
            )
            .unwrap();
        assert_eq!(first.event.unwrap().mappings.len(), 4096);
        // An invalid sentinel would error if an exhausted call visited any site.
        session.visible_sites[0][0] = usize::MAX;
        for call in 1..3 {
            let exhausted = session
                .resolve_call(
                    CallOccurrence {
                        index: call,
                        call: &session.observations(0).unwrap().calls[call],
                    },
                    &prepared,
                    0,
                )
                .unwrap();
            assert!(exhausted.budget);
            assert!(exhausted.event.is_none());
        }
    }

    fn contextual_unit(contexts: &[Option<UnitContext>]) -> UnitStore {
        let functions = contexts
            .iter()
            .enumerate()
            .map(|(index, context)| {
                let mut facts: HirFnFacts = serde_json::from_str(include_str!(
                    "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
                ))
                .unwrap();
                facts.name = format!("function-{index}");
                facts.build.frontend_options = Default::default();
                facts.build.frontend_options.insert(
                    "definition_identity".into(),
                    serde_json::json!(format!("definition-{index}")),
                );
                if let Some(context) = context {
                    facts.build.frontend_options.insert(
                        nominal_boundary::CONTEXT_KEY.into(),
                        serde_json::to_value(context).unwrap(),
                    );
                }
                facts
            })
            .collect();
        UnitStore::capture(functions).unwrap()
    }

    #[test]
    fn consumed_selected_context_mismatch_errors_and_absence_stays_unavailable() {
        let context = nominal_boundary::tests::context();
        let mut other = context.clone();
        other.crate_name = "other".into();
        let unit = contextual_unit(&[Some(context.clone()), Some(other)]);
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        assert!(session.check_contexts(0, 1).is_err());
        let missing = contextual_unit(&[Some(context), None]);
        let mut session = Session::new(&missing, Limits::default()).unwrap();
        assert!(!session.check_contexts(0, 1).unwrap());
    }

    #[test]
    fn changing_only_target_identity_cannot_select_another_original_definition() {
        let context = nominal_boundary::tests::context();
        let unit = contextual_unit(&[Some(context.clone()), Some(context)]);
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        let mut redirected = call(0);
        redirected.target = Availability::Known("definition-1".into());
        redirected.target_path = Some("function-0".into());
        session.observations.insert(
            0,
            Ok(Some(mutations::Observations {
                uses: mutations::UseInventory::unrequested(),
                direct: original_sites(),
                calls: vec![redirected],
                signature: Availability::Known(()),
            })),
        );
        assert!(session.prepare(0, 0, 1).is_err());
        assert!(session.summaries.is_empty());
    }

    #[test]
    fn identical_original_units_have_distinct_observation_and_summary_caches() {
        let unit = super::super::tests::unavailable_unit();
        let mut first = Session::new(&unit, Limits::default()).unwrap();
        first.assess(0).unwrap();
        let mut second = Session::new(&unit, Limits::default()).unwrap();
        assert_eq!(second.statistics().observations, 0);
        assert!(second.summaries().is_empty());
        second.assess(0).unwrap();
        assert_eq!(first.statistics().observations, 1);
        assert_eq!(second.statistics().observations, 1);
    }

    #[test]
    fn completed_query_kinds_stay_distinct_and_observation_budgets_are_not_published() {
        let unit = super::super::tests::unavailable_unit();
        let mut session = Session::new(&unit, Limits::default()).unwrap();
        session.assess(0).unwrap();
        assert!(session.leaf_summary(0).unwrap().summary.is_none());
        assert!(session.leaf_summary(0).unwrap().summary.is_none());
        assert_eq!(session.completed.len(), 2);
        assert_eq!(session.statistics().observations, 1);
        assert_eq!(session.statistics().leaf_checks, 1);

        let limits = Limits {
            functions: 0,
            ..Limits::default()
        };
        let mut limited = Session::new(&unit, limits).unwrap();
        limited.assess(0).unwrap();
        assert!(limited.leaf_summary(0).unwrap().budget);
        assert!(limited.completed.is_empty());
        assert_eq!(limited.statistics().observations, 0);
    }
    fn synthetic_session<'a>(
        unit: &'a UnitStore,
        edges: &[Option<usize>],
        limits: Limits,
    ) -> Session<'a> {
        let mut session = Session::new(unit, limits).unwrap();
        for (index, target) in edges.iter().enumerate() {
            let calls: Vec<_> = target
                .iter()
                .map(|target| {
                    let mut call = call(0);
                    call.target = Availability::Known(format!("definition-{target}"));
                    call.target_path = Some(format!("function-{target}"));
                    call.arguments.clear();
                    call
                })
                .collect();
            let boundaries = calls
                .iter()
                .map(|call| mutations::Boundary {
                    span: call.span,
                    availability: mutations::BoundaryAvailability::Unknown,
                    reason: "call".into(),
                })
                .collect();
            session.observations.insert(
                index,
                Ok(Some(mutations::Observations {
                    signature: Availability::Known(()),
                    uses: mutations::UseInventory::unrequested(),
                    calls,
                    direct: mutations::Assessment {
                        sites: Vec::new(),
                        boundaries,
                        ..original_sites()
                    },
                })),
            );
            session.check_contexts(index, index).unwrap();
        }
        session
    }

    fn three_functions() -> UnitStore {
        let context = nominal_boundary::tests::context();
        contextual_unit(&[Some(context.clone()), Some(context.clone()), Some(context)])
    }

    #[test]
    fn depth_queries_are_separate_and_call_free_leaf_summaries_are_canonical() {
        let unit = three_functions();
        for order in [[0, 1], [1, 0]] {
            let mut session =
                synthetic_session(&unit, &[Some(1), Some(2), None], Limits::default());
            for depth in order {
                let result: Prepared = session.query_result(1, QueryKind::Summary(depth)).unwrap();
                assert_eq!(result.summary.is_some(), depth == 1);
            }
            let wrapper: Prepared = session.query_result(1, QueryKind::Summary(1)).unwrap();
            assert!(wrapper.summary.is_some());
            let low: Prepared = session.query_result(2, QueryKind::Summary(0)).unwrap();
            let high: Prepared = session.query_result(2, QueryKind::Summary(1)).unwrap();
            assert_eq!(low.summary, high.summary);
            assert_eq!(session.statistics().wrapper_checks, 1);
            assert_eq!(session.summaries.len(), 2);
        }
    }

    #[test]
    fn cycles_and_third_edges_remain_incomplete() {
        let unit = three_functions();
        for edges in [[Some(1), Some(0), None], [Some(1), Some(2), Some(2)]] {
            let mut session = synthetic_session(&unit, &edges, Limits::default());
            let assessment = session.assess(0).unwrap();
            assert_eq!(assessment.coverage, mutations::Coverage::Incomplete);
            assert!(session.summaries.is_empty());
        }
    }

    #[test]
    fn exhausted_call_and_summary_queries_publish_no_completed_outcome() {
        let unit = three_functions();
        for limits in [
            Limits {
                calls: 0,
                ..Limits::default()
            },
            Limits {
                summaries: 0,
                ..Limits::default()
            },
        ] {
            let mut session = synthetic_session(&unit, &[Some(1), Some(2), None], limits);
            let result: Prepared = session.query_result(1, QueryKind::Summary(1)).unwrap();
            assert!(result.budget);
            assert!(!session.completed.contains_key(&(1, QueryKind::Summary(1))));
            assert!(session.summaries.is_empty());
        }
    }

    #[test]
    fn cached_summaries_do_not_skip_inner_original_edge_context_or_path() {
        let unit = three_functions();
        let mut session = synthetic_session(&unit, &[Some(1), Some(2), None], Limits::default());
        assert!(session.leaf_summary(2).unwrap().summary.is_some());
        let mut context = nominal_boundary::tests::context();
        context.crate_name = "changed".into();
        session.contexts.insert(2, Availability::Known(context));
        assert!(session.prepare(1, 0, 0).is_err());
        session.check_contexts(1, 1).unwrap();
        session
            .observations
            .get_mut(&1)
            .unwrap()
            .as_mut()
            .unwrap()
            .as_mut()
            .unwrap()
            .calls[0]
            .target_path = Some("contradictory".into());
        assert!(session.prepare(1, 0, 0).is_err());
    }

    #[test]
    fn expanded_summary_cardinality_is_checked_before_copying_paths() {
        let unit = three_functions();
        let mut session = synthetic_session(&unit, &[None, None, None], Limits::default());
        session.summaries.push(Summary {
            function_index: 2,
            parameters: Vec::new(),
            sites: vec![SiteWitness {
                function_index: 2,
                site_index: usize::MAX,
                call_path: vec![CallReference {
                    function_index: usize::MAX,
                    call_index: usize::MAX,
                }],
                root: root(),
            }],
        });
        let events = [CallEvent {
            call_index: 0,
            span: span(),
            native: root().native,
            summary_index: 0,
            mappings: vec![Mapping {
                summary_site_index: 0,
                caller_root: root(),
            }],
        }];
        assert_eq!(session.wrapper_cost(1, &events).unwrap(), 3);
        session.limits.sites = 2;
        assert!(!session.summary_fits(3));
        assert_eq!(session.summaries.len(), 1);
    }
    #[test]
    fn wrapper_witnesses_preserve_occurrences_and_compose_reordered_formals() {
        let unit = three_functions();
        let mut session = synthetic_session(&unit, &[None, None, None], Limits::default());
        session.summaries.push(Summary {
            function_index: 2,
            parameters: vec![Parameter::MutableU32],
            sites: vec![SiteWitness {
                function_index: 2,
                site_index: 7,
                call_path: Vec::new(),
                root: root(),
            }],
        });
        let mut projected = root();
        projected.formal = Some(1);
        let events: Vec<_> = (0..2)
            .map(|call_index| CallEvent {
                call_index,
                span: span(),
                native: root().native,
                summary_index: 0,
                mappings: vec![Mapping {
                    summary_site_index: 0,
                    caller_root: projected.clone(),
                }],
            })
            .collect();
        let sites = session.wrapper_witnesses(1, &events).unwrap();
        assert_eq!(sites.len(), 2);
        assert_eq!(sites[0].function_index, 2);
        assert_eq!(sites[0].site_index, 7);
        assert_ne!(sites[0].call_path, sites[1].call_path);
        assert_eq!(sites[0].call_path[0].function_index, 1);
        let wrapper = Summary {
            function_index: 1,
            sites,
            parameters: vec![Parameter::MutableU32, Parameter::MutableU32],
        };
        let mut outer = call(0);
        outer.arguments.push(call(0).arguments.remove(0));
        let mut actual = root();
        actual.formal = Some(4);
        outer.arguments[1].value =
            Availability::Known(mutations::ArgumentValue::MutableFormal(actual));
        let mapped = mappings(&outer, &wrapper, &[0, 1]).unwrap().unwrap();
        assert!(
            mapped
                .iter()
                .all(|mapping| mapping.caller_root.formal == Some(4))
        );
    }
    #[test]
    fn wrapper_publication_requires_the_complete_retention_allowance() {
        let unit = three_functions();
        for sites in [1, 2] {
            let limits = Limits {
                sites,
                ..Limits::default()
            };
            let mut session = synthetic_session(&unit, &[None, Some(2), None], limits);
            let mut owned = root();
            owned.formal = None;
            owned.provenance = mutations::Provenance::LocalOwned;
            session
                .observations
                .get_mut(&1)
                .unwrap()
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()
                .direct
                .sites
                .push(mutations::WriteSite {
                    span: span(),
                    root: owned,
                    operation: mutations::Operation::Assignment,
                    caller_visible: false,
                });
            // The synthetic observation retains its original direct write before queries.
            session.work.retained_sites = 1;
            let prepared: Prepared = session.query_result(1, QueryKind::Summary(1)).unwrap();
            if sites == 1 {
                assert!(prepared.budget);
                assert!(prepared.summary.is_none());
                assert_eq!(session.summaries.len(), 1);
                assert_eq!(session.summaries[0].function_index, 2);
                assert!(!session.completed.contains_key(&(1, QueryKind::Summary(1))));
                assert_eq!(session.work.retained_sites, 1);
            } else {
                assert!(!prepared.budget);
                let summary = &session.summaries[prepared.summary.unwrap()];
                assert_eq!(summary.function_index, 1);
                assert_eq!(summary.sites.len(), 1);
                assert_eq!(summary.sites[0].function_index, 1);
                assert_eq!(summary.sites[0].site_index, 0);
                assert!(session.completed.contains_key(&(1, QueryKind::Summary(1))));
                assert_eq!(session.work.retained_sites, 2);
            }
        }
    }
}
