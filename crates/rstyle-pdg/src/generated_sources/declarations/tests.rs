//! Independent template, payload and emitted-roster expectations; no cleanup claim.
use super::*;
use crate::{
    concrete_types::{Field, Nominal},
    generated_sources::checked::tests::fixture::Fixture,
};

const DECLARATION: &str = "pub struct $err(SmallIndexError);";

#[test]
fn exact_generated_tuple_receipt_preserves_template_and_selected_constructor() {
    let (fixture, tree, origin) = fixture();
    let checked = check(&fixture, &tree, &origin).unwrap();
    assert_eq!(checked.tree(), &tree);
    assert_eq!(checked.item().ident, "PatternIDError");
    assert_eq!(checked.namespace(), ["util", "primitives"]);
    assert_eq!(
        checked.constructor().path,
        "util::primitives::PatternIDError"
    );
    assert_eq!(tree.generated_parts().unwrap().2.as_str(), DECLARATION);
    assert!(tree.nominal_parts().is_none());
}

#[test]
fn changed_template_root_payload_and_compiler_identity_are_rejected() {
    for mutation in 0..8 {
        let (fixture, mut tree, mut origin) = fixture();
        match mutation {
            0 => origin.defining_expansion = "StateID-root".into(),
            1 => origin.origin.marks.clear(),
            2 => origin.origin.raw.span.end -= 1,
            3 => origin.file_bytes = "other source".into(),
            4 => origin.definition.definition_identity.push('x'),
            5 => origin.nominal.declaration_digest.push('x'),
            6 => {
                let TypeTree::GeneratedStruct { fields, .. } = &mut tree else {
                    panic!()
                };
                let TypeTree::Struct { nominal, .. } = &mut fields[0].ty else {
                    panic!()
                };
                nominal.definition_path = "util::primitives::OtherError".into();
            }
            _ => {
                let TypeTree::GeneratedStruct { fields, .. } = &mut tree else {
                    panic!()
                };
                fields[0].declaration = "OtherError".into();
            }
        }
        assert!(
            check(&fixture, &tree, &origin).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn missing_surplus_reparented_and_relocated_direct_definitions_are_rejected() {
    for mutation in 0..7 {
        let (mut fixture, tree, origin) = fixture();
        let last = fixture.capture.emitted.len() - 1;
        match mutation {
            0 => {
                fixture.capture.emitted.pop();
            }
            1 => fixture
                .capture
                .emitted
                .push(fixture.capture.emitted[last].clone()),
            2 => fixture.capture.emitted[last]
                .parent
                .definition_identity
                .push('x'),
            3 => fixture.capture.emitted[last].origin.raw.span.start += 1,
            4 => fixture.capture.emitted[last - 1].kind = "AssocFn".into(),
            5 => {
                fixture.capture.emitted[last - 1].origin.marks[0].transparency =
                    Transparency::Opaque
            }
            _ => fixture.capture.emitted[last - 2]
                .parent
                .definition_identity
                .push('x'),
        }
        assert!(
            check(&fixture, &tree, &origin).is_err(),
            "mutation {mutation}"
        );
    }
}

fn check(
    fixture: &Fixture,
    tree: &TypeTree,
    origin: &GeneratedOrigin,
) -> Result<CheckedGeneratedDeclaration, String> {
    let member = fixture.verify()?;
    verify(Input {
        facts: &fixture.facts,
        census: &fixture.census,
        capture: &fixture.capture,
        member: &member,
        tree,
        origin,
    })
}

fn fixture() -> (Fixture, TypeTree, GeneratedOrigin) {
    from_fixture(Fixture::new())
}

fn from_fixture(mut fixture: Fixture) -> (Fixture, TypeTree, GeneratedOrigin) {
    let source = &fixture.census.files[0];
    let raw = fixture.span(DECLARATION);
    let template = crate::SourceText::from_snapshot(source.bytes.clone(), raw.span).unwrap();
    let payload_span = fixture.span("SmallIndexError").span;
    let field = crate::SourceText::from_snapshot(source.bytes.clone(), payload_span).unwrap();
    let root_definition = definition("util::primitives::PatternIDError", "struct:error");
    let root_nominal = nominal("util::primitives::PatternIDError", &template);
    let origin = GeneratedOrigin {
        nominal: root_nominal.clone(),
        definition: root_definition.clone(),
        origin: marked(raw),
        defining_expansion: "PatternID-root".into(),
        file_bytes: source.bytes.clone(),
    };
    let tree = TypeTree::GeneratedStruct {
        nominal: root_nominal,
        template,
        definition: Box::new(root_definition.clone()),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: field,
            ty: payload(),
        }],
    };

    emitted(&mut fixture, root_definition, &origin);
    (fixture, tree, origin)
}

fn payload() -> TypeTree {
    let source: crate::SourceText = "struct SmallIndexError { attempted: u64 }".into();
    TypeTree::Struct {
        nominal: nominal("util::primitives::SmallIndexError", &source),
        source,
        fields: vec![Field {
            position: 0,
            name: "attempted".into(),
            declaration: "u64".into(),
            ty: TypeTree::Scalar("u64".into()),
        }],
    }
}

fn emitted(fixture: &mut Fixture, root_definition: Definition, origin: &GeneratedOrigin) {
    let field_origin = marked(fixture.span("SmallIndexError"));
    fixture.capture.emitted.extend([
        Emitted {
            definition: root_definition.clone(),
            parent: definition("util::primitives", "test-definition:util::primitives"),
            kind: "Struct".into(),
            origin: origin.origin.clone(),
        },
        Emitted {
            definition: definition("util::primitives::PatternIDError", "ctor:error"),
            parent: root_definition.clone(),
            kind: "Ctor(Struct, Fn)".into(),
            origin: origin.origin.clone(),
        },
        Emitted {
            definition: definition("util::primitives::PatternIDError::0", "field:error:0"),
            parent: root_definition,
            kind: "Field".into(),
            origin: field_origin,
        },
    ]);
}

fn marked(raw: crate::nominal_boundary::census::expansions::FileSpan) -> Origin {
    Origin {
        raw,
        marks: vec![Mark {
            expansion: "PatternID-root".into(),
            transparency: Transparency::SemiOpaque,
        }],
    }
}

fn definition(path: &str, identity: &str) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: identity.into(),
        crate_name: "sample".into(),
        stable_crate_id: "test-sample-crate".into(),
        local: true,
    }
}

