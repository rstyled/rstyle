use super::super::Invocation;
use crate::cpg::model::Document;
use crate::cpg::tree::{Body, Tables};
use crate::runtime_bounds::contracts::Contracts;
use serde_json::{Value, json};

const CALL: &str = "fc9f19b77.72";
const OPERAND: &str = "fc9f19b77.74";
const DECLARED: &str = "def:core::iter::traits::iterator::Iterator::next";
const SELECTED: &str = "def:core::str::iter::{impl#64}::next";
const TRAIT: &str = "def:core::iter::traits::iterator::Iterator";
const NOMINAL: &str = "def:core::str::iter::RSplit";
const ACTUAL: &str = "type:std::str::RSplit<'_, &str>";
const BORROWED: &str = "type:&mut std::str::RSplit<'_, &str>";
const OWNER: &str = "type:std::str::RSplit<'a, P>#P=1";
const OWNER_BORROW: &str = "type:&mut std::str::RSplit<'a, P>#P=1";

fn document() -> Value {
    serde_json::from_str(include_str!(
        "../../testdata/iterator-state-short-paths-production.cpg.json"
    ))
    .unwrap()
}

fn function(document: &mut Value) -> &mut Value {
    &mut document["unit"]["functions"][0]
}

fn item<'a>(rows: &'a mut Value, key: &str, id: &str) -> &'a mut Value {
    rows.as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row[key] == id)
        .unwrap()
}

fn reference(document: &mut Value) -> &mut Value {
    item(&mut function(document)["refs"], "node", CALL)
}

fn node<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    item(&mut function(document)["nodes"], "id", id)
}

fn definition<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    item(&mut document["unit"]["definitions"], "id", id)
}

fn ty<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    item(&mut document["unit"]["types"], "id", id)
}

fn with_invocation<R>(
    document: Value,
    callee: &str,
    check: impl FnOnce(&Invocation<'_, '_>) -> R,
) -> R {
    let document: Document = serde_json::from_value(document).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let function = &unit.functions.as_ref().unwrap()[0];
    let body = Body::new(function).unwrap();
    let tables = Tables::new(unit).unwrap();
    check(&Invocation {
        body: &body,
        tables: &tables,
        call: body.node(CALL).unwrap(),
        callee: tables.definition(callee).unwrap(),
        contracts: Contracts::embedded(),
    })
}

fn certified(document: Value) -> bool {
    with_invocation(document, SELECTED, |invocation| {
        invocation.state_certified()
    })
}

fn pattern(document: &mut Value, kind: &str, fields: Value) {
    let mut entry = json!({"id":"state:pattern","kind":kind,"spelling":"pattern"});
    entry
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(entry);
    ty(document, ACTUAL)["arguments"] = json!(["state:pattern"]);
}

#[test]
fn production_state_certifies_for_either_reader_without_an_inherent_role() {
    for callee in [DECLARED, SELECTED] {
        with_invocation(document(), callee, |invocation| {
            assert!(invocation.state_certified());
            assert!(!invocation.certified());
            assert!(crate::cpg::invocation::callback_free(
                invocation.body,
                invocation.tables,
                invocation.call,
                invocation.callee,
            ));
        });
    }
    let mut uncaptured = document();
    reference(&mut uncaptured)
        .as_object_mut()
        .unwrap()
        .remove("calls_back");
    assert!(certified(uncaptured));
}

