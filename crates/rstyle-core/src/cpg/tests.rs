use super::layout::{self, digest, split};
use super::run_facts;
use crate::complexity::Kind;
use crate::facts::{
    ArgSource, CallEvent, Edge, EffectEvent, EffectKind, FnFacts, MutationEvent, MutationOrigin,
};
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");
/// `unit_ref.digest` of the example's split unit document, pinned so a change to
/// canonical JSON or digests shows up here first.
const SPLIT_DIGEST: &str =
    "sha256:6f37ad7656340841e63b531345b0770dbeedeced8978cea5901a1171a6920847";

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}

fn facts_of(document: Value) -> Vec<FnFacts> {
    run_facts(vec![document]).unwrap()
}

fn error_of(documents: Vec<Value>) -> String {
    run_facts(documents).unwrap_err()
}

fn kinds(facts: &FnFacts) -> Vec<Kind> {
    let measured = facts.complexity.as_ref().unwrap();
    measured
        .contributions
        .iter()
        .map(|point| point.kind)
        .collect()
}

fn call(callee: &str, line: usize, dynamic: bool) -> CallEvent {
    CallEvent {
        callee: callee.into(),
        line,
        dynamic,
    }
}

// The example document.

#[test]
fn example_functions_have_their_identity_and_source() {
    let facts = facts_of(example());
    let names: Vec<_> = facts
        .iter()
        .map(|f| (f.id.as_str(), f.line, f.column))
        .collect();
    assert_eq!(
        names,
        [
            ("example::larger", 3, 5),
            ("example::top", 7, 5),
            ("example::count", 11, 5)
        ]
    );
    assert_eq!(
        facts[0].source,
        "fn larger<T: Ord>(a: T, b: T) -> T {\n    a.max(b)\n}"
    );
    assert_eq!(
        (facts[0].path.as_str(), facts[0].params, facts[0].has_self),
        ("src/lib.rs", 2, false)
    );
    assert_eq!(facts[0].preceding_line, "");
}

#[test]
fn example_calls_name_declared_library_callees_and_workspace_edges() {
    let facts = facts_of(example());
    assert_eq!(facts[0].calls, [call("core::cmp::Ord::max", 4, false)]);
    assert_eq!(facts[1].calls, [call("core::cmp::Ord::min", 8, false)]);
    let edge = Edge {
        callee: "example::larger".into(),
        line: 8,
        dynamic: false,
        args: vec![ArgSource::Param(0), ArgSource::Param(1)],
    };
    assert_eq!(facts[1].edges, [edge]);
    let lowered = [
        call("core::iter::IntoIterator::into_iter", 13, false),
        call("core::iter::Iterator::next", 13, false),
    ];
    assert_eq!(facts[2].calls, lowered);
}

#[test]
fn example_loop_counts_statements_and_complexity_without_its_lowering() {
    let count = &facts_of(example())[2];
    assert_eq!((count.statements, count.top_statements), (5, 3));
    assert_eq!(kinds(count), [Kind::Loop, Kind::Branch, Kind::And]);
    let measured = count.complexity.as_ref().unwrap();
    assert_eq!((measured.cyclomatic, measured.cognitive), (4, 4));
    let and = &measured.contributions[2];
    assert_eq!((and.line, and.column), (14, 26));
}

#[test]
fn desugared_nodes_are_looked_through() {
    let mut document = example();
    let nodes = document["unit"]["functions"][2]["nodes"]
        .as_array_mut()
        .unwrap();
    let into_iter = nodes
        .iter_mut()
        .find(|n| n["id"] == "count.into_iter")
        .unwrap();
    into_iter["kind"] = json!("loop");
    assert_eq!(
        kinds(&facts_of(document)[2]),
        [Kind::Loop, Kind::Branch, Kind::And]
    );
}

#[test]
fn example_mutations_separate_source_writes_from_the_lowered_iterator() {
    let count = &facts_of(example())[2];
    let event = |root: &str, line, origin| MutationEvent {
        origin,
        root: root.into(),
        field_path: String::new(),
        line,
        escapes: false,
        delegated: false,
    };
    let expected = [
        event("total", 15, MutationOrigin::Source),
        event("iter", 13, MutationOrigin::ForDesugaring),
    ];
    assert_eq!(count.mutations, expected);
}

#[test]
fn example_passes_the_default_rules() {
    let report = crate::evaluate(facts_of(example()), &crate::config::Config::default()).unwrap();
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
}

// Layouts.

#[test]
fn split_runs_derive_the_bundled_facts() {
    let bundled = example();
    let documents = split(&bundled);
    assert_eq!(documents[1]["unit_ref"]["digest"], SPLIT_DIGEST);
    assert_eq!(run_facts(documents).unwrap(), facts_of(bundled));
}

