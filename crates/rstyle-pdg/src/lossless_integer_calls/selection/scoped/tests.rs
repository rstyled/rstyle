use super::*;
use crate::{
    PathRes,
    concrete_types::{Observation, TypeTree},
};
use serde_json::json;

struct Fixture {
    inventory: Value,
    outer: HirNode,
    direct: HirNode,
    nested: HirNode,
}

fn node(owner: u32, local: u32, span: [u32; 2], kind: &str) -> HirNode {
    serde_json::from_value(json!({
        "hir":{"krate":0,"owner":owner,"local":local}, "kind":kind,
        "span":{"start":span[0],"end":span[1]}, "snippet":"", "children":[]
    }))
    .unwrap()
}

fn closure(owner: u32, body_owner: u32, span: [u32; 2], children: Vec<HirNode>) -> HirNode {
    let mut closure = node(owner, span[0], span, "expr_closure");
    let binding = node(
        body_owner,
        span[0] + 2,
        [span[0] + 1, span[0] + 2],
        "pat_binding",
    );
    let mut body = node(
        body_owner,
        span[0] + 1,
        [span[0] + 5, span[1]],
        "expr_block",
    );
    body.children = children;
    let empty = Observation::Captured {
        ty: TypeTree::Tuple(vec![]),
    };
    let environment = Environment {
        identity: IDENTITY.into(),
        closure: closure.hir.clone(),
        body: body.hir.clone(),
        definition_identity: format!("closure-{body_owner}-{}", span[0]),
        captures: vec![],
        stored_type: empty.clone(),
        signature_abi: "RustCall".into(),
        signature_inputs: vec![empty.clone()],
        signature_output: empty,
    };
    closure.res = Some(PathRes::observation(ENVIRONMENT, &environment).unwrap());
    closure.children = vec![binding, body];
    closure
}

fn record(closure: &HirNode, parent: Option<&HirNode>, direct: &[&HirNode]) -> Value {
    let env = environment(closure).unwrap();
    let mut record = json!({
        "scope":SCOPE,"complete":true,"hir":closure.hir,"span":closure.span,
        "body":env.body,"capture_count":0,"callable":[],"calls":direct.iter().map(|node| call(node)).collect::<Vec<_>>(),
        "definition":{"path":"fixture::closure","definition_identity":env.definition_identity,
            "crate":"fixture","stable_crate_id":"fixture-core","local":true}
    });
    if let Some(parent) = parent {
        record["parent_callback"] = json!(parent.hir);
    }
    record
}

fn call(node: &HirNode) -> Value {
    json!({"hir":node.hir,"span":node.span,"status":"resolved","expanded":false})
}

fn fixture() -> Fixture {
    fixture_owners(2, 3)
}

fn fixture_owners(outer_owner: u32, inner_owner: u32) -> Fixture {
    let direct = node(outer_owner, 30, [30, 40], "expr_call");
    let nested = node(inner_owner, 65, [65, 70], "expr_call");
    let inner = closure(outer_owner, inner_owner, [50, 80], vec![nested.clone()]);
    let outer = closure(
        1,
        outer_owner,
        [10, 90],
        vec![direct.clone(), inner.clone()],
    );
    let mut outer_record = record(&outer, None, &[&direct]);
    outer_record["callbacks"] = json!([record(&inner, Some(&outer), &[&nested])]);
    let inventory = json!({"identity":"rstyle.rust.call-instance-observations/1",
        "complete":true,"calls":[],"callbacks":[outer_record]});
    Fixture {
        inventory,
        outer,
        direct,
        nested,
    }
}