#[test]
fn a_state_certificate_requires_a_call_with_a_resolved_method_reference() {
    let mut missing = document();
    function(&mut missing)["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != CALL);
    assert!(!certified(missing));
    let mut identifier = document();
    node(&mut identifier, CALL)["kind"] = json!("identifier");
    assert!(!certified(identifier));
    for status in ["dependent", "unresolved"] {
        let mut changed = document();
        reference(&mut changed)["status"] = json!(status);
        assert!(!certified(changed), "{status}");
    }
    for dispatch in [
        json!(null),
        json!("static"),
        json!("builtin"),
        json!("dynamic"),
        json!("overloaded"),
        json!("function_value"),
    ] {
        let mut changed = document();
        reference(&mut changed)["dispatch"] = dispatch;
        assert!(!certified(changed));
    }
    let mut explicit_arguments = document();
    reference(&mut explicit_arguments)["arguments"] = json!([]);
    assert!(!certified(explicit_arguments));
}

#[test]
fn enumerated_execution_is_preserved_even_when_its_roster_is_empty() {
    for runs in [json!([]), json!([{"kind":"definition","id":DECLARED}])] {
        for value in [false, true] {
            let mut changed = document();
            reference(&mut changed)["calls_back"] = json!({
                "value":value,"reason":"enumerated execution","runs":runs
            });
            assert!(!certified(changed));
        }
    }
    for drives in [json!([]), json!([DECLARED])] {
        let mut changed = document();
        reference(&mut changed)["drives"] = drives;
        assert!(!certified(changed));
    }
}

#[test]
fn declared_and_selected_targets_need_definition_ids_and_the_exact_link() {
    for target in ["target", "selected"] {
        for value in [
            json!(null),
            json!({"kind":"symbol","id":DECLARED}),
            json!({"kind":"definition"}),
            json!({"kind":"definition","id":"state:missing"}),
        ] {
            let mut changed = document();
            reference(&mut changed)[target] = value;
            assert!(!certified(changed), "{target}");
        }
    }
    for implements in [json!(null), json!("state:missing"), json!(SELECTED)] {
        let mut changed = document();
        definition(&mut changed, SELECTED)["implements"] = implements;
        assert!(!certified(changed));
    }
    with_invocation(
        document(),
        "def:alloc::string::{impl#0}::clear",
        |invocation| {
            assert!(!invocation.state_certified());
        },
    );
}

#[test]
fn both_methods_must_be_standard_fixed_parameterless_members() {
    for id in [DECLARED, SELECTED] {
        for (pointer, value) in [
            ("/standard", json!(false)),
            ("/scope", json!("unit")),
            ("/scope", json!("workspace")),
            ("/kind", json!("function")),
            ("/callable", json!(null)),
            ("/callable/receiver", json!(null)),
            ("/callable/parameters", json!(["type:str"])),
            ("/callable/generics", json!(1)),
            ("/modifiers", json!(["variadic"])),
            ("/callable/modifiers", json!(["variadic"])),
        ] {
            let mut changed = document();
            *definition(&mut changed, id).pointer_mut(pointer).unwrap() = value;
            assert!(!certified(changed), "{id}{pointer}");
        }
    }
}

#[test]
fn the_declared_member_requires_its_standard_iteration_trait_and_contract_key() {
    for (field, value) in [
        ("owner", json!(null)),
        ("owner", json!("state:missing")),
        ("iteration", json!(false)),
        ("implements", json!(SELECTED)),
        ("operation", json!(null)),
        ("operation", json!("rust:Iterator.unregistered")),
    ] {
        let mut changed = document();
        definition(&mut changed, DECLARED)[field] = value;
        assert!(!certified(changed), "{field}");
    }
    for (field, value) in [
        ("kind", json!("type")),
        ("standard", json!(false)),
        ("scope", json!("unit")),
    ] {
        let mut changed = document();
        definition(&mut changed, TRAIT)[field] = value;
        assert!(!certified(changed), "{field}");
    }
}

#[test]
fn exactly_one_existing_runtime_receiver_is_required() {
    let mut empty = document();
    node(&mut empty, CALL)["children"] = json!([]);
    assert!(!certified(empty));
    let mut extra = document();
    let mut literal = node(&mut extra, "fc9f19b77.78").clone();
    literal["id"] = json!("state:extra");
    function(&mut extra)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(literal);
    node(&mut extra, CALL)["children"] = json!([OPERAND, "state:extra"]);
    assert!(!certified(extra));
    with_invocation(document(), SELECTED, |invocation| {
        let mut call = invocation.call.clone();
        call.children = vec!["state:missing".into()];
        assert!(
            !Invocation {
                call: &call,
                ..*invocation
            }
            .state_certified()
        );
    });
}

#[test]
fn instantiated_signatures_cannot_be_missing_partial_or_inconsistent() {
    for signature in [
        json!(null),
        json!({"parameters":[],"result":"type:std::option::Option<&str>"}),
        json!({"parameters":[null],"result":"type:std::option::Option<&str>"}),
        json!({"parameters":[BORROWED,BORROWED],"result":"type:std::option::Option<&str>"}),
        json!({"parameters":["type:&str"],"result":"type:std::option::Option<&str>"}),
        json!({"parameters":[BORROWED],"result":null}),
        json!({"parameters":[BORROWED],"result":"type:bool"}),
    ] {
        let mut changed = document();
        reference(&mut changed)["signature"] = signature;
        assert!(!certified(changed));
    }
    let mut missing_result = document();
    node(&mut missing_result, CALL)["type"] = json!(null);
    assert!(!certified(missing_result));
    let mut unresolved_result = document();
    node(&mut unresolved_result, CALL)["type"] = json!("state:missing");
    reference(&mut unresolved_result)["signature"]["result"] = json!("state:missing");
    assert!(!certified(unresolved_result));
    for arguments in [
        json!(null),
        json!([]),
        json!([ACTUAL, ACTUAL]),
        json!([OWNER]),
    ] {
        let mut changed = document();
        reference(&mut changed)["instance_arguments"] = arguments;
        assert!(!certified(changed));
    }
}

#[test]
fn the_adjusted_operand_must_resolve_to_an_exclusive_reference() {
    for target in [json!(null), json!("state:missing")] {
        let mut changed = document();
        node(&mut changed, OPERAND)["adjustments"][0]["target"] = target;
        assert!(!certified(changed));
    }
    let mut no_adjustment = document();
    node(&mut no_adjustment, OPERAND)["adjustments"] = json!([]);
    assert!(!certified(no_adjustment));
    for (field, value) in [
        ("kind", json!("pointer")),
        ("mode", json!("shared")),
        ("mode", json!(null)),
        ("target", json!(null)),
        ("target", json!("state:missing")),
        ("target", json!("type:str")),
    ] {
        let mut changed = document();
        ty(&mut changed, BORROWED)[field] = value;
        assert!(!certified(changed), "{field}");
    }
}

#[test]
fn selected_receiver_and_owner_type_must_resolve_and_agree() {
    for (field, value) in [
        ("owner_type", json!(null)),
        ("owner_type", json!("state:missing")),
        ("owner_type", json!("type:str")),
        ("receiver", json!("state:missing")),
    ] {
        let mut changed = document();
        definition(&mut changed, SELECTED)["callable"][field] = value;
        assert!(!certified(changed), "{field}");
    }
    for (field, value) in [
        ("kind", json!("pointer")),
        ("mode", json!("shared")),
        ("target", json!(null)),
        ("target", json!("state:missing")),
        ("target", json!(ACTUAL)),
    ] {
        let mut changed = document();
        ty(&mut changed, OWNER_BORROW)[field] = value;
        assert!(!certified(changed), "{field}");
    }
}

#[test]
fn both_state_types_require_the_same_known_nominal_record_definition() {
    for id in [ACTUAL, OWNER] {
        for (field, value) in [
            ("kind", json!("sum")),
            ("kind", json!("opaque")),
            ("definition", json!(null)),
            ("definition", json!("state:missing")),
            ("definition", json!("def:alloc::string::String")),
        ] {
            let mut changed = document();
            ty(&mut changed, id)[field] = value;
            assert!(!certified(changed), "{id}/{field}");
        }
    }
    let mut missing_nominal = document();
    missing_nominal["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .retain(|definition| definition["id"] != NOMINAL);
    assert!(!certified(missing_nominal));
}

#[test]
fn nominal_identity_requires_a_standard_library_type_without_a_destructor() {
    for (field, value) in [
        ("standard", json!(false)),
        ("scope", json!("unit")),
        ("kind", json!("trait")),
        ("destructor", json!(true)),
        ("stable_identity", json!("other::RSplit")),
    ] {
        let mut changed = document();
        definition(&mut changed, NOMINAL)[field] = value;
        assert!(!certified(changed), "{field}");
    }
}

#[test]
fn generic_rosters_must_be_complete_and_each_type_must_resolve() {
    for id in [ACTUAL, OWNER] {
        for arguments in [
            json!(null),
            json!([]),
            json!(["type:&str", "type:&str"]),
            json!(["state:missing"]),
        ] {
            let mut changed = document();
            ty(&mut changed, id)["arguments"] = arguments;
            assert!(!certified(changed), "{id}");
        }
    }
    let mut concrete = document();
    ty(&mut concrete, OWNER)["arguments"] = json!(["type:&str"]);
    assert!(certified(concrete));
    let mut different_concrete = document();
    ty(&mut different_concrete, OWNER)["arguments"] = json!(["type:char"]);
    assert!(!certified(different_concrete));
}

#[test]
fn patterns_must_be_direct_characters_or_exact_shared_primitive_strings() {
    let mut character = document();
    ty(&mut character, ACTUAL)["arguments"] = json!(["type:char"]);
    assert!(certified(character.clone()));
    ty(&mut character, OWNER)["arguments"] = json!(["type:char"]);
    assert!(certified(character));
    for (field, value) in [
        ("kind", json!("pointer")),
        ("mode", json!("exclusive")),
        ("mode", json!(null)),
        ("target", json!(null)),
        ("target", json!("state:missing")),
        ("target", json!("type:char")),
    ] {
        let mut changed = document();
        ty(&mut changed, "type:&str")[field] = value;
        assert!(!certified(changed), "{field}");
    }
    let mut generic = document();
    ty(&mut generic, ACTUAL)["arguments"] = json!(["type:P#P=1"]);
    assert!(!certified(generic));
    let mut nested_reference = document();
    pattern(
        &mut nested_reference,
        "reference",
        json!({"mode":"shared","target":"type:&str"}),
    );
    assert!(!certified(nested_reference));
}

#[test]
fn primitive_keys_cannot_certify_callable_opaque_or_nominal_payloads() {
    for kind in [
        "function",
        "closure",
        "record",
        "sum",
        "opaque",
        "parameter",
    ] {
        let mut changed = document();
        pattern(&mut changed, kind, json!({"key":"rust:char"}));
        assert!(!certified(changed), "{kind}");
    }
    for primitive in ["type:str", "type:char"] {
        for (field, value) in [
            ("key", json!(null)),
            ("key", json!("other:primitive")),
            ("definition", json!(NOMINAL)),
            ("arguments", json!(["type:bool"])),
        ] {
            let mut changed = document();
            if primitive == "type:char" {
                ty(&mut changed, ACTUAL)["arguments"] = json!([primitive]);
            }
            ty(&mut changed, primitive)[field] = value;
            assert!(!certified(changed), "{primitive}/{field}");
        }
    }
    let mut empty_payload = document();
    ty(&mut empty_payload, "type:str")["arguments"] = json!([]);
    assert!(certified(empty_payload));
}
