use super::{Budget, Cleanup, Invocation};
use crate::cpg::model::Document;
use crate::cpg::tree::{Body, Tables};
use crate::runtime_bounds::contracts::Contracts;
use serde_json::{Value, json};

const ROLE: &str = "schema = 'rstyle.library-contracts/1'\nnamespace = 't'\n\
    [[callback_free]]\nform = 'Function'\narity = 2\ncleanup_free = [0, 1]\n\
    types = [{ argument = 1, keys = ['rust:u32'] }]\n\
    basis = 'A generic first value is moved or discarded without invoking a callable'\n\
    operations = ['larger']\n";

fn document() -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../testdata/example.cpg.json")).unwrap();
    let definition = definition(&mut document);
    definition["standard"] = json!(true);
    definition["scope"] = json!("library");
    definition["operation"] = json!("t:larger");
    definition["callable"]["parameters"] = json!(["type:T", "type:u32"]);
    definition["callable"]["result"] = json!("type:u32");
    document
}

fn top(document: &mut Value) -> &mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "top")
        .unwrap()
}

fn definition(document: &mut Value) -> &mut Value {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "def:larger")
        .unwrap()
}

fn node<'d>(document: &'d mut Value, id: &str) -> &'d mut Value {
    top(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn reference<'d>(document: &'d mut Value, id: &str) -> &'d mut Value {
    top(document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == id)
        .unwrap()
}

fn ty(document: &mut Value, id: &str, kind: &str, fields: Value) {
    let mut entry = json!({"id":id,"kind":kind,"spelling":id});
    entry
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(entry);
}

fn nominal(document: &mut Value, id: &str, kind: &str, fields: Value) {
    let definition = format!("def:{id}");
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":definition,"kind":"type","name":id,"stable_identity":id,
            "scope":"workspace","destructor":false
        }));
    let mut fields = fields;
    fields["definition"] = json!(definition);
    ty(document, id, kind, fields);
}

fn drop_type(document: &mut Value) {
    nominal(document, "test:drop", "record", json!({"fields":[]}));
    let definitions = document["unit"]["definitions"].as_array_mut().unwrap();
    definitions.last_mut().unwrap()["destructor"] = json!(true);
}

fn with_invocation<R>(document: Value, check: impl FnOnce(&Invocation<'_, '_>) -> R) -> R {
    let document: Document = serde_json::from_value(document).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let function = unit
        .functions
        .as_ref()
        .unwrap()
        .iter()
        .find(|function| function.name == "top")
        .unwrap();
    let body = Body::new(function).unwrap();
    let tables = Tables::new(unit).unwrap();
    let contracts = Contracts::parse_all(&[ROLE]).unwrap();
    check(&Invocation {
        body: &body,
        tables: &tables,
        call: body.node("top.larger").unwrap(),
        callee: tables.definition("def:larger").unwrap(),
        contracts: &contracts,
    })
}

fn certified(document: Value) -> bool {
    with_invocation(document, |invocation| invocation.certified())
}

fn cleanup(document: Value, id: &str) -> bool {
    with_invocation(document, |invocation| {
        Cleanup(invocation.tables)
            .remaining(
                invocation.tables.ty(id).unwrap(),
                Budget {
                    depth: 32,
                    work: 128,
                },
            )
            .is_some()
    })
}

#[test]
fn an_injected_function_role_refines_an_unnamed_may_call_flag() {
    let mut document = document();
    reference(&mut document, "top.larger")["calls_back"] = json!({
        "value":true,"reason":"generic argument may contain a callable"
    });
    assert!(certified(document.clone()));
    reference(&mut document, "top.larger")["dispatch"] = json!("builtin");
    assert!(certified(document.clone()));
    with_invocation(document, |invocation| {
        assert!(!super::callback_free(
            invocation.body,
            invocation.tables,
            invocation.call,
            invocation.callee
        ));
    });
}

