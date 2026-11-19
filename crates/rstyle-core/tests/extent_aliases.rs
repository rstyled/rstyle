//! A local name must retain its actual formal size without hiding changes to either name.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::{self, Outcome, Program, Root};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn function(document: &mut Value) -> &mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "count")
        .unwrap()
}

fn node<'a>(function: &'a mut Value, id: &str) -> &'a mut Value {
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn symbol<'a>(function: &'a mut Value, id: &str) -> &'a mut Value {
    function["symbols"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|symbol| symbol["id"] == id)
        .unwrap()
}

fn ty<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["id"] == id)
        .unwrap()
}

fn program(document: Value) -> Program {
    let programs = rstyle_core::cpg::run_bounds(vec![document]).unwrap();
    let (_, input) = programs
        .into_iter()
        .find(|(identity, _)| identity.ends_with("::count"))
        .unwrap();
    let Input::Captured { program } = input else {
        panic!("expected a captured input: {input:?}");
    };
    program
}

/// Insert `let alias = items;` before the existing `for item in alias`.
fn alias() -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    let function = function(&mut document);
    let span = node(function, "count.items")["span"].clone();
    let mut initializer = node(function, "count.items").clone();
    initializer["id"] = json!("alias.init");
    function["nodes"].as_array_mut().unwrap().extend([
        initializer,
        json!({"id":"alias.pattern", "kind":"pattern_binding", "original_kind":"pat_binding",
               "children":[], "type":"type:slice_ref", "span":span,
               "provenance":{"origin":"source"}}),
        json!({"id":"alias.let", "kind":"local_declaration", "original_kind":"stmt_let",
               "children":["alias.pattern","alias.init"], "span":span,
               "provenance":{"origin":"source"}}),
    ]);
    node(function, "count.block")["children"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!("alias.let"));
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id":"alias.symbol", "name":"alias", "kind":"local", "scope":"count.block",
        "node":"alias.pattern", "mutable":false, "type":"type:slice_ref", "declared":span
    }));
    for reference in function["refs"].as_array_mut().unwrap().iter_mut() {
        if reference["node"] == "count.items" {
            reference["target"]["id"] = json!("alias.symbol");
        }
    }
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"alias.init", "status":"resolved",
        "target":{"kind":"symbol","id":"count.items"}
    }));
    for access in function["accesses"].as_array_mut().unwrap().iter_mut() {
        if access["node"] == "count.items" {
            access["symbol"] = json!("alias.symbol");
        }
    }
    document
}

fn alias_extent(program: &Program) -> &rstyle_core::runtime_bounds::Extent {
    program
        .extents
        .iter()
        .find(|extent| extent.input.ends_with("#4"))
        .unwrap()
}

fn assert_local(document: Value) {
    let program = program(document);
    assert_eq!(alias_extent(&program).root, Root::Local(4), "{program:?}");
}

fn assert_unknown(document: Value) {
    let program = program(document);
    let outcome = runtime_bounds::analyze(&program);
    assert!(matches!(outcome, Outcome::Unknown { .. }), "{outcome:?}");
}

/// Alias a record, then measure its slice field: the selected suffix is significant.
fn field_alias() -> Value {
    let mut document = alias();
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"alias.record", "kind":"record", "spelling":"Holder",
               "fields":[{"name":"rows","type":"type:slice_ref"}]}),
        json!({"id":"alias.record_ref", "kind":"reference", "spelling":"&Holder",
               "target":"alias.record", "mode":"shared"}),
    ]);
    let function = function(&mut document);
    function["signature"]["parameters"][0]["type"] = json!("alias.record_ref");
    for id in ["count.items", "alias.symbol"] {
        symbol(function, id)["type"] = json!("alias.record_ref");
    }
    for id in ["alias.pattern", "alias.init"] {
        node(function, id)["type"] = json!("alias.record_ref");
    }
    let mut read = node(function, "alias.init").clone();
    read["id"] = json!("alias.record_read");
    function["nodes"].as_array_mut().unwrap().push(read);
    let field = node(function, "count.items");
    field["kind"] = json!("field_access");
    field["children"] = json!(["alias.record_read"]);
    field["member"] = json!("rows");
    field["member_index"] = json!(0);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "count.items");
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"alias.record_read", "status":"resolved",
        "target":{"kind":"symbol","id":"alias.symbol"}
    }));
    function["accesses"]
        .as_array_mut()
        .unwrap()
        .retain(|access| access["node"] != "count.items");
    document
}

