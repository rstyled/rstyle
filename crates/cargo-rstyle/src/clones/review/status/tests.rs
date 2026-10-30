use super::*;

fn record(state: State) -> Record {
    Record {
        family: Family::Helper,
        entry_index: 0,
        state,
        detail: "Independently assessed retained endpoints".into(),
        reason: "Keep these computations separate".into(),
        members: vec![Locator {
            package: super::super::Package {
                identity: "Cargo.toml".into(),
                name: "sample".into(),
                version: "0.1.0".into(),
            },
            source: "src/lib.rs".into(),
            qualified_name: "sample::first".into(),
            declaration_kind: "free-function".into(),
            discriminator: "unique-declaration".into(),
        }],
    }
}

#[test]
fn exact_growth_stale_context_and_completed_negative_are_assessed() {
    let states = [
        State::HiddenExact,
        State::HiddenGrowth,
        State::Stale,
        State::ContextChanged,
        State::NoFinding,
    ];
    let report = Report::new(states.into_iter().map(record).collect(), true);
    assert_eq!(report.execution, Execution::Completed);
    assert_eq!(report.coverage, Coverage::Complete);
    assert_eq!(report.retained_entries, 5);
    assert_eq!(report.assessed_entries, 5);
    assert_eq!(report.unassessed_entries, 0);
}

#[test]
fn omitted_unrequested_and_unavailable_each_make_status_incomplete() {
    for state in [State::NotObserved, State::NotRequested, State::Unavailable] {
        let report = Report::new(vec![record(State::HiddenExact), record(state)], true);
        assert_eq!(report.execution, Execution::Completed);
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.assessed_entries, 1);
        assert_eq!(report.unassessed_entries, 1);
    }
}

#[test]
fn mixed_families_keep_individual_omissions_and_original_reasons() {
    let helper = record(State::HiddenGrowth);
    let dispatch = Record {
        family: Family::EnumDispatch,
        entry_index: 0,
        ..record(State::NoFinding)
    };
    let capability = Record {
        family: Family::CapabilitySet,
        entry_index: 1,
        ..record(State::NotRequested)
    };
    let report = Report::new(vec![helper, dispatch, capability], true);
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(report.assessed_entries, 2);
    assert_eq!(report.records[2].state, State::NotRequested);
    assert_eq!(report.records[2].entry_index, 1);
    assert_eq!(report.records[2].reason, "Keep these computations separate");
}

#[test]
fn empty_requested_store_is_complete_and_unrequested_status_is_silent() {
    let empty = Report::new(Vec::new(), true);
    assert_eq!(empty.coverage, Coverage::Complete);
    assert_eq!(empty.retained_entries, 0);

    let unrequested = Report::new(vec![record(State::HiddenExact)], false);
    assert_eq!(unrequested.execution, Execution::NotRequested);
    assert_eq!(unrequested.coverage, Coverage::Incomplete);
    assert!(unrequested.render().is_empty());
}

#[test]
fn serialization_and_text_expose_status_without_rewriting_stored_choice() {
    let report = Report::new(vec![record(State::ContextChanged)], true);
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["schema"], "rstyle.review-status/1");
    assert_eq!(value["records"][0]["family"], "helper");
    assert_eq!(value["records"][0]["state"], "context_changed");
    assert_eq!(value["records"][0]["members"][0]["source"], "src/lib.rs");
    let text = report.render();
    assert!(text.contains("1 assessed, 0 unassessed; 1 retained entries"));
    assert!(text.contains("Keep these computations separate"));
}