#[test]
fn enumerated_callbacks_and_driven_implementations_are_never_discarded() {
    for runs in [json!([]), json!([{"kind":"definition","id":"def:top"}])] {
        let mut document = document();
        reference(&mut document, "top.larger")["calls_back"] = json!({
            "value":true,"reason":"the frontend enumerated execution","runs":runs
        });
        assert!(!certified(document));
    }
    for drives in [json!([]), json!(["def:top"])] {
        let mut document = document();
        reference(&mut document, "top.larger")["drives"] = drives;
        assert!(!certified(document));
    }
}

#[test]
fn a_function_certificate_requires_the_exact_resolved_library_definition() {
    let mut absent = document();
    top(&mut absent)["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "top.larger");
    assert!(!certified(absent));
    for changed in [
        "status",
        "target",
        "selected",
        "arguments",
        "standard",
        "scope",
        "operation",
        "implements",
    ] {
        let mut document = document();
        match changed {
            "status" => reference(&mut document, "top.larger")["status"] = json!("unresolved"),
            "target" => reference(&mut document, "top.larger")["target"]["id"] = json!("def:top"),
            "selected" => {
                reference(&mut document, "top.larger")["selected"] =
                    json!({"kind":"definition","id":"def:top"})
            }
            "arguments" => reference(&mut document, "top.larger")["arguments"] = json!([]),
            "standard" => definition(&mut document)["standard"] = json!(false),
            "scope" => definition(&mut document)["scope"] = json!("workspace"),
            "operation" => definition(&mut document)["operation"] = json!("t:unregistered"),
            _ => definition(&mut document)["implements"] = json!("def:Ord.max"),
        }
        assert!(!certified(document), "{changed}");
    }
    let mut selected = document();
    reference(&mut selected, "top.larger")["selected"] =
        json!({"kind":"definition","id":"def:larger"});
    assert!(certified(selected));
    let mut interface = document();
    definition(&mut interface)["owner"] = json!("def:Ord");
    assert!(!certified(interface));
}

#[test]
fn callable_layout_dispatch_and_written_arity_must_agree() {
    for dispatch in ["method", "dynamic", "function_value", "overloaded"] {
        let mut document = document();
        reference(&mut document, "top.larger")["dispatch"] = json!(dispatch);
        assert!(!certified(document), "{dispatch}");
    }
    for changed in [
        "parameters",
        "receiver",
        "variadic",
        "definition_variadic",
        "kind",
        "callable",
        "arity",
        "call_kind",
    ] {
        let mut document = document();
        match changed {
            "parameters" => definition(&mut document)["callable"]["parameters"] = json!(["type:T"]),
            "receiver" => definition(&mut document)["callable"]["receiver"] = json!("type:u32"),
            "variadic" => definition(&mut document)["callable"]["modifiers"] = json!(["variadic"]),
            "definition_variadic" => definition(&mut document)["modifiers"] = json!(["variadic"]),
            "kind" => definition(&mut document)["kind"] = json!("method"),
            "callable" => {
                definition(&mut document)
                    .as_object_mut()
                    .unwrap()
                    .remove("callable");
            }
            "arity" => {
                node(&mut document, "top.larger")["children"] = json!(["top.callee", "top.a"])
            }
            _ => node(&mut document, "top.larger")["kind"] = json!("literal"),
        }
        assert!(!certified(document), "{changed}");
    }
}

#[test]
fn computed_or_adjusted_function_values_do_not_name_the_certified_function() {
    for changed in ["kind", "target", "adjustment"] {
        let mut document = document();
        match changed {
            "kind" => node(&mut document, "top.callee")["kind"] = json!("literal"),
            "target" => reference(&mut document, "top.callee")["target"]["id"] = json!("def:top"),
            _ => {
                node(&mut document, "top.callee")["adjustments"] =
                    json!([{"kind":"reify_fn_pointer","target":"type:fn_larger"}])
            }
        }
        assert!(!certified(document), "{changed}");
    }
}

#[test]
fn instantiated_signatures_must_match_every_adjusted_operand_and_the_result() {
    let mut correct = document();
    reference(&mut correct, "top.larger")["signature"] = json!({
        "parameters":["type:u32","type:u32"],"result":"type:u32"
    });
    assert!(certified(correct.clone()));
    for signature in [
        json!({"parameters":["type:u32"],"result":"type:u32"}),
        json!({"parameters":["type:u32",null],"result":"type:u32"}),
        json!({"parameters":["type:u32","type:bool"],"result":"type:u32"}),
        json!({"parameters":["type:u32","type:u32"],"result":"type:bool"}),
        json!({"parameters":["type:u32","type:u32"]}),
    ] {
        let mut document = correct.clone();
        reference(&mut document, "top.larger")["signature"] = signature;
        assert!(!certified(document));
    }
}

#[test]
fn adjusted_types_need_an_existing_final_target_before_they_can_supply_evidence() {
    let mut adjusted = document();
    node(&mut adjusted, "top.b")["adjustments"] = json!([{
        "kind":"auto_ref","mode":"shared","target":"type:u32_ref"
    }]);
    assert!(certified(adjusted));
    for changed in ["target", "missing", "type"] {
        let mut document = document();
        match changed {
            "target" => {
                node(&mut document, "top.b")["adjustments"] =
                    json!([{"kind":"auto_ref","mode":"shared"}])
            }
            "missing" => {
                node(&mut document, "top.b")["adjustments"] =
                    json!([{"kind":"auto_ref","mode":"shared","target":"type:missing"}])
            }
            _ => {
                node(&mut document, "top.b")
                    .as_object_mut()
                    .unwrap()
                    .remove("type");
            }
        }
        assert!(!certified(document), "{changed}");
    }
}

#[test]
fn borrowed_values_do_not_run_the_referents_destructor_but_owned_values_do() {
    let mut document = document();
    drop_type(&mut document);
    ty(
        &mut document,
        "test:shared",
        "reference",
        json!({"mode":"shared","target":"test:drop"}),
    );
    ty(
        &mut document,
        "test:exclusive",
        "reference",
        json!({"mode":"exclusive","target":"test:drop"}),
    );
    ty(
        &mut document,
        "test:owned",
        "pointer",
        json!({"mode":"owned","target":"test:drop"}),
    );
    assert!(cleanup(document.clone(), "test:shared"));
    assert!(cleanup(document.clone(), "test:exclusive"));
    assert!(!cleanup(document.clone(), "test:drop"));
    assert!(!cleanup(document.clone(), "test:owned"));
    node(&mut document, "top.a")["type"] = json!("test:shared");
    assert!(certified(document.clone()));
    node(&mut document, "top.a")["type"] = json!("test:drop");
    assert!(!certified(document));
}

#[test]
fn missing_targets_and_incomplete_or_unknown_cleanup_shapes_reject_certification() {
    for (kind, fields) in [
        ("reference", json!({"mode":"shared"})),
        (
            "reference",
            json!({"mode":"shared","target":"type:missing"}),
        ),
        ("reference", json!({"target":"type:u32"})),
        ("record", json!({})),
        ("tuple", json!({})),
        ("sequence", json!({})),
        ("opaque", json!({})),
    ] {
        let mut document = document();
        ty(&mut document, "test:incomplete", kind, fields);
        assert!(!cleanup(document, "test:incomplete"), "{kind}");
    }
    let mut document = document();
    nominal(&mut document, "test:record", "record", json!({}));
    assert!(!cleanup(document, "test:record"));
}

#[test]
fn tuple_and_sequence_cleanup_checks_every_owned_component() {
    let mut document = document();
    drop_type(&mut document);
    nominal(
        &mut document,
        "test:record",
        "record",
        json!({"fields":[{"name":"value","type":"type:u32"}]}),
    );
    for (id, kind, fields) in [
        (
            "test:tuple",
            "tuple",
            json!({"items":["type:u32","type:bool"]}),
        ),
        (
            "test:bad_tuple",
            "tuple",
            json!({"items":["type:u32","test:drop"]}),
        ),
        ("test:sequence", "sequence", json!({"element":"type:u32"})),
        (
            "test:bad_sequence",
            "sequence",
            json!({"element":"test:drop"}),
        ),
        (
            "test:missing_member",
            "tuple",
            json!({"items":["type:missing"]}),
        ),
    ] {
        ty(&mut document, id, kind, fields);
    }
    assert!(cleanup(document.clone(), "test:tuple"));
    assert!(cleanup(document.clone(), "test:sequence"));
    assert!(cleanup(document.clone(), "test:record"));
    assert!(!cleanup(document.clone(), "test:bad_tuple"));
    assert!(!cleanup(document.clone(), "test:bad_sequence"));
    assert!(!cleanup(document, "test:missing_member"));
}

#[test]
fn sum_cleanup_checks_each_variant_and_requires_complete_variant_fields() {
    let mut document = document();
    drop_type(&mut document);
    for (id, variants) in [
        (
            "test:sum",
            json!([{"name":"None","shape":"unit"},{"name":"Some","shape":"tuple","fields":[{"name":"0","type":"type:u32"}]}]),
        ),
        (
            "test:bad_sum",
            json!([{"name":"Some","shape":"tuple","fields":[{"name":"0","type":"test:drop"}]}]),
        ),
        (
            "test:incomplete_sum",
            json!([{"name":"Some","shape":"tuple"}]),
        ),
    ] {
        nominal(&mut document, id, "sum", json!({"variants":variants}));
    }
    assert!(cleanup(document.clone(), "test:sum"));
    assert!(!cleanup(document.clone(), "test:bad_sum"));
    assert!(!cleanup(document, "test:incomplete_sum"));
}

#[test]
fn recursive_owned_shapes_and_exhausted_cleanup_budgets_fail_closed() {
    let mut document = document();
    ty(
        &mut document,
        "test:cycle",
        "sequence",
        json!({"element":"test:cycle"}),
    );
    ty(
        &mut document,
        "test:wide",
        "tuple",
        json!({"items":vec!["type:u32";128]}),
    );
    let variants: Vec<_> = (0..128)
        .map(|index| json!({"name":format!("V{index}"),"shape":"unit"}))
        .collect();
    nominal(
        &mut document,
        "test:wide_sum",
        "sum",
        json!({"variants":variants}),
    );
    assert!(!cleanup(document.clone(), "test:cycle"));
    assert!(!cleanup(document.clone(), "test:wide"));
    assert!(!cleanup(document, "test:wide_sum"));
}

#[test]
fn exact_type_guards_obey_reference_depth_and_target_completeness() {
    for fields in [
        json!({"target":"type:u32"}),
        json!({"mode":"shared"}),
        json!({"mode":"shared","target":"type:missing"}),
    ] {
        let mut document = document();
        ty(&mut document, "test:incomplete_ref", "reference", fields);
        node(&mut document, "top.b")["type"] = json!("test:incomplete_ref");
        assert!(!certified(document));
    }
    for (depth, expected) in [(31, true), (32, false)] {
        let mut document = document();
        let mut target = "type:u32".to_owned();
        for index in 0..depth {
            let id = format!("test:ref{index}");
            ty(
                &mut document,
                &id,
                "reference",
                json!({"mode":"shared","target":target}),
            );
            target = id;
        }
        node(&mut document, "top.b")["type"] = json!(target);
        assert_eq!(certified(document), expected, "depth {depth}");
    }
    let mut cyclic = document();
    ty(
        &mut cyclic,
        "test:cycle_ref",
        "reference",
        json!({"mode":"shared","target":"test:cycle_ref"}),
    );
    node(&mut cyclic, "top.b")["type"] = json!("test:cycle_ref");
    assert!(!certified(cyclic));
    let mut wrong_key = document();
    node(&mut wrong_key, "top.b")["type"] = json!("type:bool");
    assert!(!certified(wrong_key));
}
