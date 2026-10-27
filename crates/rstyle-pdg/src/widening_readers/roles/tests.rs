use super::*;

fn node(kind: &str, children: Vec<HirNode>) -> HirNode {
    HirNode {
        hir: crate::HirRef {
            krate: 0,
            owner: 1,
            local: 2,
        },
        kind: kind.into(),
        span: crate::ByteSpan { start: 0, end: 1 },
        snippet: String::new(),
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

#[test]
fn reader_argument_join_rejects_shortened_and_additional_work_without_indexing() {
    for count in [0, 1, 3, 4] {
        let reader = node("expr_call", vec![node("expr_path", vec![]); count]);
        assert!(children::<2>(&reader, "expr_call").is_err());
    }
    let reader = node("expr_call", vec![node("expr_path", vec![]); 2]);
    let [slice, message] = children(&reader, "expr_call").unwrap();
    assert!(std::ptr::eq(slice, &reader.children[0]));
    assert!(std::ptr::eq(message, &reader.children[1]));
}

#[test]
fn callback_join_requires_every_original_direct_child() {
    for (kind, count) in [
        ("expr_block", 1),
        ("expr_closure", 2),
        ("pat_scalar_tuple", 2),
        ("expr_tup", 2),
        ("expr_method_call", 2),
    ] {
        let original = node(kind, vec![node("expr_path", vec![]); count]);
        let actual = match count {
            1 => children::<1>(&original, kind).is_ok(),
            2 => children::<2>(&original, kind).is_ok(),
            _ => unreachable!(),
        };
        assert!(actual);
        let changed = node(kind, vec![]);
        assert!(children::<2>(&changed, kind).is_err());
    }
}

#[test]
fn original_role_join_rejects_changed_kind_and_unsafe_work() {
    let mut reader = node("expr_call", vec![node("expr_path", vec![]); 2]);
    assert!(children::<2>(&reader, "expr_method_call").is_err());
    reader.is_unsafe_block = true;
    assert!(children::<2>(&reader, "expr_call").is_err());
}
