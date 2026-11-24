//! Callback-free ordinary operations retain evaluation, cleanup guards, and library costs.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::compose::{Analyzed, Composed, Composer};
use rstyle_core::runtime_bounds::{
    self, Assessment, Bound, Callee, Limits, Outcome, Program, Root,
};
use rstyle_core::wording::Wording;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const CLASSED: &str = include_str!("../src/cpg/testdata/callback-free-classed-production.cpg.json");
const CLASSED_PROVENANCE: &str =
    include_str!("../src/cpg/testdata/callback-free-classed-production.provenance.json");
const AGREED: &str = include_str!("../src/cpg/testdata/callback-free-agreed-production.cpg.json");
const AGREED_PROVENANCE: &str =
    include_str!("../src/cpg/testdata/callback-free-agreed-production.provenance.json");

#[derive(Clone, Copy)]
enum Operation {
    ThenSome,
    Copied,
    Ok,
}

impl Operation {
    fn key(self) -> &'static str {
        match self {
            Self::ThenSome => "rust:bool.then_some",
            Self::Copied => "rust:Option.copied",
            Self::Ok => "rust:Result.ok",
        }
    }

    fn receiver(self) -> &'static str {
        match self {
            Self::ThenSome => "type:bool",
            Self::Copied => "cb:option-reference",
            Self::Ok => "cb:result",
        }
    }

    fn result(self) -> &'static str {
        match self {
            Self::ThenSome => "cb:option-guard-reference",
            Self::Copied | Self::Ok => "cb:option-integer",
        }
    }

    fn parameters(self) -> Value {
        match self {
            Self::ThenSome => json!(["cb:shared-guard"]),
            Self::Copied | Self::Ok => json!([]),
        }
    }

    fn generics(self) -> Value {
        match self {
            Self::ThenSome => json!(["cb:shared-guard"]),
            Self::Copied => json!(["type:u32"]),
            Self::Ok => json!(["type:u32", "cb:shared-guard"]),
        }
    }
}

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

fn option(id: &str, element: &str) -> Value {
    json!({"id":id,"kind":"sum","key":"rust:Option","spelling":id,"definition":"cb:Option",
           "arguments":[element],"variants":[{"name":"None","shape":"unit","fields":[]},
             {"name":"Some","shape":"tuple","fields":[{"name":"0","type":element}]}]})
}

fn support_types(document: &mut Value) {
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"cb:guard","kind":"record","spelling":"Guard","definition":"cb:Guard","fields":[]}),
        json!({"id":"cb:shared-guard","kind":"reference","mode":"shared","spelling":"&Guard","target":"cb:guard"}),
        json!({"id":"cb:integer-reference","kind":"reference","mode":"shared","spelling":"&u32","target":"type:u32"}),
        option("cb:option-reference", "cb:integer-reference"),
        option("cb:option-integer", "type:u32"),
        option("cb:option-guard-reference", "cb:shared-guard"),
        json!({"id":"cb:result","kind":"sum","key":"rust:Result","spelling":"Result<u32, &Guard>",
               "definition":"cb:Result","arguments":["type:u32","cb:shared-guard"],
               "variants":[{"name":"Ok","shape":"tuple","fields":[{"name":"0","type":"type:u32"}]},
                           {"name":"Err","shape":"tuple","fields":[{"name":"0","type":"cb:shared-guard"}]}]}),
        json!({"id":"cb:opaque","kind":"opaque","spelling":"unknown cleanup"}),
    ]);
    document["unit"]["definitions"].as_array_mut().unwrap().extend([
        json!({"id":"cb:Guard","kind":"type","name":"Guard","stable_identity":"callback_tests::Guard",
               "scope":"unit","compilation":"callback-tests","destructor":true}),
        json!({"id":"cb:Option","kind":"type","name":"Option","stable_identity":"core::option::Option",
               "scope":"library","standard":true}),
        json!({"id":"cb:Result","kind":"type","name":"Result","stable_identity":"core::result::Result",
               "scope":"library","standard":true}),
    ]);
}

fn operation_definition(operation: Operation) -> Value {
    json!({"id":"cb:operation","kind":"method","name":operation.key(),
           "stable_identity":format!("std::{}",operation.key()),"scope":"library","standard":true,
           "operation":operation.key(),"callable":{"owner_type":operation.receiver(),
             "receiver":operation.receiver(),"parameters":operation.parameters(),"result":operation.result(),
             "generics":0,"modifiers":[]}})
}

