use super::*;

const SOURCE: &str = "fn f(){}";

fn node(kind: &str, local: u32, children: Vec<HirNode>) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local,
        },
        kind: kind.into(),
        span: ByteSpan {
            start: 0,
            end: SOURCE.len() as u32,
        },
        snippet: SOURCE.into(),
        ty: None,
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children,
    }
}

fn arm(local: u32) -> HirNode {
    let guard = node("expr_path", local + 1, vec![]);
    let wrapper = node("match_guard", local + 1, vec![guard]);
    node(
        "match_arm",
        local,
        vec![
            node("pat_wild", local + 2, vec![]),
            wrapper,
            node("expr_lit", local + 3, vec![]),
        ],
    )
}

fn collect(root: &HirNode) -> Result<Option<Nodes<'_>>, String> {
    Inventory {
        nodes: BTreeMap::new(),
        identities: BTreeSet::new(),
        pending: vec![Visit::root(root)],
        visited: 0,
    }
    .collect(SOURCE)
}

#[test]
fn established_guard_wrapper_indexes_one_real_expression_and_both_roles() {
    let root = arm(10);
    let nodes = collect(&root).unwrap().unwrap();
    assert_eq!(nodes.values().map(Vec::len).sum::<usize>(), 5);
    assert_eq!(
        nodes
            .keys()
            .filter(|(kind, _, _)| *kind == "expr_path")
            .count(),
        1
    );
}

#[test]
fn guard_wrappers_do_not_hide_repeated_real_nodes() {
    let first = arm(10);
    let mut second = arm(20);
    second.children[1].hir = first.children[1].hir.clone();
    second.children[1].children[0].hir = first.children[1].children[0].hir.clone();
    let root = node("expr_block", 1, vec![first, second]);
    assert!(
        collect(&root)
            .unwrap_err()
            .contains("repeats a HIR identity")
    );
}

#[test]
fn distinct_wrapper_identity_is_checked_as_a_separate_native_node() {
    let mut root = arm(10);
    root.children[1].hir.local = 99;
    assert_eq!(
        collect(&root)
            .unwrap()
            .unwrap()
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        5
    );
    root.children[0].hir.local = 99;
    assert!(
        collect(&root)
            .unwrap_err()
            .contains("repeats a HIR identity")
    );
}

#[test]
fn malformed_guard_aliases_do_not_acquire_wrapper_permission() {
    for mutation in 0..5 {
        let mut root = arm(10);
        match mutation {
            0 => root.kind = "expr_block".into(),
            1 => root.children[1].children.push(node("expr_lit", 99, vec![])),
            2 => root.children[1].binding = Some("forged".into()),
            3 => root.children[1].children[0].kind = "pat_binding".into(),
            _ => root.children[1].children[0].span.start += 1,
        }
        assert!(collect(&root).is_err(), "mutation {mutation}");
    }
}

#[test]
fn wrappers_count_toward_the_original_inventory_work_budget() {
    let root = node(
        "expr_block",
        1,
        (0..4000).map(|i| arm(10 + i * 4)).collect(),
    );
    // 20001 role nodes but only 16001 unique actual HIR identities.
    assert!(collect(&root).unwrap().is_none());
}

#[test]
fn guard_role_budget_accepts_exact_limit_and_rejects_one_more() {
    let mut children: Vec<_> = (0..3276).map(|i| arm(10 + i * 4)).collect();
    children.extend((20000..20003).map(|id| node("expr_lit", id, vec![])));
    let mut root = node("expr_block", 1, children);
    assert_eq!(
        collect(&root)
            .unwrap()
            .unwrap()
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        16384
    );
    root.children.push(node("expr_lit", 20003, vec![]));
    assert!(collect(&root).unwrap().is_none());
}