fn initializer_field() -> Value {
    let mut document = field_alias();
    let function = function(&mut document);
    symbol(function, "alias.symbol")["type"] = json!("type:slice_ref");
    node(function, "alias.pattern")["type"] = json!("type:slice_ref");
    let mut base = node(function, "alias.init").clone();
    base["id"] = json!("alias.initializer_base");
    function["nodes"].as_array_mut().unwrap().push(base);
    let initializer = node(function, "alias.init");
    initializer["kind"] = json!("field_access");
    initializer["children"] = json!(["alias.initializer_base"]);
    initializer["member"] = json!("rows");
    initializer["member_index"] = json!(0);
    initializer["type"] = json!("type:slice_ref");
    let read = node(function, "count.items");
    read["kind"] = json!("identifier");
    read["children"] = json!([]);
    read.as_object_mut().unwrap().remove("member");
    read.as_object_mut().unwrap().remove("member_index");
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["node"] != "alias.init");
    function["refs"].as_array_mut().unwrap().extend([
        json!({"node":"alias.initializer_base", "status":"resolved",
               "target":{"kind":"symbol","id":"count.items"}}),
        json!({"node":"count.items", "status":"resolved",
               "target":{"kind":"symbol","id":"alias.symbol"}}),
    ]);
    document
}

fn wrapped_initializer(deref: bool) -> Value {
    let mut document = alias();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"alias.double_ref", "kind":"reference", "spelling":"&&[u32]",
            "mode":"shared", "target":"type:slice_ref"
        }));
    let function = function(&mut document);
    let mut base = node(function, "alias.init").clone();
    base["id"] = json!("alias.initializer_base");
    if deref {
        base["type"] = json!("alias.double_ref");
        function["signature"]["parameters"][0]["type"] = json!("alias.double_ref");
        symbol(function, "count.items")["type"] = json!("alias.double_ref");
    } else {
        symbol(function, "alias.symbol")["type"] = json!("alias.double_ref");
        node(function, "alias.pattern")["type"] = json!("alias.double_ref");
        node(function, "count.items")["type"] = json!("alias.double_ref");
    }
    function["nodes"].as_array_mut().unwrap().push(base);
    let initializer = node(function, "alias.init");
    initializer["children"] = json!(["alias.initializer_base"]);
    if deref {
        initializer["kind"] = json!("operator");
        initializer["operator"] = json!("deref");
    } else {
        initializer["kind"] = json!("reference");
        initializer["type"] = json!("alias.double_ref");
    }
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["node"] != "alias.init");
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"alias.initializer_base", "status":"resolved",
        "target":{"kind":"symbol","id":"count.items"}
    }));
    document
}

#[test]
fn a_shared_formal_alias_keeps_formal_cardinality() {
    let program = program(alias());
    let extent = alias_extent(&program);
    assert_eq!(extent.root, Root::Formal(0));
    assert!(extent.fields.is_empty());
    assert!(matches!(
        runtime_bounds::analyze(&program),
        Outcome::Proven(_)
    ));
}

#[test]
fn a_field_below_a_record_alias_keeps_the_entire_formal_path() {
    let program = program(field_alias());
    let extent = alias_extent(&program);
    assert_eq!(extent.root, Root::Formal(0));
    assert_eq!(extent.fields.len(), 1);
    assert_eq!(extent.fields[0].index, 0);
    assert_eq!(extent.fields[0].name, "rows");
    assert!(matches!(
        runtime_bounds::analyze(&program),
        Outcome::Proven(_)
    ));
}

#[test]
fn a_native_initializer_field_contributes_its_formal_path_prefix() {
    let program = program(initializer_field());
    let extent = alias_extent(&program);
    assert_eq!(extent.root, Root::Formal(0));
    assert_eq!(extent.fields.len(), 1);
    assert_eq!(extent.fields[0].name, "rows");
    assert!(matches!(
        runtime_bounds::analyze(&program),
        Outcome::Proven(_)
    ));
}

#[test]
fn explicit_shared_reference_and_builtin_deref_preserve_cardinality() {
    for deref in [false, true] {
        let program = program(wrapped_initializer(deref));
        assert_eq!(alias_extent(&program).root, Root::Formal(0));
        assert!(matches!(
            runtime_bounds::analyze(&program),
            Outcome::Proven(_)
        ));
    }
}

#[test]
fn an_initializer_field_without_an_index_is_not_a_formal_alias() {
    let mut document = initializer_field();
    node(function(&mut document), "alias.init")
        .as_object_mut()
        .unwrap()
        .remove("member_index");
    assert_local(document);
}

#[test]
fn adjustment_rosters_cannot_exceed_the_alias_work_budget() {
    let mut document = alias();
    node(function(&mut document), "alias.init")["adjustments"] = json!(vec![
        json!({"kind":"auto_ref", "mode":"shared", "target":"type:slice_ref"});
        256
    ]);
    assert_local(document);
}