fn check_function(operation: Operation) -> Value {
    let mut function = blank("check");
    function["signature"]["parameters"] = json!([
        {"position":0,"symbol":"receiver","type":operation.receiver()}
    ]);
    function["signature"]["result"] = json!(operation.result());
    function["symbols"] = json!([parameter("receiver", operation.receiver())]);
    function["nodes"] = json!([
        expression("check.root", "block", &["check.call"], operation.result()),
        expression("check.receiver", "identifier", &[], operation.receiver()),
        expression(
            "check.call",
            "call",
            &["check.receiver"],
            operation.result()
        )
    ]);
    let mut call = reference("check.call", "definition", "cb:operation");
    call["dispatch"] = json!("method");
    call["calls_back"] =
        json!({"value":true,"reason":"Conservative generic payload classification"});
    call["instance_arguments"] = operation.generics();
    let mut parameters = vec![json!(operation.receiver())];
    parameters.extend(operation.parameters().as_array().unwrap().iter().cloned());
    call["signature"] = json!({"parameters":parameters,"result":operation.result()});
    function["refs"] = json!([call, reference("check.receiver", "symbol", "receiver")]);
    if matches!(operation, Operation::ThenSome) {
        function["signature"]["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"position":1,"symbol":"payload","type":"cb:shared-guard"}));
        function["symbols"]
            .as_array_mut()
            .unwrap()
            .push(parameter("payload", "cb:shared-guard"));
        function["nodes"].as_array_mut().unwrap().push(expression(
            "check.payload",
            "identifier",
            &[],
            "cb:shared-guard",
        ));
        function["refs"].as_array_mut().unwrap().push(reference(
            "check.payload",
            "symbol",
            "payload",
        ));
        node(&mut function, "check.call")["children"] = json!(["check.receiver", "check.payload"]);
    }
    function
}

fn document(operation: Operation) -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    document["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("unsupported");
    document["unit"]["functions"] = json!([check_function(operation)]);
    support_types(&mut document);
    document["unit"]["definitions"].as_array_mut().unwrap().extend([
        operation_definition(operation),
        json!({"id":"cb:check-definition","kind":"function","name":"check",
               "stable_identity":"callback_tests::check","scope":"unit","compilation":"callback-tests"}),
    ]);
    document
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

fn assert_uncertified(document: Value) {
    let programs = programs(document);
    let result = composition(&programs, "callback-tests/callback_tests::check");
    assert!(!matches!(result, Composed::Proven(_)), "{result:?}");
}

#[test]
fn typed_borrowed_payloads_certify_the_three_ordinary_operations() {
    for operation in [Operation::ThenSome, Operation::Copied, Operation::Ok] {
        let programs = programs(document(operation));
        let key = "callback-tests/callback_tests::check";
        let assessment = local(&programs[key]);
        assert_eq!(assessment.loop_body_entries, Bound::Const(0));
        assert_eq!(assessment.calls.len(), 1);
        assert!(matches!(assessment.calls[0].target, Callee::Library(_)));
        assert_eq!(
            assessment.calls[0].operation.as_deref(),
            Some(operation.key())
        );
        let result = composition(&programs, key);
        let Composed::Proven(composed) = result else {
            panic!("{result:?}")
        };
        assert_eq!(composed.workspace_invocations, Bound::Const(0));
        if !matches!(operation, Operation::Ok) {
            assert!(!composed.library_steps.is_complete());
        }
    }
}

fn payload_type(document: &mut Value, payload: &str) {
    let function = check(document);
    function["signature"]["parameters"][1]["type"] = json!(payload);
    function["symbols"][1]["type"] = json!(payload);
    node(function, "check.payload")["type"] = json!(payload);
    call_reference(document)["signature"]["parameters"][1] = json!(payload);
    call_reference(document)["instance_arguments"] = json!([payload]);
    definition(document, "cb:operation")["callable"]["parameters"] = json!([payload]);
    let result = ty(document, "cb:option-guard-reference");
    result["arguments"] = json!([payload]);
    result["variants"][1]["fields"][0]["type"] = json!(payload);
}

#[test]
fn owning_or_unknown_payload_cleanup_is_not_assumed_empty() {
    for payload in ["cb:guard", "cb:opaque"] {
        let mut document = document(Operation::ThenSome);
        payload_type(&mut document, payload);
        assert_uncertified(document);
    }
}

#[test]
fn result_ok_cannot_hide_destruction_of_its_error() {
    let mut document = document(Operation::Ok);
    let result = ty(&mut document, "cb:result");
    result["arguments"][1] = json!("cb:guard");
    result["variants"][1]["fields"][0]["type"] = json!("cb:guard");
    call_reference(&mut document)["instance_arguments"][1] = json!("cb:guard");
    assert_uncertified(document);
}

#[test]
fn result_ok_reads_a_result_without_variants_through_its_type_arguments() {
    // As the Rust frontend writes a library enum: its arguments, and no variants.
    let mut document = document(Operation::Ok);
    let result = ty(&mut document, "cb:result").as_object_mut().unwrap();
    result.remove("variants");
    let programs = programs(document.clone());
    let result = composition(&programs, "callback-tests/callback_tests::check");
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.workspace_invocations, Bound::Const(0));
    // Its error is still read, so an owned one with a destructor runs it.
    ty(&mut document, "cb:result")["arguments"][1] = json!("cb:guard");
    call_reference(&mut document)["instance_arguments"][1] = json!("cb:guard");
    assert_uncertified(document);
}

#[test]
fn copied_does_not_need_structural_cleanup_metadata_for_its_copy_element() {
    let mut document = document(Operation::Copied);
    ty(&mut document, "cb:integer-reference")["target"] = json!("cb:opaque");
    ty(&mut document, "cb:integer-reference")["spelling"] = json!("&OpaqueCopy");
    ty(&mut document, "cb:opaque")["spelling"] = json!("OpaqueCopy");
    ty(&mut document, "cb:opaque")["modifiers"] = json!(["rust:copy"]);
    let result = ty(&mut document, "cb:option-integer");
    result["spelling"] = json!("Option<OpaqueCopy>");
    result["arguments"] = json!(["cb:opaque"]);
    result["variants"][1]["fields"][0]["type"] = json!("cb:opaque");
    call_reference(&mut document)["instance_arguments"] = json!(["cb:opaque"]);
    let programs = programs(document);
    let key = "callback-tests/callback_tests::check";
    let assessment = local(&programs[key]);
    assert_eq!(assessment.calls.len(), 1);
    assert!(matches!(assessment.calls[0].target, Callee::Library(_)));
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, Bound::Const(0));
    assert_eq!(composed.workspace_invocations, Bound::Const(0));
    assert!(!composed.library_steps.is_complete());
}

