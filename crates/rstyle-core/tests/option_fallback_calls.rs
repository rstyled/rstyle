//! Eager fallback selection preserves evaluation and certifies only discarded cleanup.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::compose::{Analyzed, Composed, Composer};
use rstyle_core::runtime_bounds::{
    self, Assessment, Bound, Callee, Limits, Measure, Outcome, Program,
};
use rstyle_core::wording::Wording;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const KEY: &str = "callback-tests/callback_tests::check";
const LOCATED: &str = include_str!("../src/cpg/testdata/callback-free-located-production.cpg.json");
const LOCATED_PROVENANCE: &str =
    include_str!("../src/cpg/testdata/callback-free-located-production.provenance.json");
fn expression(id: &str, kind: &str, children: &[&str], ty: &str) -> Value {
    json!({"id":id, "kind":kind, "original_kind":kind, "children":children, "type":ty,
           "span":{"file":"file:lib","start":200,"end":205}, "provenance":{"origin":"source"}})
}

fn reference(node: &str, kind: &str, id: &str) -> Value {
    json!({"node":node,"status":"resolved","target":{"kind":kind,"id":id}})
}

fn parameter(id: &str, ty: &str) -> Value {
    json!({"id":id,"name":id,"kind":"parameter","scope":"cb",
           "type":ty,"mutable":false,"declared":{"file":"file:lib","start":164,"end":169}})
}

fn node<'a>(function: &'a mut Value, id: &str) -> &'a mut Value {
    function["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn definition<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == id)
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

fn check(document: &mut Value) -> &mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "check")
        .unwrap()
}

fn call_reference(document: &mut Value) -> &mut Value {
    check(document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "check.call")
        .unwrap()
}

fn blank(name: &str) -> Value {
    let document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    let mut function = document["unit"]["functions"][2].clone();
    function["id"] = json!(format!("cb:{name}"));
    function["definition"] = json!(format!("cb:{name}-definition"));
    function["name"] = json!(name);
    function["root"] = json!(format!("{name}.root"));
    function["signature"]["parameters"] = json!([]);
    function["signature"]["result"] = json!("type:unit");
    for table in ["nodes", "symbols", "refs", "accesses", "places", "values"] {
        function[table] = json!([]);
    }
    function["cfg"] = json!({"entry":"entry","exits":[],"edges":[]});
    function
}

fn programs(document: Value) -> BTreeMap<String, Program> {
    let keys: BTreeMap<_, _> = document["unit"]["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|definition| {
            let identity = definition["stable_identity"].as_str().unwrap().to_owned();
            let compilation = definition["compilation"].as_str().unwrap_or_default();
            (identity.clone(), format!("{compilation}/{identity}"))
        })
        .collect();
    rstyle_core::cpg::run_bounds(vec![document])
        .unwrap()
        .into_iter()
        .map(|(id, input)| {
            let Input::Captured { program } = input else {
                panic!("{id}: {input:?}")
            };
            (keys[&id].clone(), program)
        })
        .collect()
}

fn composition(programs: &BTreeMap<String, Program>, key: &str) -> Composed {
    let mut composer = Composer::new(
        |id: &str| {
            programs.get(id).map(|program| Analyzed {
                program: program.clone(),
                outcome: runtime_bounds::analyze(program),
            })
        },
        Limits::default(),
    );
    composer.composed(key, Wording::of("rust"))
}

fn local(program: &Program) -> Assessment {
    let result = runtime_bounds::analyze(program);
    let Outcome::Proven(assessment) = result else {
        panic!("{result:?}")
    };
    assessment
}

fn compute_nodes() -> Value {
    json!([
        expression(
            "compute.root",
            "block",
            &["compute.let", "compute.while-statement", "compute.payload"],
            "cb:shared-guard"
        ),
        expression(
            "compute.let",
            "local_declaration",
            &["compute.pattern", "compute.zero"],
            "type:unit"
        ),
        expression("compute.pattern", "pattern_binding", &[], "type:u32"),
        expression("compute.zero", "literal", &[], "type:u32"),
        expression("compute.three", "literal", &[], "type:u32"),
        expression("compute.one", "literal", &[], "type:u32"),
        expression(
            "compute.while-statement",
            "expression_statement",
            &["compute.while"],
            "type:unit"
        ),
        expression(
            "compute.while",
            "while",
            &["compute.less", "compute.body"],
            "type:unit"
        ),
        expression(
            "compute.less",
            "operator",
            &["compute.read", "compute.three"],
            "type:bool"
        ),
        expression("compute.read", "identifier", &[], "type:u32"),
        expression("compute.body", "block", &["compute.increment"], "type:unit"),
        expression(
            "compute.increment",
            "compound_assignment",
            &["compute.write", "compute.one"],
            "type:unit"
        ),
        expression("compute.write", "identifier", &[], "type:u32"),
        expression("compute.payload", "identifier", &[], "cb:shared-guard")
    ])
}