fn duplicate_scan(function: &mut Value, before: bool) {
    let original = function.clone();
    let nodes: BTreeMap<&str, &Value> = original["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| (node["id"].as_str().unwrap(), node))
        .collect();
    let mut pending = vec!["count.stmt_for"];
    let mut copied = BTreeMap::new();
    while let Some(id) = pending.pop() {
        if copied.insert(id, format!("second.{id}")).is_some() {
            continue;
        }
        pending.extend(
            nodes[id]["children"]
                .as_array()
                .unwrap()
                .iter()
                .map(|child| child.as_str().unwrap()),
        );
    }
    copy_scan_nodes(function, &nodes, &copied);
    let symbols = copy_scan_bindings(function, &original, &copied);
    copy_scan_references(function, &original, &copied, &symbols);
    node(function, "second.count.items")
        .as_object_mut()
        .unwrap()
        .remove("member_index");
    node(function, "count.block")["children"]
        .as_array_mut()
        .unwrap()
        .insert(if before { 2 } else { 3 }, json!("second.count.stmt_for"));
}

fn copy_scan_nodes(
    function: &mut Value,
    nodes: &BTreeMap<&str, &Value>,
    copied: &BTreeMap<&str, String>,
) {
    for (id, new_id) in copied {
        let mut node = nodes[id].clone();
        node["id"] = json!(new_id);
        for child in node["children"].as_array_mut().unwrap() {
            *child = json!(copied[child.as_str().unwrap()]);
        }
        function["nodes"].as_array_mut().unwrap().push(node);
    }
}

fn copy_scan_bindings<'a>(
    function: &mut Value,
    original: &'a Value,
    copied: &BTreeMap<&str, String>,
) -> BTreeMap<&'a str, String> {
    let symbols: BTreeMap<&str, String> = original["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|symbol| {
            symbol["node"]
                .as_str()
                .is_some_and(|id| copied.contains_key(id))
        })
        .map(|symbol| {
            let id = symbol["id"].as_str().unwrap();
            (id, format!("second.{id}"))
        })
        .collect();
    for symbol in original["symbols"].as_array().unwrap() {
        let Some(new_id) = symbols.get(symbol["id"].as_str().unwrap()) else {
            continue;
        };
        let mut symbol = symbol.clone();
        symbol["id"] = json!(new_id);
        symbol["node"] = json!(copied[symbol["node"].as_str().unwrap()]);
        if let Some(scope) = symbol["scope"].as_str().and_then(|scope| copied.get(scope)) {
            symbol["scope"] = json!(scope);
        }
        function["symbols"].as_array_mut().unwrap().push(symbol);
    }
    symbols
}

fn copy_scan_references(
    function: &mut Value,
    original: &Value,
    copied: &BTreeMap<&str, String>,
    symbols: &BTreeMap<&str, String>,
) {
    for reference in original["refs"].as_array().unwrap() {
        let Some(new_node) = copied.get(reference["node"].as_str().unwrap()) else {
            continue;
        };
        let mut reference = reference.clone();
        reference["node"] = json!(new_node);
        if let Some(target) = reference["target"]["id"]
            .as_str()
            .and_then(|id| symbols.get(id))
        {
            reference["target"]["id"] = json!(target);
        }
        function["refs"].as_array_mut().unwrap().push(reference);
    }
}

#[test]
fn every_read_of_a_shared_alias_key_must_have_certified_projection_metadata() {
    for before in [false, true] {
        let mut document = field_alias();
        duplicate_scan(function(&mut document), before);
        assert_local(document);
    }
}

#[test]
fn an_absent_projection_index_cannot_default_into_formal_provenance() {
    let mut document = field_alias();
    node(function(&mut document), "count.items")
        .as_object_mut()
        .unwrap()
        .remove("member_index");
    assert_local(document);
}

#[test]
fn a_projection_needs_a_complete_structural_field_roster() {
    let mut document = field_alias();
    ty(&mut document, "alias.record")
        .as_object_mut()
        .unwrap()
        .remove("fields");
    assert_local(document);
}

#[test]
fn a_property_reference_is_not_a_native_field_certificate() {
    let mut document = field_alias();
    function(&mut document)["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node":"count.items", "status":"resolved",
            "target":{"kind":"definition","id":"def:count"}
        }));
    assert_local(document);
}

#[test]
fn a_field_type_must_match_the_compiler_projection_type() {
    let mut document = field_alias();
    ty(&mut document, "alias.record")["fields"][0]["type"] = json!("type:u32");
    assert_local(document);
}

#[test]
fn a_mutable_formal_binding_does_not_supply_entry_size_provenance() {
    let mut document = alias();
    symbol(function(&mut document), "count.items")["mutable"] = json!(true);
    assert_local(document);
}

