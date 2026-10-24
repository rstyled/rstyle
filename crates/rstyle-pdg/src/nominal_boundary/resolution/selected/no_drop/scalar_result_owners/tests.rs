//! Source receipts only: these fixtures establish no callable body or admission.
use super::super::product_fields::test_support as fixture;
use super::*;
use crate::concrete_types::{Field, Nominal};
use crate::{ByteSpan, ParamFact, ResolvedTy};

const FUNCTION: &str = "fn new() -> Result<Self, Problem> { Ok(Self(1)) }";
const IMPLEMENTATION: &str = "impl Product { fn new() -> Result<Self, Problem> { Ok(Self(1)) } }";

fn product(name: &str, named: bool) -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: None,
            package_name: "sample".into(),
            package_version: "0".into(),
            manifest_digest: "unverified".into(),
            definition_path: name.into(),
            declaration_digest: "unverified".into(),
        },
        source: if named {
            format!("struct {name} {{ attempted: u64 }}")
        } else {
            format!("struct {name}(u32);")
        }
        .into(),
        fields: vec![Field {
            position: 0,
            name: if named { "attempted" } else { "0" }.into(),
            declaration: if named { "u64" } else { "u32" }.into(),
            ty: TypeTree::Scalar(if named { "u64" } else { "u32" }.into()),
        }],
    }
}

fn result(ok: TypeTree, err: TypeTree) -> TypeTree {
    TypeTree::Result {
        ok: Box::new(ok),
        err: Box::new(err),
    }
}

fn interface(facts: &HirFnFacts) -> InterfaceObservations {
    facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap()
        .into_owned()
}

fn install_interface(facts: &mut HirFnFacts, interface: InterfaceObservations) {
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        serde_json::to_value(interface).unwrap(),
    );
}

fn ordinary(extra: &str, copy: bool) -> (HirFnFacts, Resolution) {
    let derive = if copy { "#[derive(Clone, Copy)] " } else { "" };
    let source = format!(
        "{derive}struct Product(u32);\nstruct Problem {{ attempted: u64 }}\n{IMPLEMENTATION}\n{extra}"
    );
    let mut success = product("Product", false);
    let mut error = product("Problem", true);
    let mut resolution = fixture::resolution(&source, &mut success);
    let errors = fixture::resolution(&source, &mut error);
    resolution.trees.extend(errors.trees);
    let mut facts = fixture::facts(&result(success.clone(), error));
    facts.name = "Product::new".into();
    let start = source.find(FUNCTION).unwrap() as u32;
    facts.fn_span = ByteSpan {
        start,
        end: start + FUNCTION.len() as u32,
    };
    facts.source.bytes = FUNCTION.into();
    facts.source.digest = facts.source.bytes.identity().into();
    facts.signature.has_self = false;
    facts.signature.params.clear();
    let observed = interface(&facts);
    let Observation::Captured { ty } = &observed.result else {
        unreachable!()
    };
    facts.signature.return_ty.normalized = Some(crate::data_types::token(ty).unwrap());
    facts.signature.return_ty.observed = "Result<Product, Problem>".into();
    facts.build.frontend_options.insert(
        "definition_identity".into(),
        "original-result-method".into(),
    );
    facts.build.frontend_options.insert(
        "rstyle.nominal_selected_unit".into(),
        serde_json::to_value(&resolution.environment.census().context).unwrap(),
    );
    let origins = fixture::origins(&resolution, &facts);
    facts.build.frontend_options.insert(
        crate::declaration_origins::KEY.into(),
        serde_json::to_value(origins).unwrap(),
    );
    install_observation(&mut facts, &source, &success);
    (facts, resolution)
}

fn install_observation(facts: &mut HirFnFacts, source: &str, success: &TypeTree) {
    let start = source.find(IMPLEMENTATION).unwrap() as u32;
    let observation = crate::inherent_calls::Definition {
        identity: crate::inherent_calls::IDENTITY.into(),
        definition_identity: "original-result-method".into(),
        implementation_identity: "original-result-impl".into(),
        implementation_span: ByteSpan {
            start,
            end: start + IMPLEMENTATION.len() as u32,
        },
        owner_nominal: success.nominal_parts().unwrap().0.clone(),
        source_file: "/sample/src/lib.rs".into(),
        function_span: facts.fn_span,
    };
    facts.build.frontend_options.insert(
        crate::inherent_calls::KEY.into(),
        serde_json::to_value(observation).unwrap(),
    );
}