fn compute_function() -> Value {
    let mut function = blank("compute");
    function["signature"]["parameters"] =
        json!([{"position":0,"symbol":"payload","type":"cb:shared-guard"}]);
    function["signature"]["result"] = json!("cb:shared-guard");
    function["symbols"] = json!([
        parameter("payload", "cb:shared-guard"),
        {"id":"i","name":"i","kind":"local","scope":"compute.root","type":"type:u32",
         "mutable":true,"node":"compute.pattern"}
    ]);
    function["nodes"] = compute_nodes();
    for (id, value) in [
        ("compute.zero", "0"),
        ("compute.one", "1"),
        ("compute.three", "3"),
    ] {
        node(&mut function, id)["value"] = json!({"kind":"integer","value":value});
    }
    node(&mut function, "compute.less")["operator"] = json!("lt");
    node(&mut function, "compute.increment")["operator"] = json!("add");
    function["refs"] = json!([
        reference("compute.read", "symbol", "i"),
        reference("compute.write", "symbol", "i"),
        reference("compute.payload", "symbol", "payload")
    ]);
    function
}

fn support(document: &mut Value, payload: &str) {
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"cb:guard","kind":"record","spelling":"Guard","definition":"cb:Guard","fields":[]}),
        json!({"id":"cb:shared-guard","kind":"reference","mode":"shared","spelling":"&Guard","target":"cb:guard"}),
        json!({"id":"cb:function","kind":"function","spelling":"fn(u32) -> bool","parameters":["type:u32"],"result":"type:bool"}),
        json!({"id":"cb:opaque","kind":"opaque","spelling":"unknown cleanup"}),
        json!({"id":"cb:option","kind":"sum","key":"rust:Option","spelling":"Option<T>","definition":"cb:Option","arguments":[payload]}),
    ]);
    document["unit"]["definitions"].as_array_mut().unwrap().extend([
        json!({"id":"cb:Guard","kind":"type","name":"Guard","stable_identity":"callback_tests::Guard","scope":"unit","compilation":"callback-tests","destructor":true}),
        json!({"id":"cb:Option","kind":"type","name":"Option","stable_identity":"core::option::Option","scope":"library","standard":true}),
        json!({"id":"cb:check-definition","kind":"function","name":"check","stable_identity":"callback_tests::check","scope":"unit","compilation":"callback-tests"}),
        json!({"id":"cb:operation","kind":"method","name":"Option::unwrap_or","stable_identity":"core::option::unwrap_or",
               "scope":"library","standard":true,"operation":"rust:Option.unwrap_or",
               "callable":{"owner_type":"cb:option","receiver":"cb:option","parameters":[payload],"result":payload,"generics":1,"modifiers":[]}}),
    ]);
}

fn check_function(payload: &str) -> Value {
    let mut function = blank("check");
    function["signature"]["parameters"] = json!([
        {"position":0,"symbol":"receiver","type":"cb:option"},
        {"position":1,"symbol":"payload","type":payload},
    ]);
    function["signature"]["result"] = json!(payload);
    function["symbols"] = json!([
        parameter("receiver", "cb:option"),
        parameter("payload", payload)
    ]);
    function["nodes"] = json!([
        expression("check.root", "block", &["check.call"], payload),
        expression("check.receiver", "identifier", &[], "cb:option"),
        expression("check.payload", "identifier", &[], payload),
        expression(
            "check.call",
            "call",
            &["check.receiver", "check.payload"],
            payload
        ),
    ]);
    let mut call = reference("check.call", "definition", "cb:operation");
    call["dispatch"] = json!("method");
    call["calls_back"] =
        json!({"value":true,"reason":"Conservative generic cleanup classification"});
    call["instance_arguments"] = json!([payload]);
    call["signature"] = json!({"parameters":["cb:option",payload],"result":payload});
    function["refs"] = json!([
        call,
        reference("check.receiver", "symbol", "receiver"),
        reference("check.payload", "symbol", "payload"),
    ]);
    function
}

fn document(payload: &str) -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    document["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("unsupported");
    document["unit"]["functions"] = json!([check_function(payload)]);
    support(&mut document, payload);
    document
}