#[test]
fn canonical_json_sorts_keys_and_escapes_like_the_profile() {
    let value = json!({"b": [1, {"d": "é\n", "c": null}], "a": true});
    assert_eq!(
        layout::canonical(&value),
        r#"{"a":true,"b":[1,{"c":null,"d":"é\n"}]}"#
    );
    assert!(digest(&value).starts_with("sha256:"));
}

#[test]
fn split_documents_must_belong_to_their_unit() {
    let mut documents = split(&example());
    documents[0]["unit"]["configuration"] = json!(["feature=\"extra\""]);
    assert_eq!(
        error_of(documents),
        "Function fn:larger belongs to another unit document"
    );

    let mut documents = split(&example());
    documents[2]["coverage"]["layers"]["value"]["status"] = json!("unknown");
    assert_eq!(
        error_of(documents),
        "Function fn:top differs from its unit in coverage"
    );

    let mut documents = split(&example());
    documents.push(documents[0].clone());
    assert_eq!(
        error_of(documents),
        "A split run needs exactly one unit document, found 2"
    );

    let mut documents = split(&example());
    documents[0]["layout"] = json!("bundled");
    assert_eq!(
        error_of(documents),
        "A run is one bundled document or a set of split documents"
    );
}

#[test]
fn documents_outside_the_profile_are_rejected() {
    let mut newer = example();
    newer["profile"]["version"] = json!("0.2.0");
    assert!(error_of(vec![newer]).starts_with("Unsupported document"));

    let mut extra = example();
    extra["unit"]["functions"][0]["nodes"][0]["role"] = json!("tail");
    assert!(error_of(vec![extra]).contains("unknown field `role`"));

    let mut undeclared = example();
    undeclared["coverage"]["capabilities"]
        .as_object_mut()
        .unwrap()
        .remove("aliasing");
    assert_eq!(
        error_of(vec![undeclared]),
        "Coverage does not declare aliasing"
    );

    let mut no_content = example();
    no_content["unit"]["files"][0]
        .as_object_mut()
        .unwrap()
        .remove("content");
    assert_eq!(
        error_of(vec![no_content]),
        "File file:lib has no content; rule facts need source text"
    );
}

#[test]
fn malformed_trees_are_rejected() {
    let mut twice = example();
    twice["unit"]["functions"][0]["nodes"][0]["children"] = json!(["larger.max", "larger.a"]);
    assert_eq!(error_of(vec![twice]), "Node larger.a has two parents");

    let mut dangling = example();
    dangling["unit"]["functions"][0]["nodes"][0]["children"] = json!(["larger.ghost"]);
    assert_eq!(
        error_of(vec![dangling]),
        "Function fn:larger: unknown node larger.ghost"
    );
}

#[test]
fn complexity_is_unavailable_without_the_syntax_layer() {
    let mut document = example();
    document["coverage"]["layers"]["ast"]["status"] = json!("unsupported");
    assert!(
        facts_of(document)
            .iter()
            .all(|facts| facts.complexity.is_none())
    );
}

#[test]
fn a_mutated_place_must_say_whether_the_write_escapes() {
    let mut document = example();
    let places = &mut document["unit"]["functions"][2]["places"];
    places[4].as_object_mut().unwrap().remove("escapes");
    assert_eq!(
        error_of(vec![document]),
        "Place count.place.total_write is mutated but does not say whether the write escapes"
    );
}

// Focused fixtures over the example's unit tables.

struct Fixture {
    source: &'static str,
    nodes: Vec<Value>,
    refs: Vec<Value>,
    places: Vec<Value>,
    symbols: Vec<Value>,
    definitions: Vec<Value>,
    types: Vec<Value>,
    parameters: Vec<Value>,
    receiver: Option<Value>,
}

impl Fixture {
    fn new(source: &'static str) -> Self {
        Self {
            source,
            nodes: vec![],
            refs: vec![],
            places: vec![],
            symbols: vec![],
            definitions: vec![],
            types: vec![],
            parameters: vec![],
            receiver: None,
        }
    }

    /// The first `text` at or after the first `after`.
    fn span(&self, text: &str, after: &str) -> Value {
        let base = self
            .source
            .find(after)
            .unwrap_or_else(|| panic!("{after:?} not in source"));
        let start = base
            + self.source[base..]
                .find(text)
                .unwrap_or_else(|| panic!("{text:?} not after {after:?}"));
        json!({"file": "file:lib", "start": start, "end": start + text.len()})
    }

