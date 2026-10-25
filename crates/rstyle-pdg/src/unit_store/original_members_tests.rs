//! Original generic body access stays inside its owning immutable unit.
use super::*;

fn facts(identity: &str) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.build.frontend_options = Default::default();
    facts.build.frontend_options.insert(
        "definition_identity".into(),
        serde_json::Value::String(identity.into()),
    );
    facts
}

fn view(builder: &Builder) -> FunctionView<'_> {
    FunctionView {
        resolver: builder,
        id: FunctionId(0),
    }
}

#[test]
fn original_member_is_borrowed_from_its_unit_without_helper_certification() {
    let builder = Builder::new(vec![facts("caller"), facts("generic-member")]).unwrap();
    let caller = view(&builder);
    let original = caller
        .original_local_member("generic-member")
        .unwrap()
        .unwrap();
    assert!(std::ptr::eq(original, &builder.store.functions[1]));
    assert!(builder.store.certified.iter().all(Option::is_none));
    let reference: &dyn EvidenceFacts = &caller;
    assert!(std::ptr::eq(
        original,
        (&reference)
            .original_local_member("generic-member")
            .unwrap()
            .unwrap()
    ));
}

#[test]
fn unavailable_members_and_other_units_cannot_supply_an_original_body() {
    let local = Builder::new(vec![facts("caller")]).unwrap();
    let foreign = Builder::new(vec![facts("generic-member")]).unwrap();
    assert!(
        view(&local)
            .original_local_member("generic-member")
            .unwrap()
            .is_none()
    );
    assert!(
        view(&foreign)
            .original_local_member("generic-member")
            .unwrap()
            .is_some()
    );
    assert!(view(&local).original_local_member("").is_err());
    assert!(
        facts("generic-member")
            .original_local_member("generic-member")
            .unwrap()
            .is_none()
    );
    assert!(Builder::new(vec![facts("duplicate"), facts("duplicate")]).is_err());
}
