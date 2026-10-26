//! The success owner needs its own exact source and actual builtin Copy proof.
use super::*;
use crate::nominal_boundary::resolution::selected::no_drop::product_fields::test_support as fixture;

const FUNCTION: &str = "fn read() -> Self { loop {} }";
const IMPLEMENTATION: &str = "impl Product { fn read() -> Self { loop {} } }";

fn owner(copy: bool, extra: &str) -> (HirFnFacts, Resolution, TypeTree) {
    let declaration = "struct Product { a: bool, b: bool, c: bool }";
    let derive = if copy { "#[derive(Clone, Copy)]" } else { "" };
    let source = format!("{derive}\n{declaration}\n{IMPLEMENTATION}\n{extra}");
    let mut product = TypeTree::Struct {
        nominal: nominal("Product", declaration),
        source: declaration.into(),
        fields: ["a", "b", "c"]
            .into_iter()
            .enumerate()
            .map(|(position, name)| Field {
                position,
                ..field(name, "bool", TypeTree::Scalar("bool".into()))
            })
            .collect(),
    };
    let resolution = fixture::resolution(&source, &mut product);
    let mut facts = fixture::facts(&product);
    facts.name = "Product::read".into();
    let start = source.find(FUNCTION).unwrap() as u32;
    facts.fn_span = crate::ByteSpan {
        start,
        end: start + FUNCTION.len() as u32,
    };
    facts.source.bytes = FUNCTION.into();
    facts.source.digest = facts.source.bytes.identity().into();
    facts.signature.has_self = false;
    facts.signature.params.clear();
    facts.signature.return_ty.observed = "Product".into();
    facts.signature.return_ty.normalized = Some(crate::data_types::token(&product).unwrap());
    facts.build.frontend_options.insert(
        "definition_identity".into(),
        "original-reader-method".into(),
    );
    facts.build.frontend_options.insert(
        "rstyle.nominal_selected_unit".into(),
        serde_json::to_value(&resolution.environment.census().context).unwrap(),
    );
    let observation = original_observation(&source, &product, facts.fn_span);
    facts.build.frontend_options.insert(
        crate::inherent_calls::KEY.into(),
        serde_json::to_value(observation).unwrap(),
    );
    let origins = fixture::origins(&resolution, &facts);
    facts.build.frontend_options.insert(
        crate::declaration_origins::KEY.into(),
        serde_json::to_value(origins).unwrap(),
    );
    (facts, resolution, product)
}

#[test]
fn original_owner_proof_precedes_selection_and_rejects_another_namespace() {
    let (facts, resolution, product) = owner(true, "");
    assert!(resolution.copy_trees.is_empty());
    assert!(product_namespace(&facts, &resolution, &[], &product).unwrap());
    assert!(resolution.copy_trees.is_empty());
    assert!(product_namespace(&facts, &resolution, &["other".into()], &product).is_err());
}

#[test]
fn shape_cannot_replace_actual_copy_or_exhaustive_neighbor_checks() {
    let (facts, resolution, product) = owner(false, "");
    assert!(product_namespace(&facts, &resolution, &[], &product).is_err());
    for extra in [
        "impl Drop for Product { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Product { fn drop(&mut self) {} } }",
        "unknown!();",
        "#[custom] fn neighboring() {}",
    ] {
        let (facts, resolution, product) = owner(true, extra);
        assert!(
            product_namespace(&facts, &resolution, &[], &product).is_err(),
            "{extra}"
        );
    }
}

#[test]
fn source_owner_cannot_use_a_missing_or_changed_original_observation() {
    let (facts, resolution, product) = owner(true, "");
    let mut missing = facts.clone();
    missing
        .build
        .frontend_options
        .remove(crate::inherent_calls::KEY);
    assert!(product_namespace(&missing, &resolution, &[], &product).is_err());
    for key in ["function_span", "implementation_span"] {
        let mut changed = facts.clone();
        let mut observation = changed.build.frontend_options[crate::inherent_calls::KEY].clone();
        observation[key]["start"] = serde_json::json!(0);
        changed
            .build
            .frontend_options
            .insert(crate::inherent_calls::KEY.into(), observation);
        assert!(
            product_namespace(&changed, &resolution, &[], &product).is_err(),
            "{key}"
        );
    }
}

fn original_observation(
    source: &str,
    product: &TypeTree,
    function_span: crate::ByteSpan,
) -> crate::inherent_calls::Definition {
    let start = source.find(IMPLEMENTATION).unwrap() as u32;
    crate::inherent_calls::Definition {
        identity: crate::inherent_calls::IDENTITY.into(),
        definition_identity: "original-reader-method".into(),
        implementation_identity: "original-reader-impl".into(),
        implementation_span: crate::ByteSpan {
            start,
            end: start + IMPLEMENTATION.len() as u32,
        },
        owner_nominal: product.nominal_identity().unwrap().clone(),
        source_file: "/sample/src/lib.rs".into(),
        function_span,
    }
}