    /// `(id, kind)` at the first `text` after `after`.
    fn node(
        &mut self,
        (id, kind): (&str, &str),
        (text, after): (&str, &str),
        children: &[&str],
    ) -> &mut Value {
        let span = self.span(text, after);
        let node = json!({"id": id, "kind": kind, "original_kind": kind, "span": span, "children": children,
                          "provenance": {"origin": "source"}});
        self.nodes.push(node);
        self.nodes.last_mut().unwrap()
    }

    fn symbol(&mut self, id: &str, name: &str, kind: &str) {
        self.symbols.push(
            json!({"id": id, "name": name, "kind": kind, "scope": "fn:f", "mutable": true,
                         "type": "type:u32"}),
        );
    }

    fn reference(&mut self, node: &str, (target_kind, target): (&str, &str), extra: Value) {
        let mut record = json!({"node": node, "status": "resolved", "target": {"kind": target_kind, "id": target}});
        record
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        self.refs.push(record);
    }

    fn definition(
        &mut self,
        (id, kind): (&str, &str),
        (name, scope): (&str, &str),
        modifiers: &[&str],
    ) {
        self.definitions.push(
            json!({"id": id, "kind": kind, "name": name, "stable_identity": name,
                                     "scope": scope, "modifiers": modifiers}),
        );
    }

    fn document(self, signature_text: &str) -> Value {
        let whole = self.span(self.source.trim_end(), "");
        let signature_span = self.span(signature_text, "");
        let mut document = example();
        let unit = &mut document["unit"];
        unit["files"][0]["content"] = json!(self.source);
        unit["definitions"]
            .as_array_mut()
            .unwrap()
            .extend(self.definitions);
        unit["definitions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "def:f", "kind": "function",
            "name": "example::f", "stable_identity": "example::f", "scope": "unit"}));
        unit["types"].as_array_mut().unwrap().extend(self.types);
        unit["instances"] = json!([]);
        let mut signature = json!({"span": signature_span, "parameters": self.parameters,
            "result": "type:u32", "generics": [], "lifetimes": [], "outlives": [], "modifiers": []});
        if let Some(receiver) = self.receiver {
            signature["receiver"] = receiver;
        }
        unit["functions"] = json!([{"id": "fn:f", "definition": "def:f", "name": "f", "span": whole,
            "signature": signature, "root": self.nodes[0]["id"], "nodes": self.nodes,
            "cfg": {"entry": "entry", "exits": [], "edges": []}, "symbols": self.symbols, "accesses": [],
            "refs": self.refs, "places": self.places, "values": []}]);
        document
    }
}

const FLOW: &str = "fn flow(x: Option<u32>, y: u32) -> u32 {
    let Some(v) = x else { return 0 };
    'outer: loop {
        if y > 1 { break 'outer; } else if y > 2 { continue; } else { break; }
    }
    match v {
        0 if y == 0 => 1,
        _ => flow(None, y)?,
    }
}
";

fn flow_declaration(fx: &mut Fixture) {
    fx.node(("b0", "block"), ("{\n    let", ""), &["s1", "s2", "m"]);
    fx.node(
        ("s1", "local_declaration"),
        ("let Some(v) = x else { return 0 };", ""),
        &["p1", "x1", "e1"],
    );
    fx.node(("p1", "pattern_constructor"), ("Some(v)", "let "), &[]);
    fx.node(("x1", "identifier"), ("x", "= "), &[]);
    fx.node(("e1", "block"), ("{ return 0 }", "else"), &["r1"]);
    fx.node(("r1", "return"), ("return 0", ""), &["z1"]);
    fx.node(("z1", "literal"), ("0", "return "), &[]);
}

