use super::super::product_fields::test_support as fixture;
use super::*;
use crate::concrete_types::{Field, Nominal};
use crate::{ByteSpan, ParamFact, ResolvedTy};

fn tree() -> TypeTree {
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: None,
            package_name: "sample".into(),
            package_version: "0".into(),
            manifest_digest: "unverified".into(),
            definition_path: "Product".into(),
            declaration_digest: "unverified".into(),
        },
        source: "struct Product(u32);".into(),
        fields: vec![Field {
            position: 0,
            name: "0".into(),
            declaration: "u32".into(),
            ty: TypeTree::Scalar("u32".into()),
        }],
    }
}

fn ordinary(extra: &str, getter: bool) -> (HirFnFacts, Resolution) {
    let function = if getter {
        "fn read(&self) -> u32 { self.0 }"
    } else {
        "fn new() -> Self { Self(1) }"
    };
    let implementation = format!("impl Product {{ {function} }}");
    let source = format!("#[derive(Clone, Copy)] struct Product(u32);\n{implementation}\n{extra}");
    let mut root = tree();
    let resolution = fixture::resolution(&source, &mut root);
    let mut facts = fixture::facts(&root);
    facts.name = format!("Product::{}", if getter { "read" } else { "new" });
    let start = source.find(function).unwrap() as u32;
    facts.fn_span = ByteSpan {
        start,
        end: start + function.len() as u32,
    };
    facts.source.bytes = function.into();
    facts.source.digest = facts.source.bytes.identity().into();
    facts.signature.has_self = getter;
    if getter {
        install_receiver(&mut facts, &root);
    }
    let origins = fixture::origins(&resolution, &facts);
    facts.build.frontend_options.insert(
        crate::declaration_origins::KEY.into(),
        serde_json::to_value(origins).unwrap(),
    );
    facts
        .build
        .frontend_options
        .insert("definition_identity".into(), "original-method".into());
    facts.build.frontend_options.insert(
        "rstyle.nominal_selected_unit".into(),
        serde_json::to_value(&resolution.environment.census().context).unwrap(),
    );
    install_observation(&mut facts, &source, &implementation, &root);
    (facts, resolution)
}

fn install_observation(
    facts: &mut HirFnFacts,
    source: &str,
    implementation: &str,
    root: &TypeTree,
) {
    let start = source.find(implementation).unwrap() as u32;
    let observation = crate::inherent_calls::Definition {
        identity: crate::inherent_calls::IDENTITY.into(),
        definition_identity: "original-method".into(),
        implementation_identity: "original-impl".into(),
        implementation_span: ByteSpan {
            start,
            end: start + implementation.len() as u32,
        },
        owner_nominal: root.nominal_parts().unwrap().0.clone(),
        source_file: "/sample/src/lib.rs".into(),
        function_span: facts.fn_span,
    };
    facts.build.frontend_options.insert(
        crate::inherent_calls::KEY.into(),
        serde_json::to_value(observation).unwrap(),
    );
}

fn install_receiver(facts: &mut HirFnFacts, root: &TypeTree) {
    let shared = TypeTree::Shared(Box::new(root.clone()));
    facts.signature.params.push(ParamFact {
        symbol: "self".into(),
        position: 0,
        name_span: ByteSpan { start: 0, end: 0 },
        ty_span: ByteSpan { start: 0, end: 0 },
        ty: ResolvedTy {
            spelling: "&Self".into(),
            observed: "&Product".into(),
            normalized: Some(crate::data_types::token(&shared).unwrap()),
        },
        pat: facts.body.clone(),
    });
    let interface = InterfaceObservations {
        identity: concrete_types::IDENTITY.into(),
        parameters: vec![Observation::Captured { ty: shared }],
        result: Observation::Captured {
            ty: TypeTree::Scalar("u32".into()),
        },
    };
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        serde_json::to_value(interface).unwrap(),
    );
}

const NEIGHBOR: &str = "macro_rules! neighbor { ($owner:ident) => { impl $owner { fn extra() {} } }; } neighbor!(Product);";

