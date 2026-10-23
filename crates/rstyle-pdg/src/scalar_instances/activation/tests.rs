use super::*;

fn facts() -> crate::HirFnFacts {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../provenance/fixtures.json")).unwrap();
    let mut facts: crate::HirFnFacts =
        serde_json::from_value(fixtures[0]["facts"].clone()).unwrap();
    facts.signature.return_ty.normalized = Some("i32".into());
    facts.signature.return_ty.spelling = "T".into();
    facts
}

#[test]
fn generic_metadata_stays_intact_while_exact_scalar_interface_is_checked() {
    let facts = facts();
    let generics = facts.signature.generics.clone();
    interface(&facts, ScalarArgument::I32).unwrap();
    assert_eq!(facts.signature.generics, generics);
    assert!(interface(&facts, ScalarArgument::U32).is_err());
    // Interface shape is no authority: raw facts never acquire the private token.
    assert!(!verify(&facts).unwrap());
}

#[test]
fn erased_generics_concrete_source_spelling_and_mixed_arguments_fail_closed() {
    let original = facts();
    let mut changed = original.clone();
    changed.signature.generics.clear();
    assert!(interface(&changed, ScalarArgument::I32).is_err());
    changed = original.clone();
    changed.signature.params[0].ty.spelling = "i32".into();
    assert!(interface(&changed, ScalarArgument::I32).is_err());
    changed = original.clone();
    changed.signature.params[0].ty.normalized = Some("u32".into());
    assert!(interface(&changed, ScalarArgument::I32).is_err());
    changed = original.clone();
    changed.signature.is_unsafe = true;
    assert!(interface(&changed, ScalarArgument::I32).is_err());
    changed = original.clone();
    changed.signature.abi.value = Some("C".into());
    assert!(interface(&changed, ScalarArgument::I32).is_err());
}