#[test]
fn incomplete_or_mismatched_instantiated_signatures_cannot_certify_a_call() {
    for expected in [Value::Null, json!("type:u32")] {
        let mut document = document(Operation::ThenSome);
        call_reference(&mut document)["signature"]["parameters"][1] = expected;
        assert_uncertified(document);
    }
    for result in [Value::Null, json!("type:bool")] {
        let mut document = document(Operation::ThenSome);
        call_reference(&mut document)["signature"]["result"] = result;
        assert_uncertified(document);
    }
}

fn positive_effect_fact(document: &mut Value, driven: bool) {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"cb:external-effect","kind":"function","name":"external_effect",
            "stable_identity":"external_effect","scope":"library","standard":true,
            "operation":"rust:File.read"
        }));
    let reference = call_reference(document);
    if driven {
        reference["drives"] = json!(["cb:external-effect"]);
    } else {
        reference["calls_back"]["runs"] = json!([{"kind":"definition","id":"cb:external-effect"}]);
    }
}

#[test]
fn positive_named_runs_and_driven_effects_are_not_erased() {
    for driven in [false, true] {
        let mut document = document(Operation::ThenSome);
        positive_effect_fact(&mut document, driven);
        assert_uncertified(document);
    }
}

#[test]
fn dynamic_dependent_and_counterfeit_operations_are_not_certificates() {
    let mut dynamic = document(Operation::ThenSome);
    call_reference(&mut dynamic)["dispatch"] = json!("dynamic");
    assert_uncertified(dynamic);

    let mut dependent = document(Operation::ThenSome);
    call_reference(&mut dependent)["status"] = json!("dependent");
    call_reference(&mut dependent)["via"] = json!("cb:operation");
    assert_uncertified(dependent);

    let mut counterfeit = document(Operation::ThenSome);
    definition(&mut counterfeit, "cb:operation")["standard"] = json!(false);
    assert_uncertified(counterfeit);
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

fn eager_argument(document: &mut Value) {
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
    let receiver = node(function, "check.receiver");
    receiver["kind"] = json!("literal");
    receiver["value"] = json!({"kind":"bool","value":"false"});
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
fn then_some_evaluates_its_argument_even_when_the_receiver_is_false() {
    let mut document = document(Operation::ThenSome);
    eager_argument(&mut document);
    let programs = programs(document);
    let key = "callback-tests/callback_tests::check";
    let compute = local(&programs["callback-tests/callback_tests::compute"]);
    assert_eq!(compute.loop_body_entries, Bound::Const(3));
    let assessment = local(&programs[key]);
    let call = assessment
        .calls
        .iter()
        .find(|call| call.callee == "compute")
        .unwrap();
    assert_eq!(call.count, Bound::Const(1));
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.workspace_invocations, Bound::Const(1));
    assert_eq!(composed.loop_body_entries, Bound::Const(3));
}

fn saved(document: &str, provenance: &str) -> Value {
    let parsed: Value = serde_json::from_str(document).unwrap();
    let provenance: Value = serde_json::from_str(provenance).unwrap();
    let checksum = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
    assert_eq!(
        checksum(document.as_bytes()),
        provenance["fixture_checksum"]
    );
    assert_eq!(
        checksum(&serde_json::to_vec(&parsed["unit"]["functions"][0]).unwrap()),
        provenance["owner_checksum"]
    );
    for source in parsed["unit"]["files"].as_array().unwrap() {
        assert_eq!(
            checksum(source["content"].as_str().unwrap().as_bytes()),
            source["digest"]
        );
    }
    parsed
}

fn retained_then_some(assessment: &Assessment) {
    let call = assessment
        .calls
        .iter()
        .find(|call| call.operation.as_deref() == Some("rust:bool.then_some"))
        .unwrap();
    assert!(matches!(call.target, Callee::Library(_)), "{call:?}");
}

#[test]
fn exact_classed_capture_gains_a_composed_formal_proof_with_the_same_local_bound() {
    let programs = programs(saved(CLASSED, CLASSED_PROVENANCE));
    let (key, program) = programs.iter().next().unwrap();
    let local = local(program);
    retained_then_some(&local);
    assert_eq!(
        local.loop_body_entries,
        Bound::Input("len(operation)#1".into())
    );
    assert_eq!(program.extents.len(), 1);
    assert_eq!(program.extents[0].root, Root::Formal(1));
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, local.loop_body_entries);
    assert_eq!(composed.workspace_invocations, Bound::Const(0));
    assert!(!composed.library_steps.is_complete());
    assert!(
        composed
            .library_factors
            .iter()
            .any(|factor| factor.reason.contains("rust:bool.then_some"))
    );
}

