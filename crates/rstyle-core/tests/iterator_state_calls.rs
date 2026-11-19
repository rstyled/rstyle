//! Nominal iterator certificates preserve evaluation, cleanup, and opaque library work.

use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_core::runtime_bounds::compose::{Analyzed, Composed, Composer};
use rstyle_core::runtime_bounds::contracts::Contracts;
use rstyle_core::runtime_bounds::{
    self, Assessment, Bound, Callee, Limits, Outcome, Program, Root,
};
use rstyle_core::wording::Wording;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const KEY: &str = "iterator-tests/iterator_tests::check";
const PATHS: &str =
    include_str!("../src/cpg/testdata/iterator-state-short-paths-production.cpg.json");
const PATHS_PROVENANCE: &str =
    include_str!("../src/cpg/testdata/iterator-state-short-paths-production.provenance.json");
const VARIANT: &str =
    include_str!("../src/cpg/testdata/iterator-state-unit-variant-production.cpg.json");
const VARIANT_PROVENANCE: &str =
    include_str!("../src/cpg/testdata/iterator-state-unit-variant-production.provenance.json");

#[derive(Clone, Copy)]
enum Pattern {
    String,
    Character,
}

impl Pattern {
    fn ty(self) -> &'static str {
        match self {
            Self::String => "state:shared-str",
            Self::Character => "state:char",
        }
    }

    fn literal(self) -> Value {
        match self {
            Self::String => json!({"kind":"string","value":"::"}),
            Self::Character => json!({"kind":"char","value":":"}),
        }
    }
}

fn expression(id: &str, kind: &str, children: &[&str], ty: &str) -> Value {
    json!({"id":id,"kind":kind,"original_kind":kind,"children":children,"type":ty,
           "span":{"file":"file:lib","start":200,"end":205},"provenance":{"origin":"source"}})
}

fn reference(node: &str, kind: &str, id: &str) -> Value {
    json!({"node":node,"status":"resolved","target":{"kind":kind,"id":id}})
}

fn check(document: &mut Value) -> &mut Value {
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|function| function["name"] == "check")
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

fn call_reference<'a>(document: &'a mut Value, id: &str) -> &'a mut Value {
    check(document)["refs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|reference| reference["node"] == id)
        .unwrap()
}

fn parameter(id: &str, ty: &str) -> Value {
    json!({"id":id,"name":id,"kind":"parameter","scope":"state","type":ty,
           "mutable":false,"declared":{"file":"file:lib","start":164,"end":169}})
}

