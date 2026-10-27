//! Raw record constraints are conditional on the separately checked source request.
use super::*;
use crate::concrete_types::{Field, Nominal};

fn product(name: &str, field: &str, ty: TypeTree) -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: Some("test-package".into()),
            package_name: "sample".into(),
            package_version: "1".into(),
            manifest_digest: "test-manifest".into(),
            definition_path: name.into(),
            declaration_digest: "test-declaration".into(),
        },
        source: format!("pub struct {name};").into(),
        fields: vec![Field {
            position: 0,
            name: field.into(),
            declaration: "test field".into(),
            ty,
        }],
    }
}

fn results() -> [TypeTree; 3] {
    let success = product("SmallIndex", "0", TypeTree::Scalar("u32".into()));
    let error = product(
        "SmallIndexError",
        "attempted",
        TypeTree::Scalar("u64".into()),
    );
    let wrapper = product("PatternID", "0", success.clone());
    let wrapped_error = product("PatternIDError", "0", error.clone());
    [
        TypeTree::Result {
            ok: Box::new(success),
            err: Box::new(error.clone()),
        },
        TypeTree::Result {
            ok: Box::new(wrapper.clone()),
            err: Box::new(error),
        },
        TypeTree::Result {
            ok: Box::new(wrapper),
            err: Box::new(wrapped_error),
        },
    ]
}

fn captured(ty: &TypeTree) -> Observation {
    Observation::Captured { ty: ty.clone() }
}

fn method(input: &TypeTree, output: &TypeTree) -> Method {
    Method {
        call: HirRef {
            krate: 0,
            owner: 10,
            local: 4,
        },
        source_span: ByteSpan { start: 0, end: 78 },
        receiver: HirRef {
            krate: 0,
            owner: 10,
            local: 6,
        },
        receiver_type: captured(input),
        adjusted_type: captured(input),
        adjustments: Vec::new(),
        signature_inputs: vec![
            captured(input),
            Observation::Unsupported {
                reason: "FnDef callback outside concrete type grammar".into(),
            },
        ],
        signature_result: captured(output),
    }
}

#[test]
fn constructor_source_route_requires_both_exact_result_stage_types() {
    let [r0, r1, r2] = results();
    assert!(record_types(&method(&r0, &r1), [&r0, &r1]).is_ok());
    assert!(record_types(&method(&r1, &r2), [&r1, &r2]).is_ok());
    assert!(record_types(&method(&r0, &r1), [&r1, &r2]).is_err());
}

#[test]
fn unsupported_receiver_callback_substitution_and_any_adjustment_are_rejected() {
    let [r0, r1, r2] = results();
    for mutation in 0..9 {
        let mut record = method(&r0, &r1);
        match mutation {
            0 => {
                record.receiver_type = Observation::Unsupported {
                    reason: "unknown receiver".into(),
                }
            }
            1 => record.adjusted_type = captured(&r1),
            2 => record.signature_result = captured(&r2),
            3 => record.signature_inputs[0] = captured(&r1),
            4 => record.signature_inputs[1] = captured(&TypeTree::Scalar("usize".into())),
            5 => {
                record.signature_inputs.pop();
            }
            6 => record.signature_inputs.push(captured(&r0)),
            7 => record.adjustments.push(Adjustment {
                kind: Kind::SharedBorrow,
                target: captured(&r0),
            }),
            _ => {
                record.signature_result = Observation::Unsupported {
                    reason: "unknown result".into(),
                }
            }
        }
        assert!(
            record_types(&record, [&r0, &r1]).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn constructor_callback_retains_unsupported_type_observation() {
    let [r0, r1, _] = results();
    let record = method(&r0, &r1);
    assert!(matches!(
        &record.signature_inputs[1],
        Observation::Unsupported { .. }
    ));
    crate::concrete_types::validate_observations(record.signature_inputs[1..].iter()).unwrap();
}
