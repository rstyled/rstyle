//! Receipt reuse tests; these isolated bindings never mint checked conversions.
use super::*;
use serde_json::{Value, json};

fn binding_facts() -> HirFnFacts {
    let mut facts: HirFnFacts =
        serde_json::from_str(include_str!("../../../function_cleanup_tests/facts.json")).unwrap();
    let original: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/copy_conversion_members/original.json"
    )))
    .unwrap();
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        json!({
            "identity": concrete_types::IDENTITY,
            "parameters": [{"status": "Captured", "ty": original["payload"]}],
            "result": {"status": "Captured", "ty": original["tagged"]}
        }),
    );
    facts.build.frontend_options.insert(
        crate::tryfrom_calls::KEY.into(),
        json!({
            "status": "observed", "observation": original["wrap"]["observation"]
        }),
    );
    facts.build.frontend_options.insert(
        "rstyle.rust.compiled-crate".into(),
        original["wrap"]["owner"]["compiled"].clone(),
    );
    facts
}

#[test]
fn retained_member_binding_rejects_mutated_and_missing_proof_inputs() {
    let facts = binding_facts();
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap();
    interface.validate().unwrap();
    assert!(matches!(interface.parameters.as_slice(),
        [Observation::Captured { ty: TypeTree::Struct { nominal, fields, .. } }]
        if nominal.definition_path == "error::component_range::ComponentRange" && fields.len() == 2));
    assert!(matches!(&interface.result,
        Observation::Captured { ty: TypeTree::MixedEnum { variants, .. } }
        if variants.iter().map(|variant| variant.name.as_str()).eq(["InsufficientInformation", "ComponentRange"])));
    let binding = binding::Binding::capture(&facts).unwrap();
    binding.verify(&facts).unwrap();
    for key in [
        concrete_types::KEY,
        crate::tryfrom_calls::KEY,
        "rstyle.rust.compiled-crate",
    ] {
        let mut changed = facts.clone();
        changed
            .build
            .frontend_options
            .insert(key.into(), json!({"forged": true}));
        assert!(binding.verify(&changed).is_err(), "{key}");
        let mut changed = facts.clone();
        changed.build.frontend_options.remove(key);
        assert!(binding.verify(&changed).is_err(), "missing {key}");
    }
    let mut changed = facts.clone();
    changed.signature.is_unsafe = !changed.signature.is_unsafe;
    assert!(binding.verify(&changed).is_err());
}

#[test]
fn target_lookup_keeps_exact_types_and_declaration_namespaces_separate_from_methods() {
    let endpoints = super::super::tests::endpoints(Direction::Unwrap);
    let product = endpoints.product();
    let (nominal, _) = product.nominal_parts().unwrap();
    let namespace = vec!["error".into(), "component_range".into()];
    let targets = vec![targets::Target {
        tree: product.clone(),
        namespace: namespace.clone(),
        cleanup: CheckedNoDrop {
            nominal: nominal.clone(),
        },
    }];

    assert!(targets::find(&targets, product).is_some());
    assert!(targets::namespace(&targets, product, &namespace).unwrap());
    assert!(
        targets::namespace(
            &targets,
            product,
            &["error".into(), "try_from_parsed".into()]
        )
        .is_err()
    );
    assert!(targets::find(&targets, endpoints.tagged()).is_none());
    assert!(!targets::namespace(&targets, endpoints.tagged(), &namespace).unwrap());
    let mut changed = product.clone();
    let TypeTree::Struct { nominal, .. } = &mut changed else {
        unreachable!()
    };
    nominal.declaration_digest = "another-original-declaration".into();
    assert!(targets::find(&targets, &changed).is_none());
}
