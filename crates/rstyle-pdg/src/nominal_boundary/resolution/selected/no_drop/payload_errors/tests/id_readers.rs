use super::*;
use crate::nominal_boundary::resolution::selected::no_drop::product_fields::test_support as fixture;

fn wrapper(name: &str, declaration: &str, child: TypeTree) -> TypeTree {
    let source = format!("struct {name}({declaration});");
    TypeTree::Struct {
        nominal: nominal(name, &source),
        source: source.into(),
        fields: vec![field("0", declaration, child)],
    }
}

fn setup(copy: bool, extra: &str) -> (HirFnFacts, Resolution, TypeTree) {
    let derive = if copy { "#[derive(Clone, Copy)]" } else { "" };
    let source = format!(
        "#[derive(Clone, Copy)] struct Inner(u32); {derive} struct ID(Inner); fn f(){{}} {extra}"
    );
    let mut root = wrapper(
        "ID",
        "Inner",
        wrapper("Inner", "u32", TypeTree::Scalar("u32".into())),
    );
    let resolution = fixture::resolution(&source, &mut root);
    let mut facts = fixture::facts(&root);
    let start = source.find("fn f()").unwrap() as u32;
    facts.fn_span = crate::ByteSpan {
        start,
        end: start + facts.source.bytes.len() as u32,
    };
    facts.name = "f".into();
    facts.build.frontend_options.insert(
        "rstyle.nominal_selected_unit".into(),
        serde_json::to_value(&resolution.environment.census().context).unwrap(),
    );
    let origins = fixture::origins(&resolution, &facts);
    facts.build.frontend_options.insert(
        crate::declaration_origins::KEY.into(),
        serde_json::to_value(origins).unwrap(),
    );
    (facts, resolution, root)
}

#[test]
fn actual_copy_proof_selects_both_exact_success_declarations() {
    let (facts, resolution, root) = setup(true, "");
    let selected = select_dependencies(&facts, &resolution, None, Some(&root)).unwrap();
    assert_eq!(selected.copies.len(), 2);
    assert_eq!(selected.no_drop, selected.copies);
    assert!(resolution.copy_trees.is_empty());
    let mut unrelated = root.clone();
    if let TypeTree::Struct { nominal, .. } = &mut unrelated {
        nominal.definition_path = "Other".into();
    }
    assert!(!id_dependency(Some(&root), &unrelated));
    assert!(!id_dependency(None, &root));
}

#[test]
fn id_shape_does_not_replace_actual_copy_or_neighbor_cleanup_proof() {
    let (facts, resolution, root) = setup(false, "");
    assert!(select_dependencies(&facts, &resolution, None, Some(&root)).is_err());
    for extra in [
        "impl Drop for ID { fn drop(&mut self) {} }",
        "impl Drop for Inner { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for Inner { fn drop(&mut self) {} } }",
        "unknown!();",
        "#[custom] fn neighboring() {}",
    ] {
        let (facts, resolution, root) = setup(true, extra);
        assert!(
            select_dependencies(&facts, &resolution, None, Some(&root)).is_err(),
            "{extra}"
        );
    }
}
