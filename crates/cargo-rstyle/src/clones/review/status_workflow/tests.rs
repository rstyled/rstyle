use super::*;
use crate::clones::{capture, identity};

fn locator(name: &str) -> Locator {
    Locator {
        package: super::super::Package {
            identity: "Cargo.toml".into(),
            name: "sample".into(),
            version: "0.1.0".into(),
        },
        source: "src/lib.rs".into(),
        qualified_name: format!("sample::{name}"),
        declaration_kind: "free-function".into(),
        discriminator: "unique-declaration".into(),
    }
}

fn keys() -> Vec<FunctionKey> {
    let unit = capture::Unit {
        compiler_advisory_only: false,
        extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile::R0,
        target: capture::Target {
            package: "sample".into(),
            manifest_dir: "/sample".into(),
            package_context: None,
            source: "/sample/src/lib.rs".into(),
            name: "sample".into(),
            kind: vec!["lib".into()],
            test: false,
        },
        test: false,
        invocation: Vec::new(),
        directory: "/capture/sample".into(),
        artifacts: Vec::new(),
    };
    let units = [unit];
    let assigned = identity::assign(&units).unwrap();
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_str(include_str!(
        "../../../../../rstyle-pdg/src/nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    )).unwrap();
    let functions = (0..3)
        .map(|owner| {
            let mut function = facts.clone();
            function.body.hir.owner = owner;
            function
        })
        .collect::<Vec<_>>();
    identity::functions(assigned[0].context.id, &functions).unwrap()
}

fn observations(values: Vec<(&str, u8, FunctionKey)>) -> Observations<u8> {
    let mut members: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (name, context, key) in values {
        members
            .entry(locator(name))
            .or_default()
            .push(Observation { key, context });
    }
    Observations {
        members,
        failures: Vec::new(),
        truncated: false,
    }
}

fn state(result: Result<Vec<FunctionKey>, Assessment>) -> status::State {
    match result {
        Ok(_) => panic!("Expected an explicitly unassessed or changed retained entry"),
        Err(assessment) => assessment.state,
    }
}

#[test]
fn exact_context_wins_over_alternate_context_regardless_of_order() {
    let keys = keys();
    for alternate_first in [true, false] {
        let mut values = vec![("first", 1, keys[0]), ("first", 2, keys[1])];
        if alternate_first {
            values.reverse();
        }
        let observed = observations(values);
        let selected = observed.resolve([(&locator("first"), &1)].into_iter());
        assert!(matches!(selected, Ok(selected) if selected == vec![keys[0]]));
    }
}

#[test]
fn duplicate_exact_context_is_unavailable_even_with_alternatives() {
    let keys = keys();
    let observed = observations(vec![
        ("first", 1, keys[0]),
        ("first", 2, keys[1]),
        ("first", 1, keys[2]),
    ]);
    assert_eq!(
        state(observed.resolve([(&locator("first"), &1)].into_iter())),
        status::State::Unavailable
    );
}

#[test]
fn missing_member_takes_priority_over_changed_context_in_either_order() {
    let keys = keys();
    let observed = observations(vec![("changed", 2, keys[0])]);
    let changed = locator("changed");
    let missing = locator("missing");
    for members in [vec![&changed, &missing], vec![&missing, &changed]] {
        assert_eq!(
            state(observed.resolve(members.into_iter().map(|member| (member, &1)))),
            status::State::NotObserved
        );
    }
}

#[test]
fn incomplete_observations_cannot_establish_context_change_or_missing_scope() {
    let keys = keys();
    let mut observed = observations(vec![("first", 2, keys[0])]);
    observed
        .failures
        .push("Observation budget exhausted".into());
    for name in ["first", "missing"] {
        assert_eq!(
            state(observed.resolve([(&locator(name), &1)].into_iter())),
            status::State::Unavailable
        );
    }
    let exact = observed.resolve([(&locator("first"), &2)].into_iter());
    assert!(matches!(exact, Ok(selected) if selected == vec![keys[0]]));
}

#[test]
fn resolution_preserves_requested_membership_and_order() {
    let keys = keys();
    let observed = observations(vec![("first", 1, keys[0]), ("second", 1, keys[1])]);
    let first = locator("first");
    let second = locator("second");
    let selected = observed.resolve([(&second, &1), (&first, &1)].into_iter());
    assert!(matches!(selected, Ok(selected) if selected == vec![keys[1], keys[0]]));
    assert_eq!(
        state(observed.resolve([(&first, &2), (&second, &2)].into_iter())),
        status::State::ContextChanged
    );
}

#[test]
fn completed_negative_and_unassessed_detector_results_are_distinct() {
    assert_eq!(Assessment::negative(true).state, status::State::NoFinding);
    assert_eq!(
        Assessment::negative(false).state,
        status::State::Unavailable
    );
}

#[test]
fn truncated_index_cannot_prove_exact_endpoint_uniqueness() {
    let keys = keys();
    let mut observed = observations(vec![("first", 1, keys[0])]);
    observed.truncated = true;
    assert_eq!(
        state(observed.resolve([(&locator("first"), &1)].into_iter())),
        status::State::Unavailable
    );
}
