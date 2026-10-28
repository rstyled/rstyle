use super::{
    super::{Availability, mutations},
    engine::{ResolvedCall, assessment, leaf_complete, mappings},
    *,
};
use crate::{ByteSpan, HirRef, unit_store::UnitStore};

fn span(start: u32, end: u32) -> ByteSpan {
    ByteSpan { start, end }
}

fn root(formal: Option<u32>, provenance: mutations::Provenance) -> mutations::Root {
    mutations::Root {
        declaration_span: span(1, 2),
        scope_span: span(0, 100),
        native: HirRef {
            krate: 0,
            owner: 1,
            local: formal.unwrap_or(20) + 1,
        },
        formal,
        provenance,
    }
}

fn write(formal: Option<u32>, provenance: mutations::Provenance) -> mutations::WriteSite {
    mutations::WriteSite {
        span: span(30, 40),
        root: root(formal, provenance),
        operation: mutations::Operation::Assignment,
        caller_visible: provenance == mutations::Provenance::FormalBorrowed,
    }
}

fn direct(sites: Vec<mutations::WriteSite>) -> mutations::Assessment {
    mutations::Assessment {
        availability: Availability::Known(()),
        coverage: mutations::Coverage::Complete,
        execution: mutations::Execution::Completed,
        sites,
        boundaries: Vec::new(),
    }
}

fn call(boundary_index: usize) -> mutations::Call {
    mutations::Call {
        span: span(
            50 + boundary_index as u32 * 10,
            55 + boundary_index as u32 * 10,
        ),
        native: HirRef {
            krate: 0,
            owner: 2,
            local: boundary_index as u32 + 1,
        },
        boundary_index,
        target: Availability::Known("exact-local-definition".into()),
        target_path: Some("exact-local-definition".into()),
        arguments: vec![mutations::Argument {
            unchanged_formal: Availability::Known(None),
            formal: Availability::Known(Some(0)),
            span: span(51, 52),
            value: Availability::Known(mutations::ArgumentValue::MutableFormal(root(
                Some(3),
                mutations::Provenance::FormalBorrowed,
            ))),
        }],
    }
}

fn observed(calls: Vec<mutations::Call>) -> mutations::Observations {
    let boundaries = calls
        .iter()
        .map(|call| mutations::Boundary {
            span: call.span,
            availability: mutations::BoundaryAvailability::Unknown,
            reason: "unresolved call".into(),
        })
        .collect();
    mutations::Observations {
        uses: mutations::UseInventory::unrequested(),
        direct: mutations::Assessment {
            boundaries,
            coverage: mutations::Coverage::Incomplete,
            ..direct(Vec::new())
        },
        calls,
        signature: Availability::Known(()),
    }
}

fn event(call_index: usize, mapping: Vec<Mapping>) -> ResolvedCall {
    ResolvedCall {
        event: Some(CallEvent {
            call_index,
            span: call(call_index).span,
            native: call(call_index).native,
            summary_index: 0,
            mappings: mapping,
        }),
        reason: None,
        budget: false,
    }
}

#[test]
fn borrowed_leaf_sites_map_to_actual_caller_formal_and_owned_sites_stay_local() {
    let direct = direct(vec![
        write(Some(0), mutations::Provenance::FormalBorrowed),
        write(None, mutations::Provenance::LocalOwned),
    ]);
    let summary = Summary {
        function_index: 1,
        sites: witnesses(&direct),
        parameters: vec![Parameter::MutableU32],
    };
    let mapped = mappings(&call(0), &summary, &[0]).unwrap().unwrap();
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].summary_site_index, 0);
    assert_eq!(mapped[0].caller_root.formal, Some(3));
}

#[test]
fn missing_and_changed_argument_roles_cannot_complete_a_call() {
    let direct = direct(vec![write(Some(0), mutations::Provenance::FormalBorrowed)]);
    let summary = Summary {
        function_index: 1,
        sites: witnesses(&direct),
        parameters: vec![Parameter::MutableU32],
    };
    let mut missing = call(0);
    missing.arguments.clear();
    assert!(mappings(&missing, &summary, &[0]).unwrap().is_none());
    let mut changed = call(0);
    changed.arguments[0].value = Availability::Known(mutations::ArgumentValue::OwnedU32);
    assert!(mappings(&changed, &summary, &[0]).unwrap().is_none());
}

#[test]
fn every_original_call_occurrence_keeps_its_event_and_shared_summary_index() {
    let observed = observed(vec![call(0), call(1)]);
    let mapping = || {
        vec![Mapping {
            summary_site_index: 0,
            caller_root: root(Some(3), mutations::Provenance::FormalBorrowed),
        }]
    };
    let result = assessment(0, &observed, vec![event(0, mapping()), event(1, mapping())]).unwrap();
    assert_eq!(result.calls.len(), 2);
    assert_eq!(result.calls[0].summary_index, result.calls[1].summary_index);
    assert_ne!(result.calls[0].native, result.calls[1].native);
    assert_eq!(result.roots.len(), 1);
    assert_eq!(result.coverage, mutations::Coverage::Complete);
    assert_eq!(
        result.caller_visible_mutation,
        CallerVisibleMutation::WritesFormal
    );
}

