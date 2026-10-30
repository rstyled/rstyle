//! Frozen original fields and adversarial shapes; shape alone mints no witness.
use super::*;

fn tree() -> TypeTree {
    let captures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../reference_instances/body/fixtures.json"
    ))
    .unwrap();
    let mut tree: TypeTree =
        serde_json::from_value(captures[0]["interface"]["result"]["ty"].clone()).unwrap();
    let selection: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/function-families/line-reader/selection.json"
    ))
    .unwrap();
    let source = selection["owners"][0]["source"].as_str().unwrap();
    let TypeTree::Struct {
        source: captured,
        fields,
        ..
    } = &mut tree
    else {
        unreachable!()
    };
    *captured = source.into();
    let mut view = fields[0].clone();
    view.name = "remaining".into();
    view.declaration = "&'i [u8]".into();
    let mut width = fields[3].clone();
    width.position = 1;
    width.name = "line_width".into();
    width.declaration = "Option<usize>".into();
    width.ty = TypeTree::Option(Box::new(TypeTree::Scalar("usize".into())));
    *fields = vec![view, width];
    tree
}

#[test]
fn frozen_clone_only_reader_has_two_complete_fields_and_scalar_option() {
    let tree = tree();
    let TypeTree::Struct { source, fields, .. } = &tree else {
        unreachable!()
    };
    assert!(source.contains("derive(Clone)"));
    assert!(!source.contains("Copy"));
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["remaining", "line_width"]
    );
    assert!(eligible(&tree));
}

#[test]
fn nested_owners_references_results_and_unknown_option_payloads_are_not_fresh_shapes() {
    let original = tree();
    for payload in [
        original.clone(),
        TypeTree::Str,
        TypeTree::Shared(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Option(Box::new(TypeTree::Scalar("usize".into()))),
        TypeTree::Scalar("unit".into()),
    ] {
        let mut changed = original.clone();
        let TypeTree::Struct { fields, .. } = &mut changed else {
            unreachable!()
        };
        fields[1].ty = TypeTree::Option(Box::new(payload));
        assert!(!eligible(&changed));
    }
}

#[test]
fn missing_explicit_lifetime_or_missing_view_does_not_request_cleanup_authority() {
    let mut changed = tree();
    let TypeTree::Struct { source, .. } = &mut changed else {
        unreachable!()
    };
    *source = source.replace("<'i>", "").into();
    assert!(!eligible(&changed));
    let mut changed = tree();
    let TypeTree::Struct { fields, .. } = &mut changed else {
        unreachable!()
    };
    fields[0].ty = TypeTree::Scalar("usize".into());
    assert!(!eligible(&changed));
}