fn assert_uncertified(document: Value) {
    let programs = programs(document);
    let result = composition(&programs, KEY);
    assert!(!matches!(result, Composed::Proven(_)), "{result:?}");
}

#[test]
fn moved_receiver_needs_no_structural_cleanup_metadata_when_the_fallback_is_inert() {
    for payload in ["cb:shared-guard", "cb:function"] {
        let document = document(payload);
        let receiver = document["unit"]["types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|ty| ty["id"] == "cb:option")
            .unwrap();
        assert!(receiver.get("variants").is_none());
        let programs = programs(document);
        let assessment = local(&programs[KEY]);
        assert_eq!(assessment.loop_body_entries, Bound::Const(0));
        assert_eq!(assessment.calls.len(), 1);
        assert!(matches!(assessment.calls[0].target, Callee::Library(_)));
        assert_eq!(
            assessment.calls[0].operation.as_deref(),
            Some("rust:Option.unwrap_or")
        );
        let result = composition(&programs, KEY);
        let Composed::Proven(composed) = result else {
            panic!("{result:?}")
        };
        assert_eq!(composed.loop_body_entries, Bound::Const(0));
        assert_eq!(composed.workspace_invocations, Bound::Const(0));
        assert_eq!(composed.library_steps, Bound::Const(1));
    }
}

#[test]
fn discarded_fallback_cleanup_cannot_be_assumed_empty() {
    for payload in ["cb:guard", "cb:opaque"] {
        assert_uncertified(document(payload));
    }
}

fn named_execution(document: &mut Value, driven: bool) {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"cb:external","kind":"function","name":"external","stable_identity":"external",
            "scope":"library","standard":true,"operation":"rust:File.read"
        }));
    let reference = call_reference(document);
    if driven {
        reference["drives"] = json!(["cb:external"]);
    } else {
        reference["calls_back"]["runs"] = json!([{"kind":"definition","id":"cb:external"}]);
    }
}

#[test]
fn positive_named_callbacks_and_driven_code_are_never_discarded() {
    for driven in [false, true] {
        let mut document = document("cb:shared-guard");
        named_execution(&mut document, driven);
        assert_uncertified(document);
    }
}

#[test]
fn explicit_incomplete_or_mismatched_signatures_do_not_certify_selection() {
    for payload in [Value::Null, json!("type:u32")] {
        let mut document = document("cb:shared-guard");
        call_reference(&mut document)["signature"]["parameters"][1] = payload;
        assert_uncertified(document);
    }
    for result in [Value::Null, json!("type:bool")] {
        let mut document = document("cb:shared-guard");
        call_reference(&mut document)["signature"]["result"] = result;
        assert_uncertified(document);
    }
}

#[test]
fn dynamic_dependent_and_counterfeit_unwrap_or_calls_stay_unresolved() {
    let mut dynamic = document("cb:shared-guard");
    call_reference(&mut dynamic)["dispatch"] = json!("dynamic");
    assert_uncertified(dynamic);
    let mut dependent = document("cb:shared-guard");
    call_reference(&mut dependent)["status"] = json!("dependent");
    call_reference(&mut dependent)["via"] = json!("cb:operation");
    assert_uncertified(dependent);
    let mut counterfeit = document("cb:shared-guard");
    definition(&mut counterfeit, "cb:operation")["standard"] = json!(false);
    assert_uncertified(counterfeit);
    let mut wrong_receiver = document("cb:shared-guard");
    ty(&mut wrong_receiver, "cb:option")["key"] = json!("rust:Result");
    assert_uncertified(wrong_receiver);
}

fn known_some(document: &mut Value) {
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id":"cb:Some","kind":"variant","name":"Some","stable_identity":"core::option::Option::Some",
        "owner":"cb:Option","scope":"library","standard":true
    }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"cb:some-type","kind":"function","spelling":"Some(&Guard)","definition":"cb:Some",
            "parameters":["cb:shared-guard"],"result":"cb:option"
        }));
    let function = check(document);
    let receiver = node(function, "check.receiver");
    receiver["kind"] = json!("call");
    receiver["children"] = json!(["check.some-callee", "check.some-payload"]);
    function["nodes"].as_array_mut().unwrap().extend([
        expression("check.some-callee", "identifier", &[], "cb:some-type"),
        expression("check.some-payload", "identifier", &[], "cb:shared-guard"),
    ]);
    function["refs"].as_array_mut().unwrap().extend([
        reference("check.some-callee", "constructor", "cb:Some"),
        reference("check.some-payload", "symbol", "payload"),
    ]);
    let reference = function["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == "check.receiver")
        .unwrap();
    reference["target"] = json!({"kind":"constructor","id":"cb:Some"});
    reference["dispatch"] = json!("static");
}