#[test]
fn unknown_adjustments_do_not_supply_alias_provenance() {
    let mut document = alias();
    node(function(&mut document), "alias.init")["adjustments"] = json!([
        {"kind":"test:opaque_view", "target":"type:slice_ref"}
    ]);
    assert_local(document);
}

#[test]
fn missing_adjustment_targets_do_not_supply_alias_provenance() {
    let mut document = alias();
    node(function(&mut document), "alias.init")["adjustments"] = json!([
        {"kind":"auto_ref", "mode":"shared"}
    ]);
    assert_local(document);
}

#[test]
fn initializer_and_binding_types_must_agree() {
    let mut document = alias();
    node(function(&mut document), "alias.init")["type"] = json!("type:u32_ref");
    assert_local(document);
}

fn borrow_original(document: &mut Value, before: bool) {
    let function = function(document);
    let span = node(function, "alias.init")["span"].clone();
    let mut original = node(function, "alias.init").clone();
    original["id"] = json!("alias.original");
    function["nodes"].as_array_mut().unwrap().extend([
        original,
        json!({"id":"alias.borrow", "kind":"reference", "original_kind":"expr_addr_of",
               "children":["alias.original"], "modifiers":["mutable"], "span":span,
               "type":"type:slice_ref", "provenance":{"origin":"source"}}),
        json!({"id":"alias.borrow_statement", "kind":"expression_statement",
               "original_kind":"stmt_semi", "children":["alias.borrow"], "span":span,
               "provenance":{"origin":"source"}}),
    ]);
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"alias.original", "status":"resolved",
        "target":{"kind":"symbol","id":"count.items"}
    }));
    node(function, "count.block")["children"]
        .as_array_mut()
        .unwrap()
        .insert(if before { 1 } else { 2 }, json!("alias.borrow_statement"));
}

#[test]
fn a_mutable_borrow_of_the_original_before_alias_initialization_rejects() {
    let mut document = alias();
    borrow_original(&mut document, true);
    assert_unknown(document);
}

#[test]
fn a_mutable_borrow_of_the_original_after_alias_initialization_rejects() {
    let mut document = alias();
    borrow_original(&mut document, false);
    assert_unknown(document);
}

#[test]
fn writes_to_the_original_are_still_rejected() {
    let mut document = alias();
    borrow_original(&mut document, true);
    let function = function(&mut document);
    let mut rhs = node(function, "alias.init").clone();
    rhs["id"] = json!("alias.write_rhs");
    function["nodes"].as_array_mut().unwrap().push(rhs);
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"alias.write_rhs", "status":"resolved",
        "target":{"kind":"symbol","id":"count.items"}
    }));
    let write = node(function, "alias.borrow");
    write["kind"] = json!("assignment");
    write["children"] = json!(["alias.original", "alias.write_rhs"]);
    write["modifiers"] = json!([]);
    assert_unknown(document);
}

fn caller(mut template: Value) -> Value {
    let span = json!({"file":"file:alias-caller", "start":0, "end":58});
    template["id"] = json!("fn:alias-caller");
    template["definition"] = json!("def:alias-caller");
    template["name"] = json!("alias_caller");
    template["span"] = span.clone();
    template["root"] = json!("caller.block");
    template["signature"]["span"] = span.clone();
    template["signature"]["parameters"][0]["symbol"] = json!("caller.other");
    template["symbols"] = json!([
        {"id":"caller.other", "name":"other", "kind":"parameter", "scope":"fn:alias-caller",
         "type":"type:slice_ref", "mutable":false, "declared":span}
    ]);
    template["nodes"] = json!([
        {"id":"caller.block", "kind":"block", "original_kind":"expr_block",
         "children":["caller.call"], "span":span, "type":"type:u32", "provenance":{"origin":"source"}},
        {"id":"caller.call", "kind":"call", "original_kind":"expr_call",
         "children":["caller.callee","caller.argument"], "span":span, "type":"type:u32",
         "provenance":{"origin":"source"}},
        {"id":"caller.callee", "kind":"identifier", "original_kind":"expr_path", "children":[],
         "span":{"file":"file:alias-caller","start":44,"end":49},
         "type":"caller.callee_type", "provenance":{"origin":"source"}},
        {"id":"caller.argument", "kind":"identifier", "original_kind":"expr_path", "children":[],
         "span":{"file":"file:alias-caller","start":50,"end":55},
         "type":"type:slice_ref", "provenance":{"origin":"source"}}
    ]);
    template["refs"] = json!([
        {"node":"caller.call", "status":"resolved", "dispatch":"static",
         "target":{"kind":"definition","id":"def:count"}},
        {"node":"caller.callee", "status":"resolved", "target":{"kind":"definition","id":"def:count"}},
        {"node":"caller.argument", "status":"resolved", "target":{"kind":"symbol","id":"caller.other"}}
    ]);
    template["accesses"] =
        json!([{"node":"caller.argument", "symbol":"caller.other", "mode":"read"}]);
    template["places"] = json!([]);
    template["values"] = json!([]);
    template["cfg"] =
        json!({"entry":"caller.entry", "exits":[{"id":"caller.exit","kind":"normal"}], "edges":[]});
    template
}