fn flow_loop(fx: &mut Fixture) {
    let if_text = "if y > 1 { break 'outer; } else if y > 2 { continue; } else { break; }";
    fx.node(
        ("s2", "expression_statement"),
        ("'outer: loop {", ""),
        &["l1"],
    )["span"]["end"] = json!(FLOW.find("\n    }").unwrap() + 6);
    fx.node(("l1", "loop"), ("'outer: loop {", ""), &["lb"])["label"] = json!("'outer");
    let end = FLOW.find("\n    }").unwrap() + 6;
    fx.nodes.last_mut().unwrap()["span"]["end"] = json!(end);
    fx.node(("lb", "block"), ("{\n        if", "loop"), &["si"])["span"]["end"] = json!(end);
    fx.node(("si", "expression_statement"), (if_text, ""), &["i1"]);
    fx.node(("i1", "if"), (if_text, ""), &["c1", "t1", "i2"]);
    fx.node(("c1", "operator"), ("y > 1", ""), &["y1", "one"])["operator"] = json!("gt");
    fx.node(("y1", "identifier"), ("y", "if "), &[]);
    fx.node(("one", "literal"), ("1", "y > "), &[]);
    fx.node(("t1", "block"), ("{ break 'outer; }", ""), &["sb"]);
    fx.node(
        ("sb", "expression_statement"),
        ("break 'outer;", ""),
        &["k1"],
    );
    fx.node(("k1", "break"), ("break 'outer", ""), &[])["label"] = json!("'outer");
    fx.node(
        ("i2", "if"),
        ("if y > 2 { continue; } else { break; }", ""),
        &["c2", "t2", "e2"],
    );
    fx.node(("c2", "operator"), ("y > 2", ""), &["y2", "two"])["operator"] = json!("gt");
    fx.node(("y2", "identifier"), ("y", "else if "), &[]);
    fx.node(("two", "literal"), ("2", "y > 2"), &[]);
    fx.node(("t2", "block"), ("{ continue; }", ""), &["sc"]);
    fx.node(("sc", "expression_statement"), ("continue;", ""), &["k2"]);
    fx.node(("k2", "continue"), ("continue", ""), &[]);
    fx.node(("e2", "block"), ("{ break; }", ""), &["sk"]);
    fx.node(
        ("sk", "expression_statement"),
        ("break;", "{ break; }"),
        &["k3"],
    );
    fx.node(("k3", "break"), ("break", "{ break; }"), &[]);
}

fn flow_match(fx: &mut Fixture) {
    let start = FLOW.find("match v").unwrap();
    let end = FLOW.rfind("\n    }").unwrap() + 6;
    fx.node(("m", "match"), ("match v", ""), &["v1", "a1", "a2"])["span"]["end"] = json!(end);
    assert!(end > start);
    fx.node(("v1", "identifier"), ("v", "match "), &[]);
    fx.node(
        ("a1", "match_arm"),
        ("0 if y == 0 => 1", ""),
        &["pl", "g1", "o1"],
    );
    fx.node(("pl", "pattern_literal"), ("0", "match v {\n        "), &[]);
    fx.node(("g1", "match_guard"), ("if y == 0", ""), &["c3"]);
    fx.node(("c3", "operator"), ("y == 0", ""), &["y3", "zz"])["operator"] = json!("eq");
    fx.node(("y3", "identifier"), ("y", "0 if "), &[]);
    fx.node(("zz", "literal"), ("0", "y == "), &[]);
    fx.node(("o1", "literal"), ("1", "=> "), &[]);
    fx.node(
        ("a2", "match_arm"),
        ("_ => flow(None, y)?", ""),
        &["pw", "q1"],
    );
    fx.node(("pw", "pattern_wildcard"), ("_", "1,\n"), &[]);
    fx.node(("q1", "propagate"), ("flow(None, y)?", ""), &["f1"])["token"] =
        fx_span(FLOW, "?", "flow(None");
    fx.node(("f1", "call"), ("flow(None, y)", ""), &["fc", "n1", "y4"]);
    fx.node(("fc", "identifier"), ("flow", "_ => "), &[]);
    fx.node(("n1", "identifier"), ("None", ""), &[]);
    fx.node(("y4", "identifier"), ("y", "None, "), &[]);
}

fn fx_span(source: &str, text: &str, after: &str) -> Value {
    let base = source.find(after).unwrap();
    let start = base + source[base..].find(text).unwrap();
    json!({"file": "file:lib", "start": start, "end": start + text.len()})
}

fn flow() -> Value {
    let mut fx = Fixture::new(FLOW);
    flow_declaration(&mut fx);
    flow_loop(&mut fx);
    flow_match(&mut fx);
    fx.symbol("x", "x", "parameter");
    fx.symbol("y", "y", "parameter");
    fx.parameters = vec![
        json!({"position": 0, "symbol": "x", "type": "type:u32"}),
        json!({"position": 1, "symbol": "y", "type": "type:u32"}),
    ];
    fx.reference("f1", ("definition", "def:f"), json!({"dispatch": "static"}));
    fx.reference("n1", ("constructor", "def:None"), json!({}));
    fx.reference("y4", ("symbol", "y"), json!({}));
    fx.definition(
        ("def:None", "variant"),
        ("core::option::Option::None", "library"),
        &[],
    );
    fx.document("fn flow(x: Option<u32>, y: u32) -> u32")
}

