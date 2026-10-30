//! Independent source inventory expectations; these do not mint affine witnesses.
use super::*;

fn original() -> String {
    let selection: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/function-families/line-reader-wrapped/selection.json"
    )))
    .unwrap();
    selection["functions"][0]["source"]
        .as_str()
        .unwrap()
        .to_string()
}

fn plan(source: &str) -> Result<source::Plan, String> {
    let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.source.bytes = source.into();
    source::plan(&facts)
}

#[test]
fn frozen_source_has_one_init_update_move_and_guard_before_initialization() {
    let original = original();
    let plan = plan(&original).unwrap();
    assert_eq!(plan.name, "reader");
    assert_eq!(plan.guards.len(), 1);
    assert_eq!(plan.updates.len(), 1);
    assert_eq!(plan.updates[0].field, "line_width");
    assert!(plan.guards[0].end < plan.declaration.start);
    assert!(plan.initializer.end < plan.updates[0].span.start);
    assert!(plan.updates[0].span.end < plan.terminal.start);
}

#[test]
fn source_inventory_preserves_multiple_ordered_updates() {
    let source = original().replace(
        "Ok(reader)",
        "reader.line_width = Some(line_width); Ok(reader)",
    );
    let plan = plan(&source).unwrap();
    assert_eq!(plan.updates.len(), 2);
    assert!(plan.updates[0].span.end < plan.updates[1].span.start);
}

#[test]
fn source_inventory_rejects_aliases_escapes_and_hidden_evaluation() {
    let original = original();
    for source in [
        original.replace("let mut reader", "let reader"),
        original.replace("let mut reader", "let ref mut reader"),
        original.replace(
            "reader.line_width =",
            "let alias = &reader; reader.line_width =",
        ),
        original.replace(
            "reader.line_width =",
            "consume(reader); reader.line_width =",
        ),
        original.replace("Some(line_width)", "Some({ side_effect(); line_width })"),
        original.replace("reader.line_width", "other.line_width"),
        original.replace("Ok(reader)", "Ok({ reader })"),
        original.replace("Ok(reader)", "let second = reader; Ok(second)"),
        original.replace("Ok(reader)", "Ok(reader); reader"),
        original.replace("line_width < MIN_LINE_WIDTH", "side_effect()"),
        original.replace(
            "return Err(InvalidLength);",
            "side_effect(); return Err(InvalidLength);",
        ),
        original.replace(
            "reader.line_width =",
            "if flag { return Err(InvalidLength); } reader.line_width =",
        ),
    ] {
        assert!(plan(&source).is_err(), "{source}");
    }
}