fn with_caller(mut document: Value) -> Value {
    let caller = caller(function(&mut document).clone());
    let compilation = document["unit"]["id"].clone();
    for definition in document["unit"]["definitions"].as_array_mut().unwrap() {
        if definition["id"] == "def:count" {
            definition["compilation"] = compilation.clone();
            definition["callable"] = json!({
                "parameters":["type:slice_ref"], "result":"type:u32",
                "generics":0, "modifiers":[]
            });
        }
    }
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(caller);
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"def:alias-caller", "kind":"function", "name":"example::alias_caller",
            "stable_identity":"example::alias_caller", "scope":"unit"
        }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"caller.callee_type", "kind":"function", "spelling":"fn(&[u32]) -> u32 {count}",
            "definition":"def:count", "parameters":["type:slice_ref"], "result":"type:u32"
        }));
    document["unit"]["files"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"file:alias-caller", "path":"src/alias_caller.rs", "origin":"source",
            "digest":"sha256:2e334a7ba0d2178e7a38362e51ca1cdb80e57e627958da13101427e0d1e7d4c0",
            "content":"pub fn alias_caller(other: &[u32]) -> u32 { count(other) }\n"
        }));
    document
}

fn composed_caller(document: Value) -> Value {
    let demand = rstyle_core::cpg::records::Demand {
        bounds: true,
        ..Default::default()
    };
    let records = rstyle_core::cpg::run_records(vec![document], demand).unwrap();
    let report = serde_json::to_value(rstyle_core::cpg::bounds::report::from_records(
        &records, None,
    ))
    .unwrap();
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "alias_caller")
        .unwrap()
        .clone()
}

#[test]
fn composition_substitutes_the_callers_size_for_the_callee_local_alias() {
    let row = composed_caller(with_caller(alias()));
    assert!(row["composed"].get("Proven").is_some(), "{row}");
    assert_eq!(
        row["composed"]["Proven"]["loop_body_entries"],
        json!({"Input":"len(other)#0"})
    );
    assert_eq!(
        row["composed"]["Proven"]["workspace_invocations"],
        json!({"Const":1})
    );
    assert!(
        row["composed"]["Proven"].get("opaque_factors").is_none(),
        "{row}"
    );
}

fn with_alias_argument(mut document: Value) -> Value {
    let function = document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "alias_caller")
        .unwrap();
    let mut initializer = node(function, "caller.argument").clone();
    let span = initializer["span"].clone();
    initializer["id"] = json!("caller.alias_init");
    function["nodes"].as_array_mut().unwrap().extend([
        initializer,
        json!({"id":"caller.alias_pattern", "kind":"pattern_binding", "original_kind":"pat_binding",
               "children":[], "type":"type:slice_ref", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":"caller.alias_let", "kind":"local_declaration", "original_kind":"stmt_let",
               "children":["caller.alias_pattern","caller.alias_init"], "span":span,
               "provenance":{"origin":"source"}})
    ]);
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id":"caller.alias", "name":"alias", "kind":"local", "scope":"caller.block",
        "node":"caller.alias_pattern", "mutable":false, "type":"type:slice_ref", "declared":span
    }));
    for reference in function["refs"].as_array_mut().unwrap().iter_mut() {
        if reference["node"] == "caller.argument" {
            reference["target"]["id"] = json!("caller.alias");
        }
    }
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"caller.alias_init", "status":"resolved",
        "target":{"kind":"symbol","id":"caller.other"}
    }));
    for access in function["accesses"].as_array_mut().unwrap().iter_mut() {
        if access["node"] == "caller.argument" {
            access["symbol"] = json!("caller.alias");
        }
    }
    node(function, "caller.block")["children"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!("caller.alias_let"));
    document
}

#[test]
fn passing_a_local_alias_to_a_callee_preserves_composed_cardinality() {
    let row = composed_caller(with_alias_argument(with_caller(alias())));
    assert!(row["composed"].get("Proven").is_some(), "{row}");
    assert_eq!(
        row["composed"]["Proven"]["loop_body_entries"],
        json!({"Input":"len(other)#1"})
    );
    assert_eq!(
        row["composed"]["Proven"]["workspace_invocations"],
        json!({"Const":1})
    );
    assert!(
        row["composed"]["Proven"].get("opaque_factors").is_none(),
        "{row}"
    );
}