#[test]
fn control_flow_constructs_follow_the_profile() {
    let facts = &facts_of(flow())[0];
    use Kind::*;
    // The first arm adds nothing, so it has no contribution.
    let expected = [
        Branch,
        Loop,
        Branch,
        Jump,
        ElseIf,
        Else,
        Selection,
        Guard,
        Alternative,
        Recursion,
        Propagate,
    ];
    assert_eq!(kinds(facts), expected);
    let measured = facts.complexity.as_ref().unwrap();
    assert_eq!((measured.cyclomatic, measured.cognitive), (8, 11));
    let propagate = measured
        .contributions
        .iter()
        .find(|point| point.kind == Propagate)
        .unwrap();
    assert_eq!((propagate.line, propagate.column), (8, 27));
}

#[test]
fn statements_count_block_children_and_bare_arm_bodies() {
    let facts = &facts_of(flow())[0];
    assert_eq!((facts.statements, facts.top_statements), (10, 3));
}

#[test]
fn recursive_calls_are_edges_with_argument_sources() {
    let facts = &facts_of(flow())[0];
    let edge = Edge {
        callee: "example::f".into(),
        line: 8,
        dynamic: false,
        args: vec![ArgSource::Const, ArgSource::Param(1)],
    };
    assert_eq!(facts.edges, [edge]);
    assert!(
        facts.calls.is_empty(),
        "unit callees are not external calls"
    );
}

const EFFECTS: &str = "fn effects(p: fn(), mut v: Vec<u32>) {
    unsafe { raw() };
    raw();
    COUNTER;
    p();
    let f = || 1;
    f();
    missing();
    v.push(1);
}
";

fn effects_statements(fx: &mut Fixture) {
    fx.node(
        ("b", "block"),
        ("{\n    unsafe", ""),
        &["s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8"],
    )["span"]["end"] = json!(EFFECTS.trim_end().len());
    fx.node(
        ("s1", "expression_statement"),
        ("unsafe { raw() };", ""),
        &["u1"],
    );
    fx.node(("u1", "block"), ("unsafe { raw() }", ""), &["c1"])["modifiers"] = json!(["unsafe"]);
    fx.node(("c1", "call"), ("raw()", ""), &["r1"]);
    fx.node(("r1", "identifier"), ("raw", ""), &[]);
    fx.node(("s2", "expression_statement"), ("raw();", "};"), &["c2"]);
    fx.node(("c2", "call"), ("raw()", "};"), &["r2"]);
    fx.node(("r2", "identifier"), ("raw", "};"), &[]);
    fx.node(("s3", "expression_statement"), ("COUNTER;", ""), &["k"]);
    fx.node(("k", "identifier"), ("COUNTER", ""), &[]);
    fx.node(("s4", "expression_statement"), ("p();", ""), &["c4"]);
    fx.node(("c4", "call"), ("p()", ""), &["pp"]);
    fx.node(("pp", "identifier"), ("p", "COUNTER;"), &[])["type"] = json!("type:fnptr");
    fx.node(
        ("s5", "local_declaration"),
        ("let f = || 1;", ""),
        &["fp", "cl"],
    );
    fx.node(("fp", "pattern_binding"), ("f", "let "), &[]);
    fx.node(("cl", "closure"), ("|| 1", ""), &["one"]);
    fx.node(("one", "literal"), ("1", "|| "), &[]);
    fx.node(("s6", "expression_statement"), ("f();", ""), &["c6"]);
    fx.node(("c6", "call"), ("f()", ""), &["ff"]);
    fx.node(("ff", "identifier"), ("f", "1;\n"), &[]);
    fx.node(("s7", "expression_statement"), ("missing();", ""), &["c7"]);
    fx.node(("c7", "call"), ("missing()", ""), &["mm"]);
    fx.node(("mm", "identifier"), ("missing", ""), &[]);
    fx.node(("s8", "expression_statement"), ("v.push(1);", ""), &["c8"]);
    fx.node(("c8", "call"), ("v.push(1)", ""), &["vv", "one8"]);
    fx.node(("vv", "identifier"), ("v", "v.push"), &[]);
    fx.node(("one8", "literal"), ("1", "push("), &[]);
}

fn effects() -> Value {
    let mut fx = Fixture::new(EFFECTS);
    effects_statements(&mut fx);
    fx.symbol("p", "p", "parameter");
    fx.symbol("v", "v", "parameter");
    fx.symbol("f", "f", "local");
    fx.parameters = vec![
        json!({"position": 0, "symbol": "p", "type": "type:fnptr"}),
        json!({"position": 1, "symbol": "v", "type": "type:u32"}),
    ];
    fx.types.push(json!({"id": "type:fnptr", "kind": "function", "spelling": "fn()", "parameters": [], "result": "type:u32"}));
    effects_references(&mut fx);
    fx.places.push(
        json!({"id": "pv", "node": "vv", "root": {"symbol": "v"}, "path": [], "access": "read",
                          "mode": "exclusive", "escapes": false}),
    );
    fx.document("fn effects(p: fn(), mut v: Vec<u32>)")
}