#[test]
fn ordinary_constructor_and_shared_scalar_getter_retain_complete_source_receipts() {
    for getter in [false, true] {
        let (facts, resolution) = ordinary(NEIGHBOR, getter);
        let checked = capture(&facts, &resolution).unwrap().unwrap();
        assert!(
            checked
                .function_namespace(&facts, &resolution.environment, &[])
                .unwrap()
                .is_some()
        );
        assert!(
            checked
                .function_namespace(&facts, &resolution.environment, &["other".into()])
                .unwrap()
                .is_none()
        );
        assert!(checked.declaration(&checked.root).is_some());
    }
}

#[test]
fn no_namespace_expansion_requires_no_new_receipt() {
    let (facts, resolution) = ordinary("", false);
    assert!(capture(&facts, &resolution).unwrap().is_none());
}

#[test]
fn hidden_neighbor_cleanup_or_unknown_macro_never_creates_a_receipt() {
    for extra in [
        "macro_rules! hidden { ($owner:ident) => { impl Drop for $owner { fn drop(&mut self) {} } }; } hidden!(Product);",
        "fn hidden() { impl Drop for Product { fn drop(&mut self) {} } }",
        "unknown!(Product);",
        "type u32 = core::primitive::u32;",
        "use core::primitive::u32;",
        "macro_rules! alias { () => { type u32 = core::primitive::u32; }; } alias!();",
        "macro_rules! alias { () => { use core::primitive::u32; }; } alias!();",
        "macro_rules! alias { () => { use core::primitive::*; }; } alias!();",
    ] {
        let (facts, resolution) = ordinary(&format!("{NEIGHBOR} {extra}"), false);
        assert!(capture(&facts, &resolution).is_err(), "{extra}");
    }
}

#[test]
fn retained_receipt_rejects_changed_owner_source_build_and_interface() {
    let (facts, resolution) = ordinary(NEIGHBOR, true);
    let checked = capture(&facts, &resolution).unwrap().unwrap();
    for mutation in ["source", "definition", "invocation", "interface", "cfg"] {
        let mut changed = facts.clone();
        match mutation {
            "source" => changed.source.bytes = "fn read(&self) -> u32 { 9 }".into(),
            "definition" => {
                changed
                    .build
                    .frontend_options
                    .insert("definition_identity".into(), "different".into());
            }
            "invocation" => {
                changed
                    .build
                    .frontend_options
                    .insert("invocation".into(), serde_json::json!(["forged"]));
            }
            "interface" => {
                changed.build.frontend_options.insert(concrete_types::KEY.into(),serde_json::json!({"identity":concrete_types::IDENTITY,"parameters":[],"result":{"status":"Captured","ty":{"kind":"Scalar","evidence":"u32"}}}));
            }
            "cfg" => changed.build.cfgs.push("different".into()),
            _ => unreachable!(),
        }
        assert!(
            checked
                .verify_owner(&changed, &resolution.environment)
                .is_err(),
            "{mutation}"
        );
    }
    let (_, other) = ordinary(&format!("{NEIGHBOR} fn extra_source() {{}}"), true);
    assert!(checked.verify_owner(&facts, &other.environment).is_err());
}

#[test]
fn forged_namespace_nominal_or_impl_range_never_qualifies() {
    let (facts, resolution) = ordinary(NEIGHBOR, false);
    for field in ["source_file", "implementation_span", "owner_nominal"] {
        let mut changed = facts.clone();
        let mut observation = changed
            .build
            .frontend_options
            .get(crate::inherent_calls::KEY)
            .unwrap()
            .clone();
        match field {
            "source_file" => observation[field] = "/different.rs".into(),
            "implementation_span" => observation[field]["start"] = 0.into(),
            "owner_nominal" => observation[field]["definition_path"] = "other::Product".into(),
            _ => unreachable!(),
        }
        changed
            .build
            .frontend_options
            .insert(crate::inherent_calls::KEY.into(), observation);
        assert!(capture(&changed, &resolution).is_err(), "{field}");
    }
}