#[test]
fn exact_callback_chain_keeps_root_direct_and_nested_selections_separate() {
    let fixture = fixture();
    let inner = &fixture.outer.children[1].children[1];
    assert_eq!(
        site(&fixture.inventory, &fixture.direct, &[&fixture.outer]).unwrap(),
        &fixture.inventory["callbacks"][0]["calls"][0]
    );
    assert_eq!(
        site(
            &fixture.inventory,
            &fixture.nested,
            &[&fixture.outer, inner]
        )
        .unwrap(),
        &fixture.inventory["callbacks"][0]["callbacks"][0]["calls"][0]
    );
    assert!(super::super::site(&fixture.inventory, &fixture.direct).is_err());
    assert!(site(&fixture.inventory, &fixture.nested, &[&fixture.outer]).is_err());
    assert!(site(&fixture.inventory, &fixture.nested, &[inner]).is_err());
}

#[test]
fn shared_hir_owner_ids_do_not_erase_distinct_callback_scopes() {
    let fixture = fixture_owners(1, 1);
    let inner = &fixture.outer.children[1].children[1];
    assert!(site(&fixture.inventory, &fixture.direct, &[&fixture.outer]).is_ok());
    assert!(
        site(
            &fixture.inventory,
            &fixture.nested,
            &[&fixture.outer, inner]
        )
        .is_ok()
    );
    assert!(site(&fixture.inventory, &fixture.nested, &[&fixture.outer]).is_err());
}

