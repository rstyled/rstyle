//! Runtime bounds over a node of a registered extension placed in the example's `count`, before
//! its scan of `items`: what the bindings among its children write, and an extension without a
//! role.

use super::run_bounds;
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");
const COMPREHENSIONS: &str = include_str!("testdata/comprehensions-py.cpg.json");
const DEFERRED: &str = include_str!("testdata/deferred-go.cpg.json");

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
    let program = program(with_extension("py:assign-chain", json!([]), Vec::new()));
    assert!(program.contains("has unmodeled control"), "{program}");
}

/// A control role this lowering can't read, a try with nothing in it, stays unsupported.
#[test]
fn an_empty_try_is_unsupported() {
    let program = program(with_extension("py:try", json!([]), Vec::new()));
    assert!(program.contains("has unmodeled control"), "{program}");
}

/// A C++ do statement whose block holds no statement to test leaves its function unknown.
#[test]
fn a_do_statement_without_a_test_is_unsupported() {
    let mut document: Value = serde_json::from_str(EXAMPLE).unwrap();
    document["unit"]["language"]["name"] = json!("cxx");
    let function = count_function(&mut document);
    let mut looped = part(function, "count.do", "loop", json!(["count.parts"]));
    looped["original_kind"] = json!("DoStmt");
    let parts = part(function, "count.parts", "block", json!([]));
    let statement = part(
        function,
        "count.do_stmt",
        "expression_statement",
        json!(["count.do"]),
    );
    let nodes = function["nodes"].as_array_mut().unwrap();
    nodes.extend([looped, parts, statement]);
    let block = nodes.iter_mut().find(|node| node["id"] == "count.block");
    let statements = block.unwrap()["children"].as_array_mut().unwrap();
    statements.insert(1, json!("count.do_stmt"));
    let program = program(document);
    assert!(
        program.contains("Loop guard is not the last statement of its block"),
        "{program}"
    );
}

/// `count` with the source of its scan given a type of its own, or none.
fn scanning(ty: Option<Value>) -> Value {
    let mut document: Value = serde_json::from_str(EXAMPLE).unwrap();
    if let Some(ty) = &ty {
        document["unit"]["types"]
            .as_array_mut()
            .unwrap()
            .push(ty.clone());
    }
    let function = count_function(&mut document);
    let nodes = function["nodes"].as_array_mut().unwrap();
    let items = nodes.iter_mut().find(|node| node["id"] == "count.items");
    let items = items.unwrap().as_object_mut().unwrap();
    match ty {
        Some(ty) => _ = items.insert("type".into(), ty["id"].clone()),
        None => _ = items.remove("type"),
    }
    document
}