fn tuple_alias(initializer: bool) -> Value {
    let mut document = if initializer {
        initializer_field()
    } else {
        field_alias()
    };
    let tuple = ty(&mut document, "alias.record");
    tuple["kind"] = json!("tuple");
    tuple["items"] = json!(["type:u32", "type:slice_ref"]);
    tuple.as_object_mut().unwrap().remove("fields");
    let field = node(
        function(&mut document),
        if initializer {
            "alias.init"
        } else {
            "count.items"
        },
    );
    field["member"] = json!("1");
    field["member_index"] = json!(1);
    document
}

#[test]
fn tuple_fields_keep_their_positional_formal_provenance() {
    for initializer in [false, true] {
        let program = program(tuple_alias(initializer));
        let extent = alias_extent(&program);
        assert_eq!(extent.root, Root::Formal(0));
        assert_eq!(extent.fields.len(), 1);
        assert_eq!(extent.fields[0].index, 1);
        assert_eq!(extent.fields[0].name, "1");
        assert!(matches!(
            runtime_bounds::analyze(&program),
            Outcome::Proven(_)
        ));
    }
}

#[test]
fn tuple_projections_need_complete_matching_positional_facts() {
    let mut missing = tuple_alias(false);
    ty(&mut missing, "alias.record")
        .as_object_mut()
        .unwrap()
        .remove("items");
    assert_local(missing);
    let mut named = tuple_alias(false);
    node(function(&mut named), "count.items")["member"] = json!("rows");
    assert_local(named);
}

#[test]
fn an_exclusive_reference_inside_a_shared_field_chain_is_not_a_stable_alias() {
    let mut document = field_alias();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"alias.exclusive_record_ref", "kind":"reference", "spelling":"&mut Holder",
            "target":"alias.record", "mode":"exclusive"
        }));
    ty(&mut document, "alias.record_ref")["target"] = json!("alias.exclusive_record_ref");
    assert_unknown(document);
}

#[test]
fn incomplete_formal_type_facts_cannot_supply_alias_provenance() {
    let mut document = alias();
    symbol(function(&mut document), "count.items")
        .as_object_mut()
        .unwrap()
        .remove("type");
    assert_local(document);
}

fn with_local_source(mut document: Value) -> Value {
    let function = function(&mut document);
    let mut initializer = node(function, "alias.init").clone();
    let span = initializer["span"].clone();
    initializer["id"] = json!("alias.first_init");
    function["nodes"].as_array_mut().unwrap().extend([
        initializer,
        json!({"id":"alias.first_pattern", "kind":"pattern_binding", "original_kind":"pat_binding",
               "children":[], "type":"type:slice_ref", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":"alias.first_let", "kind":"local_declaration", "original_kind":"stmt_let",
               "children":["alias.first_pattern","alias.first_init"], "span":span,
               "provenance":{"origin":"source"}})
    ]);
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id":"alias.first", "name":"first", "kind":"local", "scope":"count.block",
        "node":"alias.first_pattern", "mutable":false, "type":"type:slice_ref", "declared":span
    }));
    for reference in function["refs"].as_array_mut().unwrap().iter_mut() {
        if reference["node"] == "alias.init" {
            reference["target"]["id"] = json!("alias.first");
        }
    }
    function["refs"].as_array_mut().unwrap().push(json!({
        "node":"alias.first_init", "status":"resolved",
        "target":{"kind":"symbol","id":"count.items"}
    }));
    node(function, "count.block")["children"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!("alias.first_let"));
    document
}

#[test]
fn aliases_of_other_locals_do_not_get_unverified_formal_provenance() {
    assert_local(with_local_source(alias()));
}

fn reference_layers(document: &mut Value, target: &str, count: usize) -> String {
    let mut types = Vec::new();
    let mut inner = target.to_owned();
    for index in 0..count {
        let id = format!("alias.deep_ref.{index}");
        types.push(
            json!({"id":id, "kind":"reference", "spelling":"shared nested reference",
                          "target":inner, "mode":"shared"}),
        );
        inner = id;
    }
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .extend(types);
    inner
}

fn deep_initializer() -> Value {
    let mut document = alias();
    let inner = reference_layers(&mut document, "type:slice_ref", 64);
    let function = function(&mut document);
    function["signature"]["parameters"][0]["type"] = json!(inner);
    symbol(function, "count.items")["type"] = json!(inner);
    let mut base = node(function, "alias.init").clone();
    base["id"] = json!("alias.deep_base");
    base["type"] = json!(inner);
    let mut nodes = vec![base];
    let mut child = "alias.deep_base".to_owned();
    for index in (0..64).rev() {
        let id = if index == 0 {
            "alias.init".to_owned()
        } else {
            format!("alias.deep_deref.{index}")
        };
        let target = if index == 0 {
            "type:slice_ref".to_owned()
        } else {
            format!("alias.deep_ref.{}", index - 1)
        };
        let mut deref = node(function, "alias.init").clone();
        deref["id"] = json!(id);
        deref["kind"] = json!("operator");
        deref["operator"] = json!("deref");
        deref["type"] = json!(target);
        deref["children"] = json!([child]);
        nodes.push(deref);
        child = id;
    }
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["id"] != "alias.init");
    function["nodes"].as_array_mut().unwrap().extend(nodes);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["node"] == "alias.init")
        .unwrap()["node"] = json!("alias.deep_base");
    document
}