fn scalar_parameter(facts: &mut HirFnFacts) {
    facts.signature.params.push(ParamFact {
        symbol: "index".into(),
        position: 0,
        name_span: ByteSpan { start: 0, end: 0 },
        ty_span: ByteSpan { start: 0, end: 0 },
        ty: ResolvedTy {
            spelling: "usize".into(),
            observed: "usize".into(),
            normalized: Some("usize".into()),
        },
        pat: facts.body.clone(),
    });
}

#[test]
fn shape_acceptance_requires_static_complete_scalar_parameters_and_named_scalar_error() {
    let (mut facts, _) = ordinary("", true);
    let mut observed = interface(&facts);
    assert!(source::interface(&facts, &observed).unwrap().is_some());
    scalar_parameter(&mut facts);
    observed.parameters.push(Observation::Captured {
        ty: TypeTree::Scalar("usize".into()),
    });
    assert!(source::interface(&facts, &observed).unwrap().is_some());
    facts.signature.has_self = true;
    assert!(source::interface(&facts, &observed).unwrap().is_none());
    facts.signature.has_self = false;
    observed.parameters.clear();
    assert!(source::interface(&facts, &observed).unwrap().is_none());
}

#[test]
fn shape_rejects_owned_result_error_reference_and_unknown_parameter_storage() {
    let (mut facts, _) = ordinary("", true);
    scalar_parameter(&mut facts);
    let original = interface(&facts);
    let Observation::Captured {
        ty: original_result,
    } = &original.result
    else {
        unreachable!()
    };
    let TypeTree::Result { err, .. } = original_result else {
        unreachable!()
    };
    for parameter in [
        original_result.clone(),
        err.as_ref().clone(),
        TypeTree::Shared(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Scalar("unit".into()),
    ] {
        let mut observed = original.clone();
        observed.parameters = vec![Observation::Captured { ty: parameter }];
        assert!(source::interface(&facts, &observed).unwrap().is_none());
    }
    let mut missing = original;
    missing.parameters = vec![Observation::Unsupported {
        reason: "not captured".into(),
    }];
    assert!(source::interface(&facts, &missing).unwrap().is_none());
}

#[test]
fn receipt_is_available_before_permission_selection_and_grants_no_body_authority() {
    let (facts, resolution) = ordinary("", true);
    assert!(resolution.copy_trees.is_empty());
    assert!(resolution.no_drop_trees.is_empty());
    let checked = capture(&facts, &resolution).unwrap().unwrap();
    assert!(checked.declaration(&checked.success).is_some());
    assert!(checked.declaration(&checked.error).is_some());
    assert!(!checked.trait_owner());
    assert_eq!(
        checked
            .checked_namespace(&facts, &resolution.environment)
            .unwrap(),
        &[] as &[String]
    );
    assert!(resolution.copy_trees.is_empty());
    assert!(resolution.no_drop_trees.is_empty());
    assert!(
        checked
            .function_namespace(&facts, &resolution.environment, &["other".into()])
            .unwrap()
            .is_none()
    );
}

#[test]
fn missing_original_observation_does_not_manufacture_a_source_owner() {
    let (mut facts, resolution) = ordinary("", true);
    facts
        .build
        .frontend_options
        .remove(crate::inherent_calls::KEY);
    assert!(capture(&facts, &resolution).unwrap().is_none());
}

#[test]
fn scalar_shape_does_not_supply_copy_or_hide_error_drop_and_unknown_neighbors() {
    let (facts, resolution) = ordinary("", false);
    assert!(
        source::interface(&facts, &interface(&facts))
            .unwrap()
            .is_some()
    );
    assert!(capture(&facts, &resolution).is_err());
    for extra in [
        "impl Drop for Problem { fn drop(&mut self) {} }",
        "fn nested() { impl Drop for Problem { fn drop(&mut self) {} } }",
        "macro_rules! cleanup { () => { impl Drop for Problem { fn drop(&mut self) {} } }; } cleanup!();",
        "unknown!(Problem);",
    ] {
        let (facts, resolution) = ordinary(extra, true);
        assert!(capture(&facts, &resolution).is_err(), "{extra}");
    }
}

#[test]
fn standalone_scalar_error_cannot_replace_original_owner_success() {
    let (mut facts, resolution) = ordinary("", true);
    let mut observed = interface(&facts);
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &mut observed.result
    else {
        unreachable!()
    };
    *ok = err.clone();
    install_interface(&mut facts, observed);
    assert!(capture(&facts, &resolution).is_err());
}

#[test]
fn retained_source_rejects_changed_bytes_interface_profile_observation_and_census() {
    let (facts, resolution) = ordinary("", true);
    let checked = capture(&facts, &resolution).unwrap().unwrap();
    for mutation in ["source", "interface", "profile", "observation"] {
        let mut changed = facts.clone();
        match mutation {
            "source" => changed.source.bytes = "fn new() -> Self { Self(2) }".into(),
            "interface" => {
                let mut observed = interface(&changed);
                observed.result = Observation::Captured {
                    ty: TypeTree::Scalar("u32".into()),
                };
                install_interface(&mut changed, observed);
            }
            "profile" => {
                changed
                    .build
                    .frontend_options
                    .insert("extraction_profile".into(), "hir-data-v2".into());
            }
            "observation" => {
                changed
                    .build
                    .frontend_options
                    .remove(crate::inherent_calls::KEY);
            }
            _ => unreachable!(),
        }
        assert!(
            checked
                .function_namespace(&changed, &resolution.environment, &[])
                .is_err(),
            "{mutation}"
        );
    }
    let (_, different) = ordinary("fn changed_neighbor() {}", true);
    assert!(
        checked
            .function_namespace(&facts, &different.environment, &[])
            .is_err()
    );
}

#[test]
fn non_result_and_unsupported_result_shapes_preserve_existing_routes() {
    let (mut facts, resolution) = ordinary("", true);
    let original = interface(&facts);
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &original.result
    else {
        unreachable!()
    };
    for unsupported in [
        ok.as_ref().clone(),
        err.as_ref().clone(),
        TypeTree::Scalar("u32".into()),
    ] {
        let mut changed = original.clone();
        changed.result = Observation::Captured { ty: unsupported };
        install_interface(&mut facts, changed);
        assert!(capture(&facts, &resolution).unwrap().is_none());
    }
    for unsupported in [
        result(TypeTree::Scalar("u32".into()), err.as_ref().clone()),
        result(ok.as_ref().clone(), TypeTree::Scalar("unit".into())),
        result(TypeTree::Shared(ok.clone()), err.as_ref().clone()),
    ] {
        let mut changed = original.clone();
        changed.result = Observation::Captured { ty: unsupported };
        install_interface(&mut facts, changed);
        assert!(!matches!(capture(&facts, &resolution), Ok(Some(_))));
    }
}

#[test]
fn cleanup_modes_do_not_reuse_copy_success_as_error_cleanup_authority() {
    let (facts, resolution) = ordinary("impl Drop for Problem { fn drop(&mut self) {} }", true);
    let observed = interface(&facts);
    let Some((success, error)) = source::interface(&facts, &observed).unwrap() else {
        panic!("closed scalar shapes")
    };
    let origins = facts.build.frontend_options.data::<Origins>().unwrap();
    // Copy-owner proof may inspect this same source for the unrelated Product.
    // It must not suppress the separate error's destructor check afterward.
    assert!(
        Proof::copy_owner(&facts, &resolution, &origins)
            .copy_receipts(success)
            .is_ok()
    );
    assert!(super::super::verify(&facts, error, &resolution.environment).is_err());
    assert!(capture(&facts, &resolution).is_err());
}