fn blank(name: &str) -> Value {
    let document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    let mut function = document["unit"]["functions"][2].clone();
    function["id"] = json!(format!("state:{name}"));
    function["definition"] = json!(format!("state:{name}-definition"));
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

fn nominal(id: &str, argument: &str) -> Value {
    json!({"id":id,"kind":"record","spelling":id,"definition":"state:RSplit",
           "arguments":[argument],"modifiers":["rust:lifetime-arguments"]})
}

fn option(id: &str, element: &str) -> Value {
    json!({"id":id,"kind":"sum","key":"rust:Option","spelling":id,
           "definition":"state:Option","arguments":[element],
           "variants":[{"name":"None","shape":"unit","fields":[]},
             {"name":"Some","shape":"tuple","fields":[{"name":"0","type":element}]}]})
}

fn support_types(document: &mut Value, pattern: Pattern) {
    document["unit"]["types"].as_array_mut().unwrap().extend([
        json!({"id":"state:str","kind":"string","key":"rust:str","spelling":"str"}),
        json!({"id":"state:shared-str","kind":"reference","mode":"shared","target":"state:str","spelling":"&str"}),
        json!({"id":"state:char","kind":"char","key":"rust:char","spelling":"char"}),
        json!({"id":"state:pattern","kind":"parameter","index":0,"spelling":"P"}),
        nominal("state:iterator", pattern.ty()),
        nominal("state:prototype", "state:pattern"),
        json!({"id":"state:borrowed-iterator","kind":"reference","mode":"exclusive","target":"state:iterator","spelling":"&mut RSplit<P>"}),
        json!({"id":"state:borrowed-prototype","kind":"reference","mode":"exclusive","target":"state:prototype","spelling":"&mut RSplit<P>"}),
        json!({"id":"state:opaque","kind":"opaque","spelling":"unknown pattern"}),
        json!({"id":"state:predicate","kind":"function","spelling":"fn(char) -> bool","parameters":["state:char"],"result":"type:bool"}),
        json!({"id":"state:closure","kind":"closure","spelling":"predicate closure","parameters":["state:char"],"result":"type:bool"}),
        json!({"id":"state:guard","kind":"record","spelling":"Guard","definition":"state:Guard","fields":[]}),
        option("state:option-str", "state:shared-str"),
        option("state:option-guard", "state:guard"),
    ]);
}

fn support_definitions(document: &mut Value) {
    document["unit"]["definitions"].as_array_mut().unwrap().extend([
        json!({"id":"state:RSplit","kind":"type","name":"RSplit","stable_identity":"core::str::iter::RSplit","scope":"library","standard":true}),
        json!({"id":"state:Option","kind":"type","name":"Option","stable_identity":"core::option::Option","scope":"library","standard":true}),
        json!({"id":"state:Guard","kind":"type","name":"Guard","stable_identity":"iterator_tests::Guard","scope":"unit","compilation":"iterator-tests","destructor":true}),
        json!({"id":"state:Iterator","kind":"trait","name":"Iterator","stable_identity":"core::iter::traits::iterator::Iterator","scope":"library","standard":true}),
        json!({"id":"state:next","kind":"method","name":"Iterator::next","stable_identity":"core::iter::traits::iterator::Iterator::next",
               "scope":"library","standard":true,"operation":"rust:Iterator.next","owner":"state:Iterator","iteration":true,
               "callable":{"receiver":"state:borrowed-prototype","parameters":[],"result":"state:option-str","generics":0,"modifiers":[]}}),
        json!({"id":"state:selected","kind":"method","name":"RSplit::next","stable_identity":"standard::rsplit_next",
               "scope":"library","standard":true,"implements":"state:next","operation":"rust:standard::rsplit_next",
               "callable":{"owner_type":"state:prototype","receiver":"state:borrowed-prototype","parameters":[],"result":"state:option-str","generics":0,"modifiers":[]}}),
        json!({"id":"state:rsplit","kind":"method","name":"str::rsplit","stable_identity":"core::str::rsplit",
               "scope":"library","standard":true,"operation":"rust:str.rsplit",
               "callable":{"owner_type":"state:str","receiver":"state:shared-str","parameters":["state:pattern"],"result":"state:prototype","generics":1,"modifiers":[]}}),
        json!({"id":"state:check-definition","kind":"function","name":"check","stable_identity":"iterator_tests::check","scope":"unit","compilation":"iterator-tests"}),
    ]);
}

fn state_function() -> Value {
    let mut function = blank("check");
    function["signature"]["parameters"] =
        json!([{"position":0,"symbol":"iterator","type":"state:iterator"}]);
    function["signature"]["result"] = json!("state:option-str");
    function["symbols"] = json!([parameter("iterator", "state:iterator")]);
    function["symbols"][0]["mutable"] = json!(true);
    function["nodes"] = json!([
        expression("check.root", "block", &["check.call"], "state:option-str"),
        expression("check.receiver", "identifier", &[], "state:iterator"),
        expression(
            "check.call",
            "call",
            &["check.receiver"],
            "state:option-str"
        ),
    ]);
    node(&mut function, "check.receiver")["adjustments"] =
        json!([{"kind":"auto_ref","mode":"exclusive","target":"state:borrowed-iterator"}]);
    let mut call = reference("check.call", "definition", "state:next");
    call["selected"] = json!({"kind":"definition","id":"state:selected"});
    call["dispatch"] = json!("method");
    call["instance_arguments"] = json!(["state:iterator"]);
    call["item"] = json!("state:shared-str");
    call["signature"] =
        json!({"parameters":["state:borrowed-iterator"],"result":"state:option-str"});
    call["calls_back"] =
        json!({"value":true,"reason":"Generic iterator conservatively may invoke workspace code"});
    function["refs"] = json!([call, reference("check.receiver", "symbol", "iterator")]);
    function
}

fn document(pattern: Pattern) -> Value {
    let mut document: Value =
        serde_json::from_str(include_str!("../src/cpg/testdata/example.cpg.json")).unwrap();
    document["coverage"]["capabilities"]["implicit-calls"]["status"] = json!("unsupported");
    document["unit"]["functions"] = json!([state_function()]);
    support_types(&mut document, pattern);
    support_definitions(&mut document);
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

fn assert_certified(document: Value, calls: usize) {
    let programs = programs(document);
    let assessment = local(&programs[KEY]);
    assert_eq!(assessment.loop_body_entries, Bound::Const(0));
    assert_eq!(assessment.calls.len(), calls);
    assert!(
        assessment
            .calls
            .iter()
            .all(|call| matches!(call.target, Callee::Library(_)))
    );
    assert!(programs[KEY].extents.is_empty());
    let result = composition(&programs, KEY);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, Bound::Const(0));
    assert_eq!(composed.workspace_invocations, Bound::Const(0));
    assert!(!composed.library_steps.is_complete());
}

fn assert_uncertified(document: Value) {
    let programs = programs(document);
    let result = composition(&programs, KEY);
    assert!(!matches!(result, Composed::Proven(_)), "{result:?}");
}

#[test]
fn nominal_builtin_patterns_certify_raw_formal_state_without_producer_provenance() {
    for pattern in [Pattern::String, Pattern::Character] {
        assert_certified(document(pattern), 1);
        let mut borrowed = document(pattern);
        let function = check(&mut borrowed);
        function["signature"]["parameters"][0]["type"] = json!("state:borrowed-iterator");
        function["symbols"][0]["type"] = json!("state:borrowed-iterator");
        let receiver = node(function, "check.receiver");
        receiver["type"] = json!("state:borrowed-iterator");
        receiver["adjustments"] = json!([]);
        assert_certified(borrowed, 1);
    }
}

fn alias(document: &mut Value) {
    let function = check(document);
    function["symbols"].as_array_mut().unwrap().push(json!({
        "id":"alias","name":"alias","kind":"local","scope":"check.root",
        "type":"state:iterator","mutable":true,"node":"check.alias-pattern"
    }));
    function["nodes"].as_array_mut().unwrap().extend([
        expression(
            "check.alias-pattern",
            "pattern_binding",
            &[],
            "state:iterator",
        ),
        expression("check.initial", "identifier", &[], "state:iterator"),
        expression(
            "check.let",
            "local_declaration",
            &["check.alias-pattern", "check.initial"],
            "type:unit",
        ),
    ]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .push(reference("check.initial", "symbol", "iterator"));
    node(function, "check.root")["children"] = json!(["check.let", "check.call"]);
    call_reference(document, "check.receiver")["target"]["id"] = json!("alias");
}

#[test]
fn moved_local_aliases_keep_the_same_certified_nominal_state() {
    for pattern in [Pattern::String, Pattern::Character] {
        let mut document = document(pattern);
        alias(&mut document);
        assert_certified(document, 1);
    }
}

fn constructor(document: &mut Value, pattern: Pattern, stored: bool) {
    let function = check(document);
    function["signature"]["parameters"] =
        json!([{"position":0,"symbol":"source","type":"state:shared-str"}]);
    function["symbols"] = json!([parameter("source", "state:shared-str")]);
    function["nodes"].as_array_mut().unwrap().extend([
        expression("check.source", "identifier", &[], "state:shared-str"),
        expression("check.pattern", "literal", &[], pattern.ty()),
        expression(
            "check.producer",
            "call",
            &["check.source", "check.pattern"],
            "state:iterator",
        ),
    ]);
    node(function, "check.pattern")["value"] = pattern.literal();
    let mut producer = reference("check.producer", "definition", "state:rsplit");
    producer["dispatch"] = json!("method");
    producer["instance_arguments"] = json!([pattern.ty()]);
    producer["signature"] =
        json!({"parameters":["state:shared-str",pattern.ty()],"result":"state:iterator"});
    producer["calls_back"] = json!({"value":true,"reason":"Generic Pattern may run a callable"});
    function["refs"]
        .as_array_mut()
        .unwrap()
        .extend([producer, reference("check.source", "symbol", "source")]);
    constructor_receiver(document, stored);
}

fn constructor_receiver(document: &mut Value, stored: bool) {
    let function = check(document);
    if stored {
        function["symbols"].as_array_mut().unwrap().push(json!({
            "id":"iterator","name":"iterator","kind":"local","scope":"check.root",
            "type":"state:iterator","mutable":true,"node":"check.state-pattern"
        }));
        function["nodes"].as_array_mut().unwrap().extend([
            expression(
                "check.state-pattern",
                "pattern_binding",
                &[],
                "state:iterator",
            ),
            expression(
                "check.let",
                "local_declaration",
                &["check.state-pattern", "check.producer"],
                "type:unit",
            ),
        ]);
        node(function, "check.root")["children"] = json!(["check.let", "check.call"]);
    } else {
        node(function, "check.producer")["adjustments"] =
            json!([{"kind":"auto_ref","mode":"exclusive","target":"state:borrowed-iterator"}]);
        node(function, "check.call")["children"] = json!(["check.producer"]);
        function["nodes"]
            .as_array_mut()
            .unwrap()
            .retain(|node| node["id"] != "check.receiver");
        function["refs"]
            .as_array_mut()
            .unwrap()
            .retain(|reference| reference["node"] != "check.receiver");
    }
}

#[test]
fn direct_and_named_constructors_retain_both_original_library_calls() {
    for pattern in [Pattern::String, Pattern::Character] {
        for stored in [false, true] {
            let mut document = document(pattern);
            constructor(&mut document, pattern, stored);
            assert_certified(document, 2);
        }
    }
}

#[test]
fn nominal_and_pattern_facts_must_be_complete_and_exact() {
    for arguments in [
        Value::Null,
        json!([]),
        json!(["state:shared-str", "state:char"]),
        json!(["state:opaque"]),
        json!(["state:pattern"]),
        json!(["state:predicate"]),
        json!(["state:closure"]),
    ] {
        let mut document = document(Pattern::String);
        ty(&mut document, "state:iterator")["arguments"] = arguments;
        assert_uncertified(document);
    }
    let mut counterfeit = document(Pattern::String);
    definition(&mut counterfeit, "state:RSplit")["standard"] = json!(false);
    assert_uncertified(counterfeit);
    let mut destructor = document(Pattern::String);
    definition(&mut destructor, "state:RSplit")["destructor"] = json!(true);
    assert_uncertified(destructor);
}

fn constructor_pattern(document: &mut Value, pattern: &str) {
    ty(document, "state:iterator")["arguments"] = json!([pattern]);
    let function = check(document);
    function["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"position":1,"symbol":"pattern","type":pattern}));
    function["symbols"]
        .as_array_mut()
        .unwrap()
        .push(parameter("pattern", pattern));
    let operand = node(function, "check.pattern");
    operand["kind"] = json!("identifier");
    operand["type"] = json!(pattern);
    operand.as_object_mut().unwrap().remove("value");
    function["refs"]
        .as_array_mut()
        .unwrap()
        .push(reference("check.pattern", "symbol", "pattern"));
    let producer = call_reference(document, "check.producer");
    producer["instance_arguments"] = json!([pattern]);
    producer["signature"]["parameters"][1] = json!(pattern);
}

#[test]
fn callable_and_unknown_patterns_cannot_be_certified_by_the_constructor() {
    for pattern in ["state:predicate", "state:closure", "state:opaque"] {
        let mut document = document(Pattern::String);
        constructor(&mut document, Pattern::String, false);
        constructor_pattern(&mut document, pattern);
        assert_uncertified(document);
    }
}

#[test]
fn unknown_or_different_nominals_and_reference_modes_do_not_match_the_state() {
    for identity in ["workspace::RSplit", "core::iter::adapters::map::Map"] {
        let mut document = document(Pattern::String);
        definition(&mut document, "state:RSplit")["stable_identity"] = json!(identity);
        assert_uncertified(document);
    }
    let mut absent = document(Pattern::String);
    ty(&mut absent, "state:iterator")["definition"] = Value::Null;
    assert_uncertified(absent);
    let mut shared = document(Pattern::String);
    ty(&mut shared, "state:borrowed-iterator")["mode"] = json!("shared");
    assert_uncertified(shared);
    let mut bad_pattern = document(Pattern::String);
    ty(&mut bad_pattern, "state:shared-str")["mode"] = json!("exclusive");
    assert_uncertified(bad_pattern);
}

fn bad_selection(document: &mut Value, variant: u32) {
    match variant {
        0 => call_reference(document, "check.call")["selected"] = Value::Null,
        1 => definition(document, "state:selected")["standard"] = json!(false),
        2 => definition(document, "state:selected")["implements"] = json!("state:rsplit"),
        3 => definition(document, "state:selected")["callable"]["owner_type"] = Value::Null,
        4 => {
            definition(document, "state:selected")["callable"]["receiver"] =
                json!("state:prototype")
        }
        5 => definition(document, "state:selected")["scope"] = json!("unit"),
        6 => call_reference(document, "check.call")["dispatch"] = json!("dynamic"),
        7 => {
            let call = call_reference(document, "check.call");
            call["status"] = json!("dependent");
            call["via"] = json!("state:next");
        }
        _ => unreachable!(),
    }
}

#[test]
fn selection_requires_the_exact_resolved_standard_implementation_link() {
    for variant in 0..8 {
        let mut document = document(Pattern::String);
        bad_selection(&mut document, variant);
        assert_uncertified(document);
    }
    let mut wrong_owner = document(Pattern::String);
    ty(&mut wrong_owner, "state:prototype")["definition"] = json!("state:Option");
    assert_uncertified(wrong_owner);
}

#[test]
fn instantiated_receiver_rosters_and_signatures_are_mandatory_and_coherent() {
    for arguments in [
        Value::Null,
        json!([]),
        json!(["state:prototype"]),
        json!(["state:iterator", "state:char"]),
    ] {
        let mut document = document(Pattern::String);
        call_reference(&mut document, "check.call")["instance_arguments"] = arguments;
        assert_uncertified(document);
    }
    for signature in [
        Value::Null,
        json!({"parameters":[],"result":"state:option-str"}),
        json!({"parameters":[null],"result":"state:option-str"}),
        json!({"parameters":["state:iterator"],"result":"state:option-str"}),
        json!({"parameters":["state:borrowed-iterator"],"result":null}),
        json!({"parameters":["state:borrowed-iterator"],"result":"type:bool"}),
    ] {
        let mut document = document(Pattern::String);
        call_reference(&mut document, "check.call")["signature"] = signature;
        assert_uncertified(document);
    }
}

fn named_execution(document: &mut Value, site: &str, driven: bool) {
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"state:external","kind":"function","name":"external","stable_identity":"external",
            "scope":"library","standard":true,"operation":"rust:File.read"
        }));
    let reference = call_reference(document, site);
    if driven {
        reference["drives"] = json!(["state:external"]);
    } else {
        reference["calls_back"]["runs"] = json!([{"kind":"definition","id":"state:external"}]);
    }
}

