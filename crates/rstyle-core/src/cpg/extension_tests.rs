//! Runtime bounds over a node of a registered extension placed in the example's `count`, before
//! its scan of `items`: what the bindings among its children write, and an extension without a
//! role.

use super::run_bounds;
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");

fn count_function(document: &mut Value) -> &mut Value {
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    let count = functions.iter_mut().find(|f| f["id"] == "fn:count");
    count.unwrap()
}

/// A node of `count` at the span of `total`'s pattern, of no type.
fn part(function: &Value, id: &str, kind: &str, children: Value) -> Value {
    let nodes = function["nodes"].as_array().unwrap();
    let pattern = nodes.iter().find(|node| node["id"] == "count.total_pat");
    let mut part = pattern.unwrap().clone();
    part["id"] = json!(id);
    part["kind"] = json!(kind);
    part["original_kind"] = json!(kind);
    part["children"] = children;
    part
}

fn refer(function: &mut Value, node: &str, symbol: &str) {
    let refs = function["refs"].as_array_mut().unwrap();
    refs.push(
        json!({"node": node, "status": "resolved", "target": {"kind": "symbol", "id": symbol}}),
    );
}

/// `count` with an extension statement before its loop, holding `children`.
fn with_extension(extension: &str, children: Value, nodes: Vec<Value>) -> Value {
    let mut document: Value = serde_json::from_str(EXAMPLE).unwrap();
    let function = count_function(&mut document);
    let mut node = part(function, "count.ext", "extension", children);
    node["extension"] = json!(extension);
    let statement = part(
        function,
        "count.ext_stmt",
        "expression_statement",
        json!(["count.ext"]),
    );
    let all = function["nodes"].as_array_mut().unwrap();
    all.extend(nodes);
    all.extend([node, statement]);
    let block = all.iter_mut().find(|node| node["id"] == "count.block");
    let statements = block.unwrap()["children"].as_array_mut().unwrap();
    statements.insert(1, json!("count.ext_stmt"));
    document
}

/// `count` with an import that binds `symbol` again, through a pattern of its own.
fn importing(symbol: &str) -> Value {
    let mut document = with_extension("py:import", json!(["count.bound"]), Vec::new());
    let function = count_function(&mut document);
    let binding = part(function, "count.bound", "pattern_binding", json!([]));
    function["nodes"].as_array_mut().unwrap().push(binding);
    refer(function, "count.bound", symbol);
    document
}

fn program(document: Value) -> String {
    let programs = run_bounds(vec![document]).unwrap();
    let (_, input) = programs.iter().find(|(f, _)| f.ends_with("count")).unwrap();
    serde_json::to_string(input).unwrap()
}

const UNKNOWN_WRITE: &str = r#""value":"Unknown"}"#;

#[test]
fn a_binding_an_extension_writes_again_assigns_its_counter() {
    let program = program(importing("count.total"));
    assert!(program.contains(r#""callee":"py:import""#), "{program}");
    assert!(program.contains(UNKNOWN_WRITE), "{program}");
    assert!(!program.contains("Unsupported"), "{program}");
}

#[test]
fn a_binding_an_extension_writes_over_an_extent_source_is_unsupported() {
    let program = program(importing("count.items"));
    assert!(
        program.contains("Declaration writes an extent source"),
        "{program}"
    );
}

#[test]
fn a_binding_of_anything_else_writes_nothing_a_bound_reads() {
    // The loop's iterator is neither a counter nor what the loop measures.
    let program = program(importing("count.iter"));
    assert!(!program.contains(UNKNOWN_WRITE), "{program}");
    assert!(!program.contains("Unsupported"), "{program}");
}

/// A pattern that holds a place besides a binding of its own writes both, and a binding no symbol
/// names writes nothing.
#[test]
fn a_pattern_an_extension_holds_writes_its_places() {
    let mut document = with_extension("py:import", json!(["count.pair"]), Vec::new());
    let function = count_function(&mut document);
    let mut place = part(function, "count.place", "identifier", json!([]));
    place["original_kind"] = json!("expr_path");
    let unnamed = part(function, "count.unnamed", "pattern_binding", json!([]));
    let pair = part(
        function,
        "count.pair",
        "pattern_tuple",
        json!(["count.place", "count.unnamed"]),
    );
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .extend([place, unnamed, pair]);
    refer(function, "count.place", "count.total");
    let program = program(document);
    assert_eq!(program.matches(UNKNOWN_WRITE).count(), 1, "{program}");
    assert!(!program.contains("Unsupported"), "{program}");
}

#[test]
fn an_extension_without_a_role_is_unsupported() {
    let program = program(with_extension("py:compare-chain", json!([]), Vec::new()));
    assert!(program.contains("has unmodeled control"), "{program}");
}

/// A control role this lowering can't read, a try with nothing in it, stays unsupported.
#[test]
fn an_empty_try_is_unsupported() {
    let program = program(with_extension("py:try", json!([]), Vec::new()));
    assert!(program.contains("has unmodeled control"), "{program}");
}