fn effects_references(fx: &mut Fixture) {
    fx.definition(
        ("def:raw", "function"),
        ("ffi::raw", "library"),
        &["unsafe"],
    );
    fx.definition(
        ("def:COUNTER", "static"),
        ("ffi::COUNTER", "library"),
        &["mutable"],
    );
    fx.definition(
        ("def:push", "method"),
        ("alloc::vec::Vec::<T>::push", "library"),
        &[],
    );
    fx.reference(
        "c1",
        ("definition", "def:raw"),
        json!({"dispatch": "static"}),
    );
    fx.reference(
        "c2",
        ("definition", "def:raw"),
        json!({"dispatch": "static"}),
    );
    fx.reference("k", ("definition", "def:COUNTER"), json!({}));
    fx.reference("c4", ("symbol", "p"), json!({"dispatch": "function_value"}));
    fx.reference(
        "c6",
        ("symbol", "f"),
        json!({"dispatch": "function_value", "closure": "cl"}),
    );
    fx.refs
        .push(json!({"node": "c7", "status": "unresolved", "reason": "no such function"}));
    fx.refs.push(json!({"node": "c8", "status": "resolved", "dispatch": "method",
                        "target": {"kind": "library_operation", "id": "def:push", "operation": "rust:Vec.push"}}));
}

#[test]
fn effects_cover_unsafe_code_and_mutable_statics() {
    let facts = &facts_of(effects())[0];
    let effect = |kind, detail: &str, line| EffectEvent {
        kind,
        detail: detail.into(),
        line,
    };
    let expected = [
        effect(EffectKind::Unsafe, "unsafe block", 2),
        effect(EffectKind::Unsafe, "call to unsafe fn ffi::raw", 3),
        effect(EffectKind::Global, "static mut ffi::COUNTER", 4),
    ];
    assert_eq!(facts.effects, expected);
}

#[test]
fn calls_through_values_are_dynamic_unless_they_invoke_a_local_closure() {
    let facts = &facts_of(effects())[0];
    let expected = [
        call("ffi::raw", 2, false),
        call("ffi::raw", 3, false),
        call("<fn pointer>", 5, true),
        call("<unresolved>", 8, true),
        call("alloc::vec::Vec::push", 9, false),
    ];
    assert_eq!(facts.calls, expected);
    assert!(kinds(facts).is_empty(), "a closure nests but adds no score");
    assert_eq!(facts.mutations.len(), 1);
}

const DELEGATE: &str = "fn delegate(&mut self, other: &mut Data) {
    self.helper();
    other.count = 1;
}
";

fn delegate() -> Value {
    let mut fx = Fixture::new(DELEGATE);
    fx.node(("b", "block"), ("{\n    self", ""), &["s1", "s2"])["span"]["end"] =
        json!(DELEGATE.trim_end().len());
    fx.node(
        ("s1", "expression_statement"),
        ("self.helper();", ""),
        &["c1"],
    );
    fx.node(("c1", "call"), ("self.helper()", ""), &["sr"]);
    fx.node(("sr", "identifier"), ("self", "{\n    "), &[]);
    fx.node(
        ("s2", "expression_statement"),
        ("other.count = 1;", ""),
        &["as"],
    );
    fx.node(
        ("as", "assignment"),
        ("other.count = 1", ""),
        &["fa", "one"],
    );
    fx.node(("fa", "field_access"), ("other.count", ""), &["ot"]);
    fx.node(("ot", "identifier"), ("other", "helper"), &[]);
    fx.node(("one", "literal"), ("1", "= "), &[]);
    fx.symbol("self", "self", "receiver");
    fx.symbol("other", "other", "parameter");
    fx.receiver = Some(json!({"symbol": "self", "type": "type:Data"}));
    fx.parameters = vec![json!({"position": 0, "symbol": "other", "type": "type:Data"})];
    fx.types.push(
        json!({"id": "type:Data", "kind": "record", "spelling": "Data", "definition": "def:Data"}),
    );
    fx.definition(("def:Data", "type"), ("example::Data", "unit"), &[]);
    fx.definition(
        ("def:helper", "method"),
        ("example::Data::helper", "unit"),
        &[],
    );
    fx.reference(
        "c1",
        ("definition", "def:helper"),
        json!({"dispatch": "method"}),
    );
    fx.places.push(json!({"id": "ps", "node": "sr", "root": {"symbol": "self"}, "path": [{"deref": true}],
                          "access": "read", "mode": "exclusive", "type": "type:Data", "escapes": true}));
    fx.places.push(json!({"id": "po", "node": "fa", "root": {"symbol": "other"},
                          "path": [{"deref": true}, {"field": "count"}], "access": "write", "mode": "value",
                          "type": "type:u32", "escapes": true}));
    fx.document("fn delegate(&mut self, other: &mut Data)")
}