#[test]
fn unrelated_unknown_boundary_and_callback_signature_preserve_positive_witness() {
    let mut observed = observed(vec![call(0)]);
    observed.signature = Availability::Unsupported {
        reason: "callback formal".into(),
    };
    observed.direct.boundaries.push(mutations::Boundary {
        span: span(80, 90),
        availability: mutations::BoundaryAvailability::Unsupported,
        reason: "opaque callback".into(),
    });
    let mapping = vec![Mapping {
        summary_site_index: 0,
        caller_root: root(Some(3), mutations::Provenance::FormalBorrowed),
    }];
    let result = assessment(0, &observed, vec![event(0, mapping)]).unwrap();
    assert_eq!(result.unresolved_boundaries, [1]);
    assert_eq!(result.coverage, mutations::Coverage::Incomplete);
    assert_eq!(
        result.caller_visible_mutation,
        CallerVisibleMutation::WitnessedWritesFormalWithUnknownRemainder
    );
}

#[test]
fn empty_complete_leaf_proves_only_absence_of_bounded_writes() {
    let observed = mutations::Observations {
        uses: mutations::UseInventory::unrequested(),
        direct: direct(Vec::new()),
        calls: Vec::new(),
        signature: Availability::Known(()),
    };
    assert!(leaf_complete(&observed));
    let result = assessment(0, &observed, Vec::new()).unwrap();
    assert_eq!(
        result.caller_visible_mutation,
        CallerVisibleMutation::NoneProven
    );
}

#[test]
fn nonleaf_and_unsupported_signatures_cannot_mint_leaf_summaries() {
    let mut observed = observed(vec![call(0)]);
    assert!(!leaf_complete(&observed));
    observed.calls.clear();
    observed.direct = direct(Vec::new());
    observed.signature = Availability::Unknown {
        reason: "missing type observation".into(),
    };
    assert!(!leaf_complete(&observed));
}

#[test]
fn mapping_budget_does_not_erase_an_independent_known_direct_write() {
    let mut observed = observed(vec![call(0)]);
    observed
        .direct
        .sites
        .push(write(Some(2), mutations::Provenance::FormalBorrowed));
    let outcome = ResolvedCall {
        event: None,
        reason: Some("mapping budget exhausted".into()),
        budget: true,
    };
    let result = assessment(0, &observed, vec![outcome]).unwrap();
    assert_eq!(result.direct_site_count, 1);
    assert_eq!(result.execution, mutations::Execution::BudgetExhausted);
    assert_eq!(
        result.caller_visible_mutation,
        CallerVisibleMutation::WitnessedWritesFormalWithUnknownRemainder
    );
}

pub(super) fn unavailable_unit() -> UnitStore {
    let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options = Default::default();
    UnitStore::capture(vec![facts]).unwrap()
}

#[test]
fn completed_unavailable_observation_is_reused_within_owning_session() {
    let unit = unavailable_unit();
    let mut session = Session::new(&unit, Limits::default()).unwrap();
    let first = session.assess(0).unwrap();
    let second = session.assess(0).unwrap();
    assert_eq!(first.coverage, mutations::Coverage::Incomplete);
    assert_eq!(second.coverage, first.coverage);
    assert_eq!(session.statistics().observations, 1);
    assert!(session.summaries().is_empty());
}

#[test]
fn owner_budget_is_explicit_and_does_not_inspect_an_omitted_owner() {
    let unit = unavailable_unit();
    let mut session = Session::new(
        &unit,
        Limits {
            functions: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    let result = session.assess(0).unwrap();
    assert_eq!(result.execution, mutations::Execution::BudgetExhausted);
    assert_eq!(
        result.caller_visible_mutation,
        CallerVisibleMutation::Unknown
    );
    assert_eq!(session.statistics().observations, 0);
}

#[test]
fn corrupt_summary_site_and_contradictory_root_metadata_fail() {
    let summary = Summary {
        function_index: 1,
        sites: Vec::new(),
        parameters: vec![Parameter::MutableU32],
    };
    assert!(mappings(&call(0), &summary, &[99]).is_err());
    let mut observed = observed(vec![call(0)]);
    observed
        .direct
        .sites
        .push(write(Some(3), mutations::Provenance::FormalBorrowed));
    let mut changed = root(Some(3), mutations::Provenance::FormalBorrowed);
    changed.scope_span = span(0, 101);
    let result = assessment(
        0,
        &observed,
        vec![event(
            0,
            vec![Mapping {
                summary_site_index: 0,
                caller_root: changed,
            }],
        )],
    );
    assert!(result.is_err());
}

fn witnesses(direct: &mutations::Assessment) -> Vec<SiteWitness> {
    direct
        .sites
        .iter()
        .enumerate()
        .map(|(site_index, site)| SiteWitness {
            function_index: 1,
            site_index,
            call_path: Vec::new(),
            root: site.root.clone(),
        })
        .collect()
}
