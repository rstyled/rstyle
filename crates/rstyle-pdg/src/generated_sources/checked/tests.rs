//! Hand-specified source, owner roster, and eight HIR roles; no verifier-generated fixtures.
use super::*;
use crate::nominal_boundary::census::{IDENTITY as CENSUS_IDENTITY, UnitFile};
use serde_json::json;
pub(crate) mod fixture;

#[test]
fn complete_source_member_and_eight_roles_join_without_callable_permission() {
    let fixture = fixture::Fixture::new();
    let checked = fixture.verify().unwrap();
    assert_eq!(checked.function().sig.ident, "new");
    assert_eq!(checked.namespace(), ["util", "primitives"]);
    assert_eq!(checked.implementation().items.len(), 16);
    assert_eq!(checked.expansion().items().len(), 2);
    assert_eq!(checked.roles().len(), 8);
    assert_eq!(checked.roles()[0].path(), [0, 0]);
    assert_eq!(checked.roles()[0].kind(), "pat_binding");
    assert_eq!(checked.roles()[0].hir().local, 2);
    assert_eq!(
        checked.roles()[7].source(),
        &fixture.span_in("map_err($err)", "map_err(", "$err")
    );
}

#[test]
fn missing_surplus_duplicate_and_reparented_members_fail_closed() {
    for mutation in 0..5 {
        let mut fixture = fixture::Fixture::new();
        match mutation {
            0 => {
                fixture.capture.emitted.pop();
            }
            1 => {
                fixture
                    .capture
                    .emitted
                    .push(fixture.capture.emitted[2].clone());
            }
            2 => fixture.capture.emitted[2]
                .parent
                .definition_identity
                .push('x'),
            3 => fixture.capture.emitted[2].definition.path.push('x'),
            _ => fixture.capture.emitted[2].kind = "AssocType".into(),
        }
        assert!(fixture.verify().is_err(), "mutation {mutation}");
    }
}

#[test]
fn source_and_exact_hygiene_changes_cannot_reuse_node_correspondence() {
    for mutation in 0..7 {
        let mut fixture = fixture::Fixture::new();
        let observation = fixture.observation_mut();
        match mutation {
            0 => observation.nodes[3].origin.raw.span.end -= 1,
            1 => observation.nodes[3].origin.marks.clear(),
            2 => observation.nodes[3].origin.marks[0].transparency = Transparency::Opaque,
            3 => observation.nodes[3].origin.marks[0].expansion = "StateID-expansion".into(),
            4 => observation.nodes[3].path.push(0),
            5 => {
                observation.nodes.pop();
            }
            _ => observation.nodes[3].hir.local += 1,
        }
        assert!(fixture.verify().is_err(), "mutation {mutation}");
    }
}

#[test]
fn exact_expansion_owner_and_header_normalization_cannot_be_replaced_by_containment() {
    for mutation in 0..7 {
        let mut fixture = fixture::Fixture::new();
        match mutation {
            0 => {
                fixture.capture.expansions[0]
                    .definition
                    .as_mut()
                    .unwrap()
                    .span
                    .end += 1
            }
            1 => fixture.capture.expansions[0].invocation.raw.span.end += 1,
            2 => fixture.capture.expansions[0].local_inner_macros = true,
            3 => fixture.capture.expansions[0].parent = Some("outer".into()),
            4 => fixture
                .observation_mut()
                .owner
                .definition_identity
                .push('x'),
            5 => fixture
                .observation_mut()
                .implementation
                .definition_identity
                .push('x'),
            _ => {
                fixture.capture.expansions[0]
                    .macro_definition
                    .as_mut()
                    .unwrap()
                    .path = "other::index_type_impls".into()
            }
        }
        assert!(fixture.verify().is_err(), "mutation {mutation}");
    }
}

#[test]
fn neighboring_unknown_body_remains_in_source_receipt_and_has_no_cleanup_certificate() {
    let mut fixture = fixture::Fixture::new();
    fixture.change_source(
        "pub fn iter() -> usize { 0 }",
        "pub fn iter() -> usize { unknown!() }",
    );
    let checked = fixture.verify().unwrap();
    let syn::ImplItem::Fn(neighbor) = &checked.implementation().items[15] else {
        panic!("Expected neighbor")
    };
    assert!(crate::source_evidence::closed_body(&neighbor.block).is_err());
    let mut selected = fixture::Fixture::new();
    selected.change_source(
        "SmallIndex::new(value).map($name).map_err($err)",
        "unknown!()",
    );
    assert!(selected.verify().is_err());
}

#[test]
fn incomplete_unrelated_observations_never_replace_complete_selected_member_roster() {
    let mut fixture = fixture::Fixture::new();
    fixture.capture.complete = false;
    for path in [
        "util::primitives::PatternIDIter::new",
        "util::primitives::StateIDIter::new",
    ] {
        let mut owner = fixture.observation_mut().owner.clone();
        owner.path = path.into();
        owner.definition_identity = format!("unsupported:{path}");
        fixture.capture.outcomes.push(Outcome::Unsupported {
            owner,
            reason: "Module expansion references a foreign source buffer".into(),
        });
    }
    let checked = fixture.verify().unwrap();
    assert_eq!(checked.owner().path, "util::primitives::PatternID::new");
    assert_eq!(checked.defining_expansion(), "PatternID-root");
    assert_eq!(fixture.capture.outcomes.len(), 3);
    assert!(!fixture.capture.complete);

    fixture.capture.emitted.pop();
    assert!(fixture.verify().is_err());
}