#[test]
fn workspace_reborrows_are_delegated_and_field_writes_escape() {
    let facts = &facts_of(delegate())[0];
    let event = |root: &str, field: &str, line, escapes, delegated| MutationEvent {
        origin: MutationOrigin::Source,
        root: root.into(),
        field_path: field.into(),
        line,
        escapes,
        delegated,
    };
    assert_eq!(
        facts.mutations,
        [
            event("self", "", 2, false, true),
            event("other", "count", 3, true, false)
        ]
    );
    assert_eq!((facts.params, facts.has_self), (1, true));
    let edge = Edge {
        callee: "example::Data::helper".into(),
        line: 2,
        dynamic: false,
        args: vec![],
    };
    assert_eq!(facts.edges, [edge]);
}

const NOISY: &str = "fn noisy() {
    log!(\"a\");
    trace!();
    let x = 1;
}
";

fn noisy() -> Value {
    let mut fx = Fixture::new(NOISY);
    let call_site = fx.span("log!(\"a\")", "");
    fx.node(("b", "block"), ("{\n    log", ""), &["s1", "gs", "s2"])["span"]["end"] =
        json!(NOISY.trim_end().len());
    fx.node(
        ("s1", "expression_statement"),
        ("log!(\"a\");", ""),
        &["mx"],
    );
    for (id, kind, children) in [
        ("mx", "block", vec!["ms1", "ms2"]),
        ("ms1", "expression_statement", vec!["mif"]),
        ("ms2", "expression_statement", vec![]),
        ("mif", "if", vec!["mc", "mt"]),
        ("mc", "literal", vec![]),
        ("mt", "block", vec![]),
    ] {
        fx.node((id, kind), ("log!(\"a\")", ""), &children)["provenance"] =
            json!({"origin": "macro_expansion", "call_site": call_site, "macro": "log"});
    }
    fx.node(("gs", "expression_statement"), ("trace!();", ""), &[])["provenance"] = json!({"origin": "macro_expansion", "call_site": fx_span(NOISY, "trace!()", ""), "macro": "trace"});
    fx.node(
        ("s2", "local_declaration"),
        ("let x = 1;", ""),
        &["xp", "xv"],
    );
    fx.node(("xp", "pattern_binding"), ("x", "let "), &[]);
    fx.node(("xv", "literal"), ("1", "= "), &[]);
    fx.document("fn noisy()")
}

#[test]
fn desugared_nodes_are_not_statements() {
    let mut document = example();
    let nodes = document["unit"]["functions"][2]["nodes"]
        .as_array_mut()
        .unwrap();
    let block = nodes.iter_mut().find(|n| n["id"] == "count.block").unwrap();
    block["children"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!("count.lowered"));
    nodes.push(json!({"id": "count.lowered", "kind": "expression_statement", "original_kind": "stmt_semi",
                      "children": [], "synthetic": true, "provenance": {"origin": "desugaring", "construct": "for"}}));
    assert_eq!(facts_of(document)[2].statements, 5);
}