fn nominal(path: &str, source: &crate::SourceText) -> Nominal {
    Nominal {
        source_identity: Some("fixture".into()),
        package_name: "sample".into(),
        package_version: "1.0.0".into(),
        manifest_digest: "fixture".into(),
        definition_path: path.into(),
        declaration_digest: source.identity().into(),
    }
}

fn independent_check(
    fixture: &Fixture,
    tree: &TypeTree,
    origin: &GeneratedOrigin,
) -> Result<CheckedGeneratedDeclaration, String> {
    independent::verify_using(
        IndependentInput {
            facts: &fixture.facts,
            capture: &fixture.capture,
            tree,
            origin,
        },
        &fixture.census,
        crate::nominal_boundary::census::generated_sources::verify_compiler,
    )
}

#[test]
fn ordinary_caller_needs_no_generated_function_observation_or_global_completeness() {
    let (mut fixture, tree, origin) = fixture();
    fixture.capture.outcomes.clear();
    fixture.capture.complete = false;
    fixture.facts.name = "wire::arithmetic_overflow".into();
    fixture
        .facts
        .build
        .frontend_options
        .remove("definition_identity");
    let receipt = independent_check(&fixture, &tree, &origin).unwrap();
    assert_eq!(receipt.tree(), &tree);
    assert_eq!(receipt.constructor().definition_identity, "ctor:error");
    assert!(check(&fixture, &tree, &origin).is_err());
}

#[test]
fn independent_declaration_checks_exact_root() {
    for mutation in 0..11 {
        let (mut fixture, tree, mut origin) = fixture();
        match mutation {
            0 => origin.defining_expansion = "StateID-root".into(),
            1 => fixture.capture.expansions[0].parent = Some("outer".into()),
            2 => fixture.capture.expansions[0].edition = "2018".into(),
            3 => fixture.capture.expansions[0].invocation.marks = origin.origin.marks.clone(),
            4 => fixture.capture.expansions[0]
                .macro_definition
                .as_mut()
                .unwrap()
                .path
                .push('x'),
            5 => fixture.capture.expansions[0]
                .macro_definition
                .as_mut()
                .unwrap()
                .stable_crate_id
                .clear(),
            6 => {
                fixture.capture.expansions[0]
                    .definition
                    .as_mut()
                    .unwrap()
                    .span
                    .end -= 1
            }
            7 => fixture.capture.expansions[0].invocation.raw.span.end -= 1,
            8 => fixture
                .capture
                .expansions
                .push(fixture.capture.expansions[0].clone()),
            9 => fixture.capture.expansions[0].local_inner_macros = true,
            _ => fixture.capture.expansions[0].kind = ExpansionKind::Derive,
        }
        assert!(
            independent_check(&fixture, &tree, &origin).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn independent_declaration_checks_module_and_complete_direct_roster() {
    for mutation in 11..17 {
        let (mut fixture, tree, origin) = fixture();
        match mutation {
            11 => fixture
                .capture
                .emitted
                .last_mut()
                .unwrap()
                .parent
                .definition_identity
                .push('x'),
            12 => {
                fixture.capture.emitted.pop();
            }
            13 => fixture
                .capture
                .emitted
                .push(fixture.capture.emitted.last().unwrap().clone()),
            14 => {
                let len = fixture.capture.emitted.len();
                fixture.capture.emitted[len - 3]
                    .parent
                    .stable_crate_id
                    .clear();
            }
            15 => {
                let len = fixture.capture.emitted.len();
                fixture.capture.emitted[len - 3].parent.local = false;
            }
            _ => {
                let len = fixture.capture.emitted.len();
                fixture.capture.emitted[len - 3]
                    .parent
                    .definition_identity
                    .push('x');
            }
        }
        assert!(
            independent_check(&fixture, &tree, &origin).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn source_inactive_declaration_cannot_be_revived_by_an_emitted_row() {
    let source = include_str!("../checked/tests/source.rs").replace(
        DECLARATION,
        &format!("#[cfg(not(feature = \"hide_error\"))] {DECLARATION}"),
    );
    let (mut fixture, tree, origin) = from_fixture(Fixture::with_source(&source));
    independent_check(&fixture, &tree, &origin).unwrap();
    fixture
        .facts
        .build
        .cfgs
        .push("feature=\"hide_error\"".into());
    assert!(
        matches!(independent_check(&fixture, &tree, &origin), Err(error) if error.contains("inactive"))
    );
}