#[test]
fn named_execution_is_never_erased_at_the_producer_or_pull() {
    for site in ["check.producer", "check.call"] {
        for driven in [false, true] {
            let mut document = document(Pattern::String);
            constructor(&mut document, Pattern::String, false);
            named_execution(&mut document, site, driven);
            assert_uncertified(document);
        }
    }
}

#[test]
fn adapter_states_and_owning_adapter_outputs_still_require_their_own_model() {
    let mut document = document(Pattern::String);
    definition(&mut document, "state:RSplit")["stable_identity"] =
        json!("core::iter::adapters::map::Map");
    ty(&mut document, "state:iterator")["arguments"] = json!(["state:closure"]);
    let function = check(&mut document);
    function["signature"]["result"] = json!("state:option-guard");
    node(function, "check.call")["type"] = json!("state:option-guard");
    node(function, "check.root")["type"] = json!("state:option-guard");
    let reference = call_reference(&mut document, "check.call");
    reference["signature"]["result"] = json!("state:option-guard");
    reference["item"] = json!("state:guard");
    assert_uncertified(document);
}

#[test]
fn an_empty_pattern_adds_no_cardinality_or_library_cost_contract() {
    let contracts = Contracts::embedded();
    assert!(contracts.extent("rust:str.rsplit").is_none());
    assert!(contracts.consumer("rust:Iterator.next").is_none());
    assert!(contracts.get("rust:str.rsplit").is_none());
    let mut document = document(Pattern::String);
    constructor(&mut document, Pattern::String, false);
    node(check(&mut document), "check.pattern")["value"] = json!({"kind":"string","value":""});
    let function = check(&mut document);
    function["signature"]["parameters"] = json!([]);
    function["symbols"] = json!([]);
    function["refs"]
        .as_array_mut()
        .unwrap()
        .retain(|reference| reference["node"] != "check.source");
    let source = node(function, "check.source");
    source["kind"] = json!("literal");
    source["value"] = json!({"kind":"string","value":""});
    assert_certified(document, 2);
}

