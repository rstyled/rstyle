use super::*;

fn function(id: &str) -> FnFacts {
    serde_json::from_value(serde_json::json!({
        "id": id, "path":"src/lib.rs", "line":1, "column":1,
        "name":"go", "params":0, "has_self":false, "statements":1,
        "source":"fn go(){}", "preceding_line":"", "mutations":[],
        "calls":[], "effects":[]
    }))
    .unwrap()
}

fn method(ordinal: usize) -> FnFacts {
    let sig = serde_json::from_value(serde_json::json!({
        "self_ty":"sample::Owner", "self_args":[], "receiver":"ref",
        "params":[], "ret":{"prim":"u32"}, "is_async":false,
        "is_unsafe":false, "is_const":false, "generics":0
    }))
    .unwrap();
    FnFacts {
        sig: Some(sig),
        ..function(&format!("sample::{{impl#{ordinal}}}::go"))
    }
}

#[test]
fn full_census_detects_a_duplicate_after_the_retained_owner_prefix() {
    let facts: Vec<_> = std::iter::once(method(0))
        .chain((1..4096).map(|index| function(&format!("sample::f{index}"))))
        .chain(std::iter::once(method(1)))
        .collect();
    let observed = collect_names(&facts);
    assert_eq!(facts.len(), 4097);
    assert_eq!(observed.counts["sample::Owner::go"], 2);
    assert!(observed.by_id[&facts[0].id].is_err());
    assert!(observed.by_id[&facts[4096].id].is_err());
    assert_eq!(observed.counts["sample::f4095"], 1);
    assert_eq!(
        observed.by_id["sample::f4095"].as_deref(),
        Ok("sample::f4095")
    );
}

#[test]
fn census_keeps_unstable_names_and_the_existing_identity_policy() {
    let stable = method(17);
    let unresolved = function("sample::{impl#99}::go");
    let facts = [stable, unresolved, function("sample::free")];
    let observed = collect_names(&facts);
    assert_eq!(observed.counts["sample::{impl#99}::go"], 1);
    assert!(observed.by_id["sample::{impl#99}::go"].is_err());
    assert_eq!(
        observed.by_id[&facts[0].id].as_deref(),
        Ok("sample::Owner::go")
    );
    assert_eq!(observed.by_id, names(&facts));
}
