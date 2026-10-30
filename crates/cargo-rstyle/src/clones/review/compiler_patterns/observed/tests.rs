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

fn compilation_units() -> Vec<UnitId> {
    let units: Vec<crate::clones::capture::Unit> = ["caller", "dependency", "unrelated"]
        .into_iter()
        .map(|name| {
            serde_json::from_value(serde_json::json!({
                "extraction_profile": "r0",
                "target": {
                    "package": name, "manifest_dir": format!("/{name}"),
                    "source": format!("/{name}/src/lib.rs"), "name": name,
                    "kind": ["lib"], "test": false
                },
                "test": false, "invocation": [], "directory": format!("/run/{name}"),
                "artifacts": []
            }))
            .unwrap()
        })
        .collect();
    crate::clones::identity::assign(&units)
        .unwrap()
        .into_iter()
        .map(|assignment| assignment.context.id)
        .collect()
}

#[test]
fn review_call_locators_require_the_callers_actual_dependency_owner() {
    let units = compilation_units();
    let ids = [
        "caller::local",
        "dependency::read",
        "unrelated::read",
        "external::read",
    ];
    let names = BTreeMap::from([
        (ids[0].into(), Ok("stable-local".into())),
        (ids[1].into(), Ok("stable-dependency".into())),
        (
            ids[2].into(),
            Err("Unrelated ambiguous locator must not be consulted".into()),
        ),
    ]);
    let calls = CallNames {
        names: &names,
        owners: BTreeMap::from([(ids[0], units[0]), (ids[1], units[1]), (ids[2], units[2])]),
        routes: BTreeMap::from([(units[0], BTreeSet::from([units[1]]))]),
    };
    let mut graph: rstyle_core::facts::Pdg = serde_json::from_value(serde_json::json!({
        "nodes": ids.map(|id| serde_json::json!({"kind": "call", "callee_id": id, "line": 9})),
        "edges": []
    }))
    .unwrap();
    calls.normalize(&mut graph, units[0]).unwrap();
    let actual: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| node.callee_id.as_deref().unwrap())
        .collect();
    assert_eq!(
        actual,
        ["stable-local", "stable-dependency", ids[2], ids[3]]
    );
    assert!(graph.nodes.iter().all(|node| node.line == 0));
}