#[test]
fn a_loop_driven_by_next_does_not_gain_a_termination_or_cardinality_proof() {
    let mut document = document(Pattern::String);
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id":"state:is-some","kind":"method","name":"Option::is_some","stable_identity":"standard::is_some",
        "scope":"library","standard":true,"operation":"rust:Option.is_some"
    }));
    let function = check(&mut document);
    function["signature"]["result"] = json!("type:unit");
    node(function, "check.root")["type"] = json!("type:unit");
    node(function, "check.root")["children"] = json!(["check.while"]);
    function["nodes"].as_array_mut().unwrap().extend([
        expression("check.is-some", "call", &["check.call"], "type:bool"),
        expression("check.body", "block", &[], "type:unit"),
        expression(
            "check.while",
            "while",
            &["check.is-some", "check.body"],
            "type:unit",
        ),
    ]);
    let mut is_some = reference("check.is-some", "definition", "state:is-some");
    is_some["dispatch"] = json!("method");
    is_some["calls_back"] = json!({"value":false,"reason":"No callback"});
    function["refs"].as_array_mut().unwrap().push(is_some);
    assert_uncertified(document);
}

fn compute_nodes(name: &str) -> Value {
    let id = |part: &str| format!("{name}.{part}");
    json!([
        expression(
            &id("root"),
            "block",
            &[&id("let"), &id("while-statement"), &id("payload")],
            "state:shared-str"
        ),
        expression(
            &id("let"),
            "local_declaration",
            &[&id("pattern"), &id("zero")],
            "type:unit"
        ),
        expression(&id("pattern"), "pattern_binding", &[], "type:u32"),
        expression(&id("zero"), "literal", &[], "type:u32"),
        expression(&id("limit"), "literal", &[], "type:u32"),
        expression(&id("one"), "literal", &[], "type:u32"),
        expression(
            &id("while-statement"),
            "expression_statement",
            &[&id("while")],
            "type:unit"
        ),
        expression(
            &id("while"),
            "while",
            &[&id("less"), &id("body")],
            "type:unit"
        ),
        expression(
            &id("less"),
            "operator",
            &[&id("read"), &id("limit")],
            "type:bool"
        ),
        expression(&id("read"), "identifier", &[], "type:u32"),
        expression(&id("body"), "block", &[&id("increment")], "type:unit"),
        expression(
            &id("increment"),
            "compound_assignment",
            &[&id("write"), &id("one")],
            "type:unit"
        ),
        expression(&id("write"), "identifier", &[], "type:u32"),
        expression(&id("payload"), "identifier", &[], "state:shared-str"),
    ])
}