#[test]
fn writes_to_library_macro_bindings_are_not_mutations_unless_they_escape() {
    let mut document = example();
    let function = &mut document["unit"]["functions"][2];
    let pattern = function["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "count.total_pat")
        .unwrap();
    pattern["provenance"] = json!({"origin": "macro_expansion", "call_site": pattern["span"],
                                   "macro": "tally", "macro_definition": "def:tally"});
    let definitions = document["unit"]["definitions"].as_array_mut().unwrap();
    definitions.push(
        json!({"id": "def:tally", "kind": "other", "name": "tally::tally",
                            "stable_identity": "tally::tally", "scope": "library"}),
    );
    let roots = |document: &Value| {
        facts_of(document.clone())[2]
            .mutations
            .iter()
            .map(|m| m.root.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(roots(&document), ["iter"]);

    let mut workspace_macro = document.clone();
    let definitions = workspace_macro["unit"]["definitions"]
        .as_array_mut()
        .unwrap();
    definitions.last_mut().unwrap()["scope"] = json!("workspace");
    let message = "a workspace macro's bindings are the developer's";
    assert_eq!(roots(&workspace_macro), ["total", "iter"], "{message}");

    let places = document["unit"]["functions"][2]["places"]
        .as_array_mut()
        .unwrap();
    places
        .iter_mut()
        .find(|p| p["id"] == "count.place.total_write")
        .unwrap()["escapes"] = json!(true);
    assert_eq!(roots(&document), ["total", "iter"]);
}

#[test]
fn generated_code_counts_once_at_its_outermost_node() {
    let facts = &facts_of(noisy())[0];
    assert_eq!((facts.statements, facts.top_statements), (3, 3));
    assert!(
        kinds(facts).is_empty(),
        "macro-expanded control flow is not the developer's"
    );
}

// Comparison with a frontend's own facts.

#[test]
fn comparison_ignores_event_order_but_not_values() {
    use super::compare::compare;
    let reference = facts_of(example());
    assert!(compare(&reference, &reference).differences.is_empty());

    let mut reordered = reference.clone();
    reordered[2].calls.reverse();
    assert!(compare(&reference, &reordered).differences.is_empty());

    let mut changed = reference.clone();
    changed[2].statements += 1;
    let comparison = compare(&reference, &changed);
    let fields: Vec<_> = comparison
        .differences
        .iter()
        .map(|d| (d.function.as_str(), d.field))
        .collect();
    assert_eq!(fields, [("example::count", "statements")]);
    assert_eq!(comparison.by_field()["statements"], 1);
}

#[test]
fn comparison_reports_functions_missing_from_the_document() {
    let reference = facts_of(example());
    let comparison = super::compare::compare(&reference, &reference[..2]);
    assert_eq!(comparison.functions, 3);
    assert_eq!(comparison.differences[0].field, "function");
    assert_eq!(comparison.differences[0].derived, "absent");
}

// Malformed documents the derivation relies on being well formed.

fn with_node(change: impl FnOnce(&mut Value)) -> String {
    let mut document = example();
    let nodes = document["unit"]["functions"][2]["nodes"]
        .as_array_mut()
        .unwrap();
    change(nodes.iter_mut().find(|n| n["id"] == "count.if").unwrap());
    error_of(vec![document])
}

#[test]
fn malformed_constructs_are_rejected() {
    let message = with_node(|node| node["children"] = json!(["count.cond"]));
    assert_eq!(
        message,
        "Node count.if: an if needs a condition and a then-block"
    );

    let mut document = example();
    let nodes = document["unit"]["functions"][2]["nodes"]
        .as_array_mut()
        .unwrap();
    let cond = nodes.iter_mut().find(|n| n["id"] == "count.cond").unwrap();
    cond["children"] = json!(["count.gt"]);
    assert_eq!(
        error_of(vec![document]),
        "Node count.cond: a binary operator needs two operands"
    );

    let mut rooted = example();
    rooted["unit"]["functions"][2]["root"] = json!("count.let");
    assert_eq!(
        error_of(vec![rooted]),
        "Function fn:count: root has a parent"
    );
}

#[test]
fn malformed_unit_tables_are_rejected() {
    let mut duplicate = example();
    let file = duplicate["unit"]["files"][0].clone();
    duplicate["unit"]["files"]
        .as_array_mut()
        .unwrap()
        .push(file);
    assert_eq!(error_of(vec![duplicate]), "Duplicate file ID file:lib");

    let mut definitions = example();
    let definition = definitions["unit"]["definitions"][0].clone();
    definitions["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(definition);
    assert_eq!(
        error_of(vec![definitions]),
        "Duplicate definition ID def:LIMIT"
    );

    let mut cross_file = example();
    let mut other = cross_file["unit"]["files"][0].clone();
    other["id"] = json!("file:other");
    cross_file["unit"]["files"]
        .as_array_mut()
        .unwrap()
        .push(other);
    cross_file["unit"]["functions"][0]["span"]["file"] = json!("file:other");
    assert_eq!(
        error_of(vec![cross_file]),
        "Spans in file:lib and file:other do not form one range"
    );

    let mut function_document = example();
    function_document["document"] = json!("function");
    assert_eq!(
        error_of(vec![function_document]),
        "A run must merge into one bundled unit document"
    );

    let mut bodiless = example();
    bodiless["unit"]
        .as_object_mut()
        .unwrap()
        .remove("functions");
    assert_eq!(
        error_of(vec![bodiless]),
        "The bundled unit document has no function bodies"
    );
}

#[test]
fn or_chains_and_nested_items_follow_the_profile() {
    let mut document = example();
    let nodes = document["unit"]["functions"][2]["nodes"]
        .as_array_mut()
        .unwrap();
    nodes.iter_mut().find(|n| n["id"] == "count.cond").unwrap()["operator"] = json!("or");
    let let_node = nodes.iter_mut().find(|n| n["id"] == "count.let").unwrap();
    let_node["kind"] = json!("item_declaration");
    assert_eq!(
        kinds(&facts_of(document)[2]),
        [Kind::Loop, Kind::Branch, Kind::Or]
    );
}