#[test]
fn scoped_selection_rejects_incomplete_foreign_and_reparented_observations() {
    let fixture = fixture();
    for (pointer, replacement) in [
        ("/complete", json!(false)),
        ("/callbacks/0/complete", json!(false)),
        ("/callbacks/0/scope", json!("owning-body")),
        ("/callbacks/0/body/owner", json!(9)),
        (
            "/callbacks/0/definition/definition_identity",
            json!("other"),
        ),
        ("/callbacks/0/definition/local", json!(false)),
        ("/callbacks/0/capture_count", json!(1)),
        ("/callbacks/0/calls/0/status", json!("unresolved")),
        ("/callbacks/0/calls/0/expanded", json!(true)),
        ("/callbacks/0/calls/0/hir/owner", json!(9)),
        ("/callbacks/0/calls/0/span/start", json!(31)),
        ("/callbacks/0/callbacks/0/parent_callback/owner", json!(9)),
        (
            "/callbacks/0/callbacks/0/definition/crate",
            json!("foreign"),
        ),
    ] {
        let mut changed = fixture.inventory.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            site(&changed, &fixture.direct, &[&fixture.outer]).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn duplicate_or_flattened_call_records_cannot_escape_their_exact_scope() {
    let fixture = fixture();
    for change in ["root", "sibling", "duplicate", "flatten", "omit-child"] {
        let mut changed = fixture.inventory.clone();
        let selected = changed["callbacks"][0]["calls"][0].clone();
        match change {
            "root" => changed["calls"] = json!([selected]),
            "sibling" => changed["callbacks"][0]["callbacks"][0]["calls"] = json!([selected]),
            "duplicate" => changed["callbacks"][0]["calls"]
                .as_array_mut()
                .unwrap()
                .push(selected),
            "flatten" => {
                changed["calls"] = json!([selected]);
                changed["callbacks"][0]["calls"] = json!([]);
            }
            _ => changed["callbacks"][0]["callbacks"] = json!([]),
        }
        assert!(
            site(&changed, &fixture.direct, &[&fixture.outer]).is_err(),
            "{change}"
        );
    }
}

#[test]
fn retained_callback_owner_and_environment_are_required_independently() {
    let fixture = fixture();
    let mut changed = fixture.outer.clone();
    changed.res.as_mut().unwrap().kind = "compiler_id_error_map_environment".into();
    assert!(site(&fixture.inventory, &fixture.direct, &[&changed]).is_err());
    let mut changed = fixture.outer.clone();
    changed.children[1].children.pop();
    assert!(site(&fixture.inventory, &fixture.direct, &[&changed]).is_err());
}

#[test]
fn selected_call_collisions_in_an_unselected_root_sibling_are_rejected() {
    let fixture = fixture();
    let sibling = closure(1, 4, [100, 180], vec![]);
    let mut changed = fixture.inventory.clone();
    changed["callbacks"]
        .as_array_mut()
        .unwrap()
        .push(record(&sibling, None, &[&fixture.direct]));
    let error = site(&changed, &fixture.direct, &[&fixture.outer]).unwrap_err();
    assert!(error.contains("collides"));
}

#[test]
fn complete_receipt_requires_a_separate_expected_source_call_roster_proof() {
    let fixture = fixture();
    let mut changed = fixture.inventory.clone();
    changed["callbacks"][0]["callbacks"][0]["calls"] = json!([]);
    // Eligible-call capture is intentionally not every source call. The owning
    // reader proof must require its expected sites even when another is selected.
    assert!(site(&changed, &fixture.direct, &[&fixture.outer]).is_ok());
    let inner = &fixture.outer.children[1].children[1];
    assert!(site(&changed, &fixture.nested, &[&fixture.outer, inner]).is_err());
}

#[test]
fn indexed_call_membership_rejects_duplicate_hir_or_span_in_retained_hir() {
    let fixture = fixture();
    for collision in ["hir", "span", "both"] {
        let mut outer = fixture.outer.clone();
        let mut duplicate = fixture.direct.clone();
        if collision == "hir" {
            duplicate.span = ByteSpan { start: 41, end: 45 };
        } else if collision == "span" {
            duplicate.hir.local += 1;
        }
        outer.children[1].children.push(duplicate);
        assert!(
            site(&fixture.inventory, &fixture.direct, &[&outer]).is_err(),
            "{collision}"
        );
    }
}

#[test]
fn explicit_chain_must_use_the_same_retained_nested_callback() {
    let fixture = fixture();
    let mut inner = fixture.outer.children[1].children[1].clone();
    inner.children[1].children.clear();
    assert!(
        site(
            &fixture.inventory,
            &fixture.nested,
            &[&fixture.outer, &inner]
        )
        .is_err()
    );
    assert!(site(&fixture.inventory, &fixture.direct, &[]).is_err());
    let oversized = vec![&fixture.outer; crate::concrete_types::MAX_DEPTH + 1];
    assert!(site(&fixture.inventory, &fixture.direct, &oversized).is_err());
}

#[test]
fn inventory_and_retained_hir_share_one_node_budget() {
    let fixture = fixture();
    let mut inventory = fixture.inventory.clone();
    let other = node(8, 1, [200, 201], "expr_call");
    // The inventory alone fits; the retained subtree makes the combined visit
    // exceed the budget. Distinct selected identity avoids collision rejection.
    inventory["calls"] = json!(vec![call(&other); MAX_NODES - 8]);
    let error = site(&inventory, &fixture.direct, &[&fixture.outer]).unwrap_err();
    assert!(error.contains("budget"));
    let mut outer = fixture.outer.clone();
    outer.children[1].children.extend(vec![other; MAX_NODES]);
    let error = site(&fixture.inventory, &fixture.direct, &[&outer]).unwrap_err();
    assert!(error.contains("budget"));
}

#[test]
fn iterative_inventory_and_hir_walks_reject_excessive_depth() {
    let fixture = fixture();
    let mut changed = fixture.inventory.clone();
    let mut nested = json!({"calls":[], "callbacks":[]});
    for _ in 0..crate::concrete_types::MAX_DEPTH + 2 {
        nested = json!({"calls":[], "callbacks":[nested]});
    }
    changed["callbacks"].as_array_mut().unwrap().push(nested);
    let error = site(&changed, &fixture.direct, &[&fixture.outer]).unwrap_err();
    assert!(error.contains("budget"));
    let mut outer = fixture.outer.clone();
    let mut deep = node(2, 200, [20, 21], "expr_block");
    for _ in 0..MAX_HIR_DEPTH + 1 {
        let mut parent = node(2, 201, [20, 21], "expr_block");
        parent.children = vec![deep];
        deep = parent;
    }
    outer.children[1].children.push(deep);
    let error = site(&fixture.inventory, &fixture.direct, &[&outer]).unwrap_err();
    assert!(error.contains("budget"));
}