fn compute_function(name: &str, limit: u32) -> Value {
    let id = |part: &str| format!("{name}.{part}");
    let mut function = blank(name);
    function["signature"]["parameters"] =
        json!([{"position":0,"symbol":"payload","type":"state:shared-str"}]);
    function["signature"]["result"] = json!("state:shared-str");
    function["symbols"] = json!([
        parameter("payload", "state:shared-str"),
        {"id":"i","name":"i","kind":"local","scope":id("root"),"type":"type:u32","mutable":true,"node":id("pattern")}
    ]);
    function["nodes"] = compute_nodes(name);
    for (part, value) in [
        ("zero", "0".to_owned()),
        ("one", "1".to_owned()),
        ("limit", limit.to_string()),
    ] {
        node(&mut function, &id(part))["value"] = json!({"kind":"integer","value":value});
    }
    node(&mut function, &id("less"))["operator"] = json!("lt");
    node(&mut function, &id("increment"))["operator"] = json!("add");
    function["refs"] = json!([
        reference(&id("read"), "symbol", "i"),
        reference(&id("write"), "symbol", "i"),
        reference(&id("payload"), "symbol", "payload"),
    ]);
    function
}

fn computed_operand(document: &mut Value, site: &str, limit: u32) {
    let name = site.replace('.', "_");
    let definition = format!("state:{name}-definition");
    let function_type = format!("state:{name}-type");
    let function = compute_function(&name, limit);
    document["unit"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(function);
    document["unit"]["definitions"].as_array_mut().unwrap().push(json!({
        "id":definition,"kind":"function","name":name,"stable_identity":format!("iterator_tests::{name}"),
        "scope":"unit","compilation":"iterator-tests","callable":{"parameters":["state:shared-str"],"result":"state:shared-str","generics":0,"modifiers":[]}
    }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":function_type,"kind":"function","spelling":name,"definition":definition,
            "parameters":["state:shared-str"],"result":"state:shared-str"
        }));
    replace_operand(document, site, &definition, &function_type);
}

