use super::*;
use crate::concrete_types::{InterfaceObservations, Observation, TypeTree};

fn facts() -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options = Default::default();
    facts.signature.return_ty.normalized = Some("unit".into());
    facts.body.children.clear();
    facts.body.ty = None;
    facts.body.res = None;
    facts.body.dispatch = None;
    facts.body.kind = "expr_block".into();
    facts
        .build
        .frontend_options
        .insert_data(InterfaceObservations {
            identity: crate::concrete_types::IDENTITY.into(),
            parameters: Vec::new(),
            result: Observation::Captured {
                ty: TypeTree::Scalar("unit".into()),
            },
        });
    facts
        .build
        .frontend_options
        .insert_data(crate::field_locations::FieldObservations {
            identity: crate::field_locations::IDENTITY.into(),
            operations: Vec::new(),
            possibly_aliasing_formals: Vec::new(),
            scalar_formal_accesses: None,
        });
    facts
}

#[test]
fn frontend_repeated_success_matches_raw_boundary_in_both_policies() {
    let facts = facts();
    let analyses = SourceAnalyses::default();
    let analysis = analyses.function(&facts);
    let check = || {
        assert_eq!(facts.verify_data_frontend(), Ok(()));
        for _ in 0..2 {
            assert_eq!(analysis.verify_data_frontend(), Ok(()));
        }
        assert!(analysis.scalar_instance().is_none());
        assert!(analysis.product_instance().is_none());
        assert!(analysis.reference_instance().is_none());
    };
    crate::verification_policy::strict(check);
    crate::verification_policy::generated(check);
    crate::verification_policy::strict(check);
}

#[test]
fn frontend_fresh_analysis_rejects_edited_owner_after_success() {
    let mut facts = facts();
    let analyses = SourceAnalyses::default();
    {
        let analysis = analyses.function(&facts);
        analysis.verify_data_frontend().unwrap();
    }
    facts.signature.return_ty.normalized = Some("u32".into());
    let edited = analyses.function(&facts);
    let check = || {
        let raw = facts.verify_data_frontend();
        assert!(raw.is_err());
        assert_eq!(edited.verify_data_frontend(), raw);
    };
    crate::verification_policy::generated(check);
    crate::verification_policy::strict(check);
    let fresh = SourceAnalyses::default();
    assert_eq!(
        fresh.function(&facts).verify_data_frontend(),
        facts.verify_data_frontend()
    );
}