#[test]
fn deeply_nested_builtin_dereferences_keep_local_size_without_formal_credit() {
    assert_local(deep_initializer());
}

fn add_budget_alias(function: &mut Value, index: usize) {
    let id = |suffix: &str| format!("budget.{index}.{suffix}");
    let span = node(function, "count.block")["span"].clone();
    function["nodes"].as_array_mut().unwrap().extend([
        json!({"id":id("pattern"), "kind":"pattern_binding", "original_kind":"pat_binding",
               "children":[], "type":"type:slice_ref", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":id("initializer"), "kind":"identifier", "original_kind":"expr_path",
               "children":[], "type":"type:slice_ref", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":id("declaration"), "kind":"local_declaration", "original_kind":"stmt_let",
               "children":[id("pattern"),id("initializer")], "span":span, "provenance":{"origin":"source"}}),
        json!({"id":id("read"), "kind":"identifier", "original_kind":"expr_path",
               "children":[], "type":"type:slice_ref", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":id("length"), "kind":"call", "original_kind":"expr_method_call",
               "children":[id("read")], "type":"budget.usize", "span":span, "provenance":{"origin":"source"}}),
        json!({"id":id("statement"), "kind":"expression_statement", "original_kind":"stmt_semi",
               "children":[id("length")], "span":span, "provenance":{"origin":"source"}})
    ]);
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id":id("symbol"), "name":format!("alias_{index}"), "kind":"local", "scope":"count.block",
        "node":id("pattern"), "mutable":false, "type":"type:slice_ref", "declared":span
    }));
    function["refs"].as_array_mut().unwrap().extend([
        json!({"node":id("initializer"), "status":"resolved", "target":{"kind":"symbol","id":"count.items"}}),
        json!({"node":id("read"), "status":"resolved", "target":{"kind":"symbol","id":id("symbol")}}),
        json!({"node":id("length"), "status":"resolved", "dispatch":"method",
               "target":{"kind":"definition","id":"budget.len"},
               "calls_back":{"value":false,"reason":"Slice length takes no callable arguments"}})
    ]);
    node(function, "count.block")["children"]
        .as_array_mut()
        .unwrap()
        .extend([json!(id("declaration")), json!(id("statement"))]);
}

fn many_selected_aliases(count: usize) -> Value {
    let mut document = alias();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"budget.usize", "kind":"integer", "spelling":"usize", "bits":64,
            "signed":false, "pointer_sized":true
        }));
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id":"budget.len", "kind":"function", "name":"slice::len", "stable_identity":"slice::len",
        "scope":"library", "standard":true, "operation":"rust:slice.len",
        "callable":{"owner_type":"type:u32_slice", "receiver":"type:slice_ref",
                    "parameters":[], "result":"budget.usize", "generics":0}
    }));
    let function = function(&mut document);
    let mut root = node(function, "count.block").clone();
    root["children"] = json!([]);
    root["type"] = json!("type:unit");
    function["nodes"] = json!([root]);
    function["signature"]["result"] = json!("type:unit");
    function["symbols"]
        .as_array_mut()
        .unwrap()
        .retain(|symbol| symbol["id"] == "count.items");
    for table in ["refs", "accesses", "places", "values"] {
        function[table] = json!([]);
    }
    function["cfg"] =
        json!({"entry":"entry", "exits":[{"id":"count.exit","kind":"normal"}], "edges":[]});
    for index in 0..count {
        add_budget_alias(function, index);
    }
    document
}

fn assert_frame_budget(document: Value) {
    let (_, input) = rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .into_iter()
        .find(|(identity, _)| identity.ends_with("::count"))
        .unwrap();
    assert!(matches!(input, Input::BudgetExhausted { .. }), "{input:?}");
}

#[test]
fn protecting_formal_origins_also_respects_the_extent_budget() {
    assert_frame_budget(many_selected_aliases(
        rstyle_core::advisory::runtime_bounds::MAX_VARIABLES,
    ));
}

#[test]
fn too_many_selected_aliases_stop_before_formal_origin_protection() {
    assert_frame_budget(many_selected_aliases(
        rstyle_core::advisory::runtime_bounds::MAX_VARIABLES + 1,
    ));
}