fn replace_operand(document: &mut Value, site: &str, definition: &str, function_type: &str) {
    let argument_id = format!("{site}-argument");
    let callee_id = format!("{site}-callee");
    let function = check(document);
    let mut argument = node(function, site).clone();
    argument["id"] = json!(argument_id);
    let operand = node(function, site);
    operand["kind"] = json!("call");
    operand["children"] = json!([callee_id, argument_id]);
    operand.as_object_mut().unwrap().remove("value");
    function["nodes"].as_array_mut().unwrap().extend([
        argument,
        expression(&callee_id, "identifier", &[], function_type),
    ]);
    for reference in function["refs"].as_array_mut().unwrap().iter_mut() {
        if reference["node"] == site {
            reference["node"] = json!(argument_id);
        }
    }
    let mut call = reference(site, "definition", definition);
    call["dispatch"] = json!("static");
    function["refs"]
        .as_array_mut()
        .unwrap()
        .extend([call, reference(&callee_id, "definition", definition)]);
}

#[test]
fn computed_source_and_pattern_calls_keep_their_workspace_work() {
    let mut document = document(Pattern::String);
    constructor(&mut document, Pattern::String, false);
    computed_operand(&mut document, "check.source", 2);
    computed_operand(&mut document, "check.pattern", 3);
    let programs = programs(document);
    let assessment = local(&programs[KEY]);
    for name in ["check_source", "check_pattern"] {
        let helper = format!("iterator-tests/iterator_tests::{name}");
        let expected = if name == "check_source" { 2 } else { 3 };
        assert_eq!(
            local(&programs[&helper]).loop_body_entries,
            Bound::Const(expected)
        );
        assert_eq!(
            assessment
                .calls
                .iter()
                .find(|call| call.callee == name)
                .unwrap()
                .count,
            Bound::Const(1)
        );
    }
    let result = composition(&programs, KEY);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, Bound::Const(5));
    assert_eq!(composed.workspace_invocations, Bound::Const(2));
    assert!(!composed.library_steps.is_complete());
}

