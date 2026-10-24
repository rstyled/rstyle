//! Independent original HIR contracts; these tests never mint source authority.
use super::*;
use crate::HirFnFacts;
use serde_json::{Value, json};

fn facts(inverse: bool) -> HirFnFacts {
    serde_json::from_str(if inverse {
        include_str!("tests/1.json")
    } else {
        include_str!("tests/0.json")
    })
    .unwrap()
}

fn trees() -> [TypeTree; 3] {
    let original: Value =
        serde_json::from_str(include_str!("../copy_conversion_members/original.json")).unwrap();
    ["payload", "tagged", "error"]
        .map(|name| serde_json::from_value(original[name].clone()).unwrap())
}

fn replay(facts: &HirFnFacts, inverse: bool) -> Result<(), String> {
    let [product, tagged, error] = trees();
    let output = TypeTree::Result {
        ok: Box::new(product.clone()),
        err: Box::new(error.clone()),
    };
    let node = expression(facts)?;
    let context = Context {
        node,
        facts,
        endpoints: Endpoints {
            output: if inverse { &output } else { &tagged },
            product: &product,
            tagged: &tagged,
            error: inverse.then_some(&error),
        },
    };
    if inverse {
        matched::shape(node, &context).map(|_| ())
    } else {
        constructor_context(node, &context).map(|_| ())
    }
}

fn changed(inverse: bool, path: &str, value: Value) -> HirFnFacts {
    let mut serialized = serde_json::to_value(facts(inverse)).unwrap();
    *serialized.pointer_mut(path).unwrap() = value;
    serde_json::from_value(serialized).unwrap()
}

#[test]
fn original_single_wrap_and_original_wildcard_inverse_have_exact_hir_shapes() {
    for inverse in [false, true] {
        replay(&facts(inverse), inverse).unwrap();
    }
}

#[test]
fn wrapper_rejects_same_typed_substitution_and_changed_compiler_identity() {
    for (path, value) in [
        ("/body/children/0/children/0/res/target/local", json!(999)),
        (
            "/body/children/0/dispatch/op",
            json!("other::ComponentRange"),
        ),
        ("/body/children/0/children/0/span/start", json!(72)),
        (
            "/body/children/0/res/kind",
            json!("resolved_result_variant"),
        ),
    ] {
        assert!(
            replay(&changed(false, path, value), false).is_err(),
            "{path}"
        );
    }
    let mut candidate = facts(false);
    let identity = candidate.body.children[0].res.as_mut().unwrap();
    let mut description: Value = serde_json::from_str(&identity.description).unwrap();
    description["crate_hash"] = json!("another-crate");
    identity.description = description.to_string();
    assert!(replay(&candidate, false).is_err());
}

#[test]
fn inverse_rejects_wrong_formal_binding_reference_and_replaced_wildcard() {
    for (path, value) in [
        ("/body/children/0/children/0/res/target/local", json!(999)),
        (
            "/body/children/0/children/1/children/0/children/0/binding_mut",
            json!(true),
        ),
        (
            "/body/children/0/children/1/children/1/children/0/res/target/local",
            json!(2),
        ),
        (
            "/body/children/0/children/2/children/0/kind",
            json!("pat_local_variant"),
        ),
        (
            "/body/children/0/children/2/children/1/children/0/kind",
            json!("expr_tuple"),
        ),
        (
            "/body/children/0/children/1/children/0/span/end",
            json!(128),
        ),
    ] {
        assert!(replay(&changed(true, path, value), true).is_err(), "{path}");
    }
}

#[test]
fn inverse_rejects_same_layout_nominal_product_and_unit_error_substitutions() {
    for path in [
        "/body/children/0/children/1/children/0/children/0/ty/normalized",
        "/body/children/0/children/2/children/1/children/0/ty/normalized",
    ] {
        let original = serde_json::to_value(facts(true)).unwrap();
        let mut token: Value =
            serde_json::from_str(original.pointer(path).unwrap().as_str().unwrap()).unwrap();
        token["value"]["nominal"]["definition_path"] = json!("same_layout::Substitute");
        assert!(
            replay(&changed(true, path, json!(token.to_string())), true).is_err(),
            "{path}"
        );
    }
}