/// A source of a sequence type of one fixed length, as an array's, counts that many passes.
#[test]
fn a_fixed_length_source_counts_its_length() {
    let array = json!({"id": "type:array", "kind": "sequence", "spelling": "[u32; 4]",
                       "element": "type:u32", "fixed": true, "length": 4});
    let program = program(scanning(Some(array)));
    assert!(program.contains(r#""value":{"Const":4}"#), "{program}");
    assert!(!program.contains("Unsupported"), "{program}");
}

/// A source of no type has no count.
#[test]
fn a_source_of_no_type_is_unsized() {
    let program = program(scanning(None));
    assert!(program.contains("have a size model"), "{program}");
}

/// A generated `for` wrapper that holds more than one source isn't looked through, so the loop
/// isn't counted by either.
#[test]
fn a_generated_wrapper_of_two_sources_is_unsized() {
    let mut document: Value = serde_json::from_str(EXAMPLE).unwrap();
    let function = count_function(&mut document);
    let mut other = part(function, "count.other", "identifier", json!([]));
    other["original_kind"] = json!("expr_path");
    let nodes = function["nodes"].as_array_mut().unwrap();
    nodes.push(other);
    let wrapper = nodes
        .iter_mut()
        .find(|node| node["id"] == "count.into_iter");
    let sources = wrapper.unwrap()["children"].as_array_mut().unwrap();
    sources.push(json!("count.other"));
    refer(function, "count.other", "count.items");
    let program = program(document);
    assert!(program.contains("have a size model"), "{program}");
}

/// A source of a tuple type counts the items its type lists, and one whose type doesn't list them
/// has no count, rather than none.
#[test]
fn a_tuple_source_counts_only_the_items_its_type_lists() {
    let listed = json!({"id": "type:triple", "kind": "tuple", "spelling": "(u32, u32, u32)",
                        "items": ["type:u32", "type:u32", "type:u32"]});
    let counted = program(scanning(Some(listed)));
    assert!(counted.contains(r#""value":{"Const":3}"#), "{counted}");
    let unlisted = json!({"id": "type:triple", "kind": "tuple", "spelling": "tuple"});
    let unlisted = program(scanning(Some(unlisted)));
    assert!(unlisted.contains("have a size model"), "{unlisted}");
}

/// The program of the function named `name` in `document`.
fn named(document: Value, name: &str) -> String {
    let programs = run_bounds(vec![document]).unwrap();
    let suffix = format!(".{name}");
    let (_, input) = programs.iter().find(|(f, _)| f.ends_with(&suffix)).unwrap();
    serde_json::to_string(input).unwrap()
}

/// The program of `name` in the comprehensions document, after `change` edits that function.
fn comprehension(name: &str, change: impl FnOnce(&mut Value)) -> String {
    let mut document: Value = serde_json::from_str(COMPREHENSIONS).unwrap();
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    change(functions.iter_mut().find(|f| f["name"] == name).unwrap());
    named(document, name)
}

/// `summed`'s program, after `change` edits the reference of its call of `sum`, whose argument is
/// a generator expression.
fn summed(change: impl FnOnce(&mut Value)) -> String {
    comprehension("summed", |function| {
        let refs = function["refs"].as_array_mut().unwrap();
        change(refs.iter_mut().find(|r| r["node"] == "f4.n8").unwrap());
    })
}

const UNCONSUMED: &str = "A lazy value not consumed by the call it's an argument of is unmodeled";

/// A generator expression handed first to a builtin that drains it runs where it's written. One a
/// call could keep, return or pass on, by a keyword, after another argument or to a function of
/// the workspace's own, is unsupported.
#[test]
fn a_lazy_value_runs_in_place_only_where_its_call_drains_it() {
    let drained = summed(|_| {});
    assert!(!drained.contains(UNCONSUMED), "{drained}");
    let kept = summed(|call| {
        call["target"]["id"] = json!("def:py::comprehensions.summed");
    });
    assert!(kept.contains(UNCONSUMED), "{kept}");
    let keyword = summed(|call| call["arguments"] = json!([{"parameter": "start"}]));
    assert!(keyword.contains(UNCONSUMED), "{keyword}");
    let later = summed(|call| call["arguments"] = json!([{"position": 1}]));
    assert!(later.contains(UNCONSUMED), "{later}");
}

/// A generator expression that isn't one of a call's written arguments isn't drained by it: one
/// a local holds, one in a wrapper outside any call, one in a call with no reference and one that
/// is the call's callee.
#[test]
fn a_lazy_value_outside_a_calls_written_arguments_is_unsupported() {
    let stored = comprehension("stored", |_| {});
    assert!(stored.contains(UNCONSUMED), "{stored}");
    let wrapped = comprehension("stored", |function| {
        let nodes = function["nodes"].as_array_mut().unwrap();
        let mut wrapper = nodes.iter().find(|n| n["id"] == "f6.n6").unwrap().clone();
        wrapper["id"] = json!("f6.wrapper");
        wrapper["extension"] = json!("py:fstring");
        wrapper["children"] = json!(["f6.n6"]);
        wrapper.as_object_mut().unwrap().remove("modifiers");
        nodes.push(wrapper);
        let declaration = nodes.iter_mut().find(|n| n["id"] == "f6.n8").unwrap();
        declaration["children"] = json!(["f6.n7", "f6.wrapper"]);
    });
    assert!(wrapped.contains(UNCONSUMED), "{wrapped}");
    let unreferenced = comprehension("summed", |function| {
        let refs = function["refs"].as_array_mut().unwrap();
        refs.retain(|r| r["node"] != "f4.n8");
    });
    assert!(unreferenced.contains(UNCONSUMED), "{unreferenced}");
    let callee = comprehension("summed", |function| {
        let nodes = function["nodes"].as_array_mut().unwrap();
        let call = nodes.iter_mut().find(|n| n["id"] == "f4.n8").unwrap();
        call["children"] = json!(["f4.n7", "f4.n0"]);
    });
    assert!(callee.contains(UNCONSUMED), "{callee}");
}

/// `Reported` with its deferred closure literal bound to an immutable local `f` first, and the
/// defer calling `f`: `f := func() {...}; defer f()`.
fn deferring_a_local_closure() -> Value {
    let mut document: Value = serde_json::from_str(DEFERRED).unwrap();
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    let function = functions
        .iter_mut()
        .find(|f| f["name"] == "Reported")
        .unwrap();
    let span =
        |start: usize, end: usize| json!({"file": "file:deferred.go", "start": start, "end": end});
    let nodes = function["nodes"].as_array_mut().unwrap();
    for node in nodes.iter_mut() {
        match node["id"].as_str() {
            Some("f3.n26") => node["children"] = json!(["f3.n34"]),
            Some("f3.n33") => node["children"] = json!(["f3.n4", "f3.n36", "f3.n28", "f3.n32"]),
            _ => {}
        }
    }
    let source = json!({"origin": "source"});
    nodes.extend([
        json!({"id": "f3.n34", "kind": "identifier", "original_kind": "Ident", "children": [],
               "span": span(790, 791), "type": "t17", "provenance": source}),
        json!({"id": "f3.n35", "kind": "pattern_binding", "original_kind": "Ident.binding",
               "children": [], "span": span(790, 791), "type": "t17", "provenance": source}),
        json!({"id": "f3.n36", "kind": "local_declaration", "original_kind": "AssignStmt.define",
               "children": ["f3.n35", "f3.n25"], "span": span(790, 852), "type": "t8",
               "provenance": source}),
    ]);
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id": "f3.s3", "name": "f", "kind": "local", "scope": "f3", "node": "f3.n35",
        "declared": {"file": "file:deferred.go", "start": 790, "end": 791},
        "mutable": false, "type": "t17"
    }));
    function["refs"].as_array_mut().unwrap().push(
        json!({"node": "f3.n34", "status": "resolved", "target": {"kind": "symbol", "id": "f3.s3"}}),
    );
    let accesses = function["accesses"].as_array_mut().unwrap();
    accesses.push(json!({"node": "f3.n35", "symbol": "f3.s3", "mode": "write"}));
    accesses.push(json!({"node": "f3.n34", "symbol": "f3.s3", "mode": "read"}));
    document
}

/// A deferred call of a closure an immutable local holds reads what the closure does when it
/// runs, as one of a closure written there does, so it's unsupported too.
#[test]
fn a_deferred_call_of_a_local_closure_is_unsupported() {
    const LATER: &str = "A closure that runs later reads values that can change first";
    let written = named(serde_json::from_str(DEFERRED).unwrap(), "Reported");
    assert!(written.contains(LATER), "{written}");
    let local = named(deferring_a_local_closure(), "Reported");
    assert!(local.contains(LATER), "{local}");
}