fn saved(document: &str, provenance: &str) -> Value {
    let parsed: Value = serde_json::from_str(document).unwrap();
    let provenance: Value = serde_json::from_str(provenance).unwrap();
    let checksum = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
    assert_eq!(
        checksum(document.as_bytes()),
        provenance["fixture_checksum"]
    );
    for retained in provenance["retained_functions"].as_array().unwrap() {
        let function = parsed["unit"]["functions"]
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
    for source in parsed["unit"]["files"].as_array().unwrap() {
        assert_eq!(
            checksum(source["content"].as_str().unwrap().as_bytes()),
            source["digest"]
        );
    }
    parsed
}

fn retained_state_calls(assessment: &Assessment, expected: usize) {
    let calls: Vec<_> = assessment
        .calls
        .iter()
        .filter(|call| {
            call.operation.as_deref() == Some("rust:str.rsplit")
                || matches!(&call.target, Callee::Library(identity)
                    if identity == "core::str::iter::{impl#64}::next")
        })
        .collect();
    assert_eq!(calls.len(), expected);
    assert!(
        calls
            .iter()
            .all(|call| matches!(call.target, Callee::Library(_)))
    );
}

#[test]
fn exact_short_paths_capture_gains_composition_of_its_existing_formal_bound() {
    let programs = programs(saved(PATHS, PATHS_PROVENANCE));
    let (key, program) = programs.iter().next().unwrap();
    let local = local(program);
    retained_state_calls(&local, 4);
    assert_eq!(local.loop_body_entries, Bound::Input("len(owner)#0".into()));
    assert_eq!(program.extents.len(), 1);
    assert_eq!(program.extents[0].root, Root::Formal(0));
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, local.loop_body_entries);
    assert_eq!(composed.workspace_invocations, Bound::Const(0));
    assert!(!composed.library_steps.is_complete());
}

#[test]
fn exact_variant_capture_does_not_promote_its_derived_last_substring_to_formal_input() {
    let programs = programs(saved(VARIANT, VARIANT_PROVENANCE));
    let (key, program) = programs
        .iter()
        .find(|(key, _)| key.ends_with("::is_unit_variant"))
        .unwrap();
    let local = local(program);
    retained_state_calls(&local, 2);
    assert_eq!(local.loop_body_entries, Bound::Input("len(last)#1".into()));
    let extent = program
        .extents
        .iter()
        .find(|extent| extent.input == "len(last)#1")
        .unwrap();
    assert_eq!(extent.root, Root::Local(1));
    let result = composition(&programs, key);
    let Composed::Proven(composed) = result else {
        panic!("{result:?}")
    };
    assert_eq!(composed.loop_body_entries, local.loop_body_entries);
    assert_eq!(composed.workspace_invocations, Bound::Const(1));
    assert!(!composed.library_steps.is_complete());
}