#[test]
fn exact_agreed_capture_gains_composition_without_promoting_its_local_collection_size() {
    let programs = programs(saved(AGREED, AGREED_PROVENANCE));
    let (key, program) = programs.iter().next().unwrap();
    let local = local(program);
    retained_then_some(&local);
    let peers = program
        .extents
        .iter()
        .find(|extent| extent.input == "len(peers)#1")
        .unwrap();
    let others = program
        .extents
        .iter()
        .find(|extent| extent.input == "len(others)#7")
        .unwrap();
    assert_eq!(peers.root, Root::Formal(1));
    assert_eq!(others.root, Root::Local(7));
    let expected = Bound::Add(vec![
        Bound::Input(peers.input.clone()),
        Bound::Input(others.input.clone()),
        Bound::Input(peers.input.clone()),
        Bound::Input(peers.input.clone()),
    ]);
    assert_eq!(local.loop_body_entries, expected);
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, local.loop_body_entries);
    assert_eq!(
        composed
            .loop_body_entries
            .evaluate(&|label| Some(if label == peers.input { 5 } else { 3 })),
        Some(18)
    );
    assert!(!composed.library_steps.is_complete());
    assert!(
        composed
            .library_factors
            .iter()
            .any(|factor| factor.reason.contains("rust:bool.then_some"))
    );
}