#[test]
fn checked_module_context_preserves_source_member_and_exact_roles() {
    let fixture = fixture::Fixture::new();
    let environment = checked_module_environment(&fixture);
    assert!(
        environment
            .generated_source_modules(&fixture.facts.build.cfgs)
            .is_some()
    );
    let standalone = fixture.verify().unwrap();
    let checked = verify_in(
        Input {
            facts: &fixture.facts,
            census: environment.census(),
            capture: &fixture.capture,
        },
        &environment,
    )
    .unwrap();
    assert_same_source_member(&checked, &standalone);
    let mut shifted = fixture.capture.clone();
    shifted.expansions[0].definition.as_mut().unwrap().span.end += 1;
    assert!(
        verify_in(
            Input {
                facts: &fixture.facts,
                census: environment.census(),
                capture: &shifted,
            },
            &environment,
        )
        .is_err()
    );
    assert!(
        verify_in(
            Input {
                facts: &fixture.facts,
                census: &fixture.census,
                capture: &fixture.capture,
            },
            &environment,
        )
        .is_err()
    );
}

fn checked_module_environment(
    fixture: &fixture::Fixture,
) -> crate::nominal_boundary::resolution::Environment {
    use crate::nominal_boundary::resolution::{Environment, SourceOptions};
    Environment::with_source_options(
        std::sync::Arc::new(fixture.census.clone()),
        SourceOptions {
            constants: true,
            cfgs: Some(&fixture.facts.build.cfgs),
            expansions: true,
            module_lints: true,
        },
    )
    .unwrap()
}

fn assert_same_source_member(actual: &CheckedMember, expected: &CheckedMember) {
    assert_eq!(actual.owner(), expected.owner());
    assert_eq!(
        actual.expansion().definition(),
        expected.expansion().definition()
    );
    assert_eq!(
        actual.expansion().invocation(),
        expected.expansion().invocation()
    );
    assert_eq!(
        actual.expansion().bindings(),
        expected.expansion().bindings()
    );
    for (actual, expected) in actual.roles().iter().zip(expected.roles()) {
        assert_eq!(actual.path(), expected.path());
        assert_eq!(actual.hir(), expected.hir());
        assert_eq!(actual.kind(), expected.kind());
        assert_eq!(actual.source(), expected.source());
    }
}

#[test]
fn different_configuration_or_module_proof_keeps_standalone_verification() {
    use crate::nominal_boundary::resolution::{Environment, SourceOptions};
    let mut fixture = fixture::Fixture::new();
    for module_lints in [false, true] {
        let environment = Environment::with_source_options(
            std::sync::Arc::new(fixture.census.clone()),
            SourceOptions {
                constants: true,
                cfgs: Some(&[]),
                expansions: true,
                module_lints,
            },
        )
        .unwrap();
        fixture.facts.build.cfgs = vec!["different_configuration".into()];
        assert!(
            environment
                .generated_source_modules(&fixture.facts.build.cfgs)
                .is_none()
        );
        if !module_lints {
            assert!(environment.generated_source_modules(&[]).is_none());
        }
        assert!(
            verify_in(
                Input {
                    facts: &fixture.facts,
                    census: environment.census(),
                    capture: &fixture.capture,
                },
                &environment,
            )
            .is_ok()
        );
    }
}

fn root_module_fixture() -> fixture::Fixture {
    let mut fixture = fixture::Fixture::new();
    fixture.change_source("mod util {\n    pub mod primitives {\n", "");
    fixture.change_source("\n    }\n}\n", "\n");
    let root_path = |definition: &mut Definition| {
        definition.path = definition
            .path
            .strip_prefix("util::primitives::")
            .unwrap_or("")
            .to_owned();
    };
    for emitted in &mut fixture.capture.emitted {
        root_path(&mut emitted.definition);
        root_path(&mut emitted.parent);
    }
    root_path(
        fixture.capture.expansions[0]
            .macro_definition
            .as_mut()
            .unwrap(),
    );
    let observation = fixture.observation_mut();
    root_path(&mut observation.owner);
    root_path(&mut observation.implementation);
    fixture.facts.name = "PatternID::new".into();
    fixture
}

#[test]
fn exact_crate_root_module_parent_joins_complete_member_roster() {
    let fixture = root_module_fixture();
    assert_eq!(fixture.capture.emitted[0].parent.path, "");
    let member = fixture.verify().unwrap();
    assert!(member.namespace().is_empty());
    assert_eq!(member.owner().path, "PatternID::new");
    assert_eq!(member.roles().len(), 8);
    assert!(fixture::Fixture::new().verify().is_ok());
}

#[test]
fn root_module_parent_exception_preserves_crate_identity_and_namespace() {
    for mutation in 0..5 {
        let mut fixture = root_module_fixture();
        let parent = &mut fixture.capture.emitted[0].parent;
        match mutation {
            0 => parent.crate_name = "foreign".into(),
            1 => parent.stable_crate_id = "foreign-stable-id".into(),
            2 => parent.local = false,
            3 => parent.definition_identity.clear(),
            _ => parent.path = "util::primitives".into(),
        }
        assert!(fixture.verify().is_err(), "mutation {mutation}");
    }
    let mut nonroot = fixture::Fixture::new();
    nonroot.capture.emitted[0].parent.path.clear();
    assert!(nonroot.verify().is_err());
}

#[test]
fn crate_root_module_parent_does_not_authorize_empty_callable_or_macro_paths() {
    for mutation in 0..3 {
        let mut fixture = root_module_fixture();
        match mutation {
            0 => fixture.observation_mut().owner.path.clear(),
            1 => fixture.observation_mut().implementation.path.clear(),
            _ => fixture.capture.expansions[0]
                .macro_definition
                .as_mut()
                .unwrap()
                .path
                .clear(),
        }
        assert!(fixture.verify().is_err(), "mutation {mutation}");
    }
}