#[test]
fn a_static_slice_alias_is_not_an_entry_formal() {
    let mut document = alias();
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"alias.static", "kind":"static", "name":"STATIC_ITEMS",
            "stable_identity":"example::STATIC_ITEMS", "scope":"unit"
        }));
    let function = function(&mut document);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "alias.init")
        .unwrap()["target"] = json!({"kind":"definition","id":"alias.static"});
    assert_local(document);
}

#[test]
fn an_expression_block_is_not_a_certified_direct_place_initializer() {
    let mut document = alias();
    let function = function(&mut document);
    let mut value = node(function, "alias.init").clone();
    value["id"] = json!("alias.block_value");
    function["nodes"].as_array_mut().unwrap().push(value);
    let initializer = node(function, "alias.init");
    initializer["kind"] = json!("block");
    initializer["children"] = json!(["alias.block_value"]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "alias.init")
        .unwrap()["node"] = json!("alias.block_value");
    assert_local(document);
}

#[test]
fn missing_base_type_information_cannot_certify_a_reference_projection() {
    let mut document = wrapped_initializer(false);
    node(function(&mut document), "alias.initializer_base")
        .as_object_mut()
        .unwrap()
        .remove("type");
    assert_local(document);
}

#[test]
fn unnamed_record_storage_does_not_supply_a_named_formal_field_path() {
    let mut document = field_alias();
    ty(&mut document, "alias.record")["fields"][0]["name"] = json!("");
    node(function(&mut document), "count.items")["member"] = json!("");
    assert_local(document);
}

#[test]
fn a_normalized_opaque_field_owner_stays_local_without_a_structural_path_model() {
    let mut document = field_alias();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"alias.opaque_owner", "kind":"opaque", "spelling":"OpaqueHolderAlias",
            "normalized":"alias.record"
        }));
    ty(&mut document, "alias.record_ref")["target"] = json!("alias.opaque_owner");
    node(function(&mut document), "alias.record_read")["adjustments"] = json!([
        {"kind":"auto_deref", "target":"alias.record"}
    ]);
    assert_local(document);
}

#[test]
fn an_initializer_field_cannot_cross_an_exclusive_reference() {
    let mut document = initializer_field();
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"alias.exclusive_record_ref", "kind":"reference", "spelling":"&mut Holder",
            "target":"alias.record", "mode":"exclusive"
        }));
    ty(&mut document, "alias.record_ref")["target"] = json!("alias.exclusive_record_ref");
    assert_local(document);
}

#[test]
fn resolving_a_native_field_respects_the_reference_depth_budget() {
    let mut document = initializer_field();
    let reference = reference_layers(&mut document, "alias.record", 64);
    let function = function(&mut document);
    function["signature"]["parameters"][0]["type"] = json!(reference);
    symbol(function, "count.items")["type"] = json!(reference);
    node(function, "alias.initializer_base")["type"] = json!(reference);
    assert_local(document);
}

#[test]
fn a_helper_return_is_not_assumed_to_have_its_arguments_cardinality() {
    let mut document = alias();
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id":"alias.choose", "kind":"function", "name":"choose", "stable_identity":"example::choose",
        "scope":"workspace", "callable":{"parameters":["type:slice_ref"],"result":"type:slice_ref","generics":0}
    }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"alias.choose_type", "kind":"function", "spelling":"fn(&[u32]) -> &[u32] {choose}",
            "definition":"alias.choose", "parameters":["type:slice_ref"], "result":"type:slice_ref"
        }));
    let function = function(&mut document);
    let mut argument = node(function, "alias.init").clone();
    argument["id"] = json!("alias.choose_argument");
    let mut callee = argument.clone();
    callee["id"] = json!("alias.choose_callee");
    callee["type"] = json!("alias.choose_type");
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .extend([argument, callee]);
    let call = node(function, "alias.init");
    call["kind"] = json!("call");
    call["children"] = json!(["alias.choose_callee", "alias.choose_argument"]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["node"] != "alias.init");
    function["refs"].as_array_mut().unwrap().extend([
        json!({"node":"alias.init", "status":"resolved", "dispatch":"static", "target":{"kind":"definition","id":"alias.choose"}}),
        json!({"node":"alias.choose_callee", "status":"resolved", "target":{"kind":"definition","id":"alias.choose"}}),
        json!({"node":"alias.choose_argument", "status":"resolved", "target":{"kind":"symbol","id":"count.items"}})
    ]);
    let program = program(document);
    assert_eq!(alias_extent(&program).root, Root::Local(4));
    let Outcome::Proven(assessment) = runtime_bounds::analyze(&program) else {
        panic!("{program:?}")
    };
    assert_eq!(
        assessment.calls.len(),
        1,
        "The initializer call must remain counted"
    );
}