fn eager_fallback(document: &mut Value) {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(compute_function());
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id":"cb:compute-definition","kind":"function","name":"compute",
        "stable_identity":"callback_tests::compute","scope":"unit","compilation":"callback-tests",
        "callable":{"parameters":["cb:shared-guard"],"result":"cb:shared-guard","generics":0,"modifiers":[]}
    }));
    document["unit"]["types"].as_array_mut().unwrap().push(json!({
        "id":"cb:compute-type","kind":"function","spelling":"fn(&Guard) -> &Guard {compute}",
        "definition":"cb:compute-definition","parameters":["cb:shared-guard"],"result":"cb:shared-guard"
    }));
    let function = check(document);
    function["nodes"].as_array_mut().unwrap().extend([
        expression("check.compute-callee", "identifier", &[], "cb:compute-type"),
        expression(
            "check.compute",
            "call",
            &["check.compute-callee", "check.payload"],
            "cb:shared-guard",
        ),
    ]);
    let mut call = reference("check.compute", "definition", "cb:compute-definition");
    call["dispatch"] = json!("static");
    function["refs"].as_array_mut().unwrap().extend([
        call,
        reference(
            "check.compute-callee",
            "definition",
            "cb:compute-definition",
        ),
    ]);
    node(function, "check.call")["children"] = json!(["check.receiver", "check.compute"]);
}

#[test]
fn a_known_some_still_evaluates_the_fallback_before_selecting_its_payload() {
    let mut document = document("cb:shared-guard");
    known_some(&mut document);
    eager_fallback(&mut document);
    let programs = programs(document);
    assert_eq!(
        local(&programs["callback-tests/callback_tests::compute"]).loop_body_entries,
        Bound::Const(3)
    );
    let assessment = local(&programs[KEY]);
    let call = assessment
        .calls
        .iter()
        .find(|call| call.callee == "compute")
        .unwrap();
    assert_eq!(call.count, Bound::Const(1));
    assert!(
        assessment
            .calls
            .iter()
            .any(|call| matches!(call.target, Callee::Constructor))
    );
    let result = composition(&programs, KEY);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.workspace_invocations, Bound::Const(1));
    assert_eq!(composed.loop_body_entries, Bound::Const(3));
    assert_eq!(composed.library_steps, Bound::Const(1));
}

fn saved() -> Value {
    let document: Value = serde_json::from_str(LOCATED).unwrap();
    let provenance: Value = serde_json::from_str(LOCATED_PROVENANCE).unwrap();
    let checksum = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
    assert_eq!(checksum(LOCATED.as_bytes()), provenance["fixture_checksum"]);
    for retained in provenance["retained_functions"].as_array().unwrap() {
        let function = document["unit"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["id"] == retained["function"])
            .unwrap();
        assert_eq!(
            checksum(&serde_json::to_vec(function).unwrap()),
            retained["checksum"]
        );
    }
    for file in document["unit"]["files"].as_array().unwrap() {
        assert_eq!(
            checksum(file["content"].as_str().unwrap().as_bytes()),
            file["digest"]
        );
    }
    document
}

#[test]
fn exact_located_capture_gains_its_helpers_formal_field_bound() {
    let programs = programs(saved());
    let (key, program) = programs
        .iter()
        .find(|(key, _)| key.ends_with("/rstyle_core::cpg::records::located"))
        .unwrap();
    let assessment = local(program);
    assert_eq!(assessment.loop_body_entries, Bound::Const(0));
    assert_eq!(program.formals.len(), 1);
    assert_eq!(program.formals[0].measure, Measure::Shared);
    assert!(program.extents.is_empty());
    let selection = assessment
        .calls
        .iter()
        .find(|call| call.operation.as_deref() == Some("rust:Option.unwrap_or"))
        .unwrap();
    assert!(matches!(selection.target, Callee::Library(_)));
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(
        composed.loop_body_entries,
        Bound::Input("len(function.nodes)".into())
    );
    assert_eq!(composed.workspace_invocations, Bound::Const(2));
    assert!(!composed.library_steps.is_complete());
    assert!(
        composed
            .library_factors
            .iter()
            .all(|factor| !factor.reason.contains("Option.unwrap_or"))
    );
}
