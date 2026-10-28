use super::*;

fn method(ordinal: u32) -> FnFacts {
    serde_json::from_value(serde_json::json!({
        "id": format!("sample::{{impl#{ordinal}}}::go"),
        "implements": "sample::Marker::go", "path":"src/lib.rs", "line":1,
        "column":1, "name":"go", "params":0, "has_self":true,
        "statements":1, "source":"fn go(&self)->u32{7}", "preceding_line":"",
        "mutations":[], "calls":[], "effects":[],
        "sig":{"self_ty":"sample::Owner","self_args":[],"receiver":"ref",
            "params":[],"ret":{"prim":"u32"},"is_async":false,"is_unsafe":false,"is_const":false,"generics":0}
    })).unwrap()
}

#[test]
fn ambiguous_bases_never_gain_source_order_review_identities() {
    let first = method(0);
    let second = method(1);
    let table = names(&[first.clone(), second.clone()]);
    assert!(table[&first.id].is_err());
    assert!(table[&second.id].is_err());
    let renamed = method(7);
    assert_eq!(
        names(std::slice::from_ref(&first))[&first.id],
        names(std::slice::from_ref(&renamed))[&renamed.id]
    );
    let unresolved = FnFacts { sig: None, ..first };
    assert!(names(std::slice::from_ref(&unresolved))[&unresolved.id].is_err());
}
