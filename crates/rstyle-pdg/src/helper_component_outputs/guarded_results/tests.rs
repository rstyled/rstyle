use super::*;

fn nominal() -> crate::concrete_types::Nominal {
    crate::concrete_types::Nominal {
        source_identity: Some("shape-only".into()),
        package_name: "fixture".into(),
        package_version: "1".into(),
        manifest_digest: "shape-only".into(),
        definition_path: "Reader".into(),
        declaration_digest: "shape-only".into(),
    }
}

fn result() -> TypeToken {
    TypeToken::Result {
        ok: Box::new(TypeToken::Struct {
            nominal: nominal(),
            fields: vec![
                (
                    "remaining".into(),
                    TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
                        "u8".into(),
                    ))))),
                ),
                (
                    "width".into(),
                    TypeToken::Option(Box::new(TypeToken::Scalar("usize".into()))),
                ),
            ],
        }),
        err: Box::new(TypeToken::FieldlessEnum {
            nominal: nominal(),
            variants: vec![("Invalid".into(), 0)],
        }),
    }
}

fn summary() -> Summary {
    let ty = result();
    let mut outputs = roster::leaves(&ty).unwrap();
    outputs[0].origins.push(Origin {
        parameter: 0,
        path: vec![Projection::SharedReferent],
    });
    Summary {
        identity: SUMMARY.into(),
        argument_types: vec![],
        result_type: ty.text().unwrap(),
        mutates_arguments: false,
        memory_effects: false,
        may_panic: false,
        always_returns: true,
        argument_evaluation: "source-order-once/1".into(),
        result_dependence: RESULT_DEPENDENCE.into(),
        bounded_loop: None,
        reference_outputs: None,
        component_outputs: Some(outputs),
        memory_reads: None,
    }
}

#[test]
fn guarded_roster_addresses_ok_payload_without_inventing_a_variant_component() {
    let ty = result();
    let leaves = roster::leaves(&ty).unwrap();
    assert_eq!(leaves.len(), 1);
    assert_eq!(leaves[0].selectors, vec![0]);
    assert!(roster::selected(&leaves, &[0, 0]).is_err());
    assert!(roster::selected(&leaves, &[1]).is_err());
    assert!(roster::leaves(&TypeToken::Tuple(vec![ty.clone()])).is_err());
    assert!(roster::leaves(&TypeToken::Option(Box::new(ty))).is_err());
}

#[test]
fn guarded_summary_preserves_saved_result_effects_and_nonempty_backing() {
    let valid = summary();
    verify_summary(&valid, &result()).unwrap();
    let changes: [fn(&mut Summary); 9] = [
        |s| s.identity = super::super::SUMMARY.into(),
        |s| s.result_dependence = "formal-origins-only".into(),
        |s| s.argument_evaluation = "unordered".into(),
        |s| s.mutates_arguments = true,
        |s| s.memory_effects = true,
        |s| s.may_panic = true,
        |s| s.always_returns = false,
        |s| s.component_outputs = None,
        |s| s.component_outputs.as_mut().unwrap()[0].origins.clear(),
    ];
    for change in changes {
        let mut invalid = valid.clone();
        change(&mut invalid);
        assert!(verify_summary(&invalid, &result()).is_err());
    }
}
