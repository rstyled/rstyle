use super::*;
use crate::cpg::bounds::{facts::Facts, frame};
use crate::cpg::model::{Document, Type};
use crate::cpg::source::Sources;
use crate::cpg::tree::{Body, Tables};
use crate::runtime_bounds::{Action, Arithmetic, Flow};
use serde_json::{Value, json};

const PAIRS: &str = include_str!("../../../testdata/iterator-locals-pairs-production.cpg.json");
const PRODUCERS: &str =
    include_str!("../../../testdata/iterator-locals-data-producers-production.cpg.json");
const CONSUMERS: &str =
    include_str!("../../../testdata/iterator-locals-data-consumers-production.cpg.json");
const CLASSES: &str = include_str!("../../../testdata/iterator-locals-classes-production.cpg.json");

#[derive(Clone, Copy, Debug)]
enum Site {
    Root,
    Declaration,
    Pattern,
    Initializer,
    Source,
    Receiver,
    Tail,
    Adapter,
    Use,
    InitialCallback,
    Callback,
    ConsumerCallback,
}

fn document(source: &str) -> Value {
    serde_json::from_str(source).unwrap()
}

fn function(document: &mut Value) -> &mut Value {
    &mut document["unit"]["functions"][0]
}

fn item<'a>(items: &'a mut Value, key: &str, id: &str) -> &'a mut Value {
    items
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item[key] == id)
        .unwrap()
}

fn id(document: &Value, site: Site) -> String {
    let owner = &document["unit"]["functions"][0];
    if matches!(site, Site::Root) {
        return owner["root"].as_str().unwrap().to_owned();
    }
    let (parent, index) = match site {
        Site::Declaration => (Site::Root, 0),
        Site::Tail => (Site::Root, 1),
        Site::Pattern => (Site::Declaration, 0),
        Site::Initializer => (Site::Declaration, 1),
        Site::Source => (Site::Initializer, 0),
        Site::Receiver => (Site::Source, 0),
        Site::Adapter => (Site::Tail, 0),
        Site::Use => (Site::Adapter, 0),
        Site::InitialCallback => (Site::Initializer, 1),
        Site::Callback => (Site::Adapter, 1),
        Site::ConsumerCallback => (Site::Tail, 1),
        Site::Root => unreachable!(),
    };
    let parent = id(document, parent);
    owner["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == parent)
        .unwrap()["children"][index]
        .as_str()
        .unwrap()
        .to_owned()
}

fn node(document: &mut Value, site: Site) -> &mut Value {
    let id = id(document, site);
    item(&mut function(document)["nodes"], "id", &id)
}

fn reference(document: &mut Value, site: Site) -> &mut Value {
    let id = id(document, site);
    item(&mut function(document)["refs"], "node", &id)
}

fn definition(document: &mut Value, site: Site) -> &mut Value {
    let id = reference(document, site)["target"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    item(&mut document["unit"]["definitions"], "id", &id)
}

fn state_type(document: &mut Value, site: Site) -> &mut Value {
    let id = node(document, site)["type"].as_str().unwrap().to_owned();
    item(&mut document["unit"]["types"], "id", &id)
}

fn with_collector<R>(value: Value, check: impl FnOnce(&mut Collector<'_, '_, '_>) -> R) -> R {
    let document: Document = serde_json::from_value(value).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let body = Body::new(&unit.functions.as_ref().unwrap()[0]).unwrap();
    let tables = Tables::new(unit).unwrap();
    let sources = Sources::new(unit).unwrap();
    let facts = Facts {
        body: &body,
        tables: &tables,
        sources: &sources,
        pointer_bits: 64,
        destructors_are_calls: false,
    };
    let mut collector = Collector::new(&facts, Arithmetic::Checked);
    let parameters = collector.parameter_symbols().unwrap();
    collector.formal_positions(&parameters);
    check(&mut collector)
}

fn with_frame<R>(value: Value, check: impl FnOnce(&mut Collector<'_, '_, '_>) -> R) -> R {
    with_collector(value, |collector| {
        let parameters = collector.parameter_symbols().unwrap();
        let frame = frame::inspect(collector.facts, &parameters).unwrap();
        collector
            .install_extents(&parameters, frame.extents)
            .unwrap();
        collector.parameters(&parameters).unwrap();
        check(collector)
    })
}

fn prepared(value: Value) -> bool {
    with_frame(value, |collector| {
        collector
            .prepare_deferred(collector.facts.body.root().unwrap())
            .unwrap();
        collector.deferred.is_some()
    })
}

fn call_valid(value: Value, site: Site) -> bool {
    let id = id(&value, site);
    with_collector(value, |collector| {
        collector.deferred_call(collector.facts.body.node(&id).unwrap())
    })
}

#[test]
fn production_bindings_retain_both_pipeline_halves_in_order() {
    for source in [PAIRS, PRODUCERS, CONSUMERS, CLASSES] {
        let value = document(source);
        let init = id(&value, Site::Initializer);
        let adapter = id(&value, Site::Adapter);
        with_frame(value, |collector| {
            let initial = collector
                .chain(collector.facts.body.node(&init).unwrap())
                .unwrap()
                .unwrap();
            assert!(!initial.initialized);
            assert_eq!(initial.stages.len(), 1);
            collector
                .prepare_deferred(collector.facts.body.root().unwrap())
                .unwrap();
            assert!(collector.deferred.is_some());
            let combined = collector
                .chain(collector.facts.body.node(&adapter).unwrap())
                .unwrap()
                .unwrap();
            assert!(combined.initialized && combined.standard);
            assert_eq!(combined.root.id, initial.root.id);
            assert_eq!(combined.stages.len(), 2);
            assert_eq!(matches!(combined.stages[0], Stage::Pass), source == PAIRS);
            assert_eq!(
                matches!(combined.stages[1], Stage::Filter(_)),
                source == PAIRS
            );
            assert!(combined.arguments.is_empty());
        });
    }
}

#[test]
fn only_an_adjacent_plain_block_can_prepare_a_binding() {
    for (field, value) in [
        ("kind", json!("tuple")),
        ("label", json!("named")),
        ("modifiers", json!(["unsafe"])),
        ("children", json!([])),
    ] {
        let mut changed = document(PAIRS);
        node(&mut changed, Site::Root)[field] = value;
        assert!(!prepared(changed), "{field}");
    }
    let mut changed = document(PAIRS);
    node(&mut changed, Site::Pattern)["binding_mode"] = json!("exclusive");
    assert!(!prepared(changed));
}

#[test]
fn every_crossing_call_requires_complete_static_resolution() {
    for site in [Site::Source, Site::Initializer, Site::Adapter, Site::Tail] {
        assert!(call_valid(document(PAIRS), site));
        for (field, value) in [
            ("status", json!("dependent")),
            ("dispatch", json!("dynamic")),
            ("arguments", json!([])),
            ("drives", json!([])),
            (
                "calls_back",
                json!({"value":true,"reason":"explicit","runs":[]}),
            ),
            ("target", json!(null)),
            ("target", json!({"kind":"symbol","id":"absent"})),
            ("target", json!({"kind":"definition","id":"absent"})),
            ("selected", json!({"kind":"symbol","id":"absent"})),
            ("selected", json!({"kind":"definition","id":"absent"})),
        ] {
            let mut changed = document(PAIRS);
            reference(&mut changed, site)[field] = value;
            assert!(!call_valid(changed, site), "{site:?}: {field}");
        }
        let mut absent = document(PAIRS);
        let call = id(&absent, site);
        function(&mut absent)["refs"]
            .as_array_mut()
            .unwrap()
            .retain(|reference| reference["node"] != call);
        assert!(!call_valid(absent, site));
    }
}

#[test]
fn iteration_selection_and_standard_method_identity_are_required() {
    for site in [Site::Source, Site::Initializer, Site::Adapter, Site::Tail] {
        for (field, value) in [
            ("standard", json!(false)),
            ("scope", json!("unit")),
            ("kind", json!("function")),
            ("callable", json!(null)),
            ("modifiers", json!(["variadic"])),
        ] {
            let mut changed = document(PAIRS);
            definition(&mut changed, site)[field] = value;
            assert!(!call_valid(changed, site), "{site:?}: {field}");
        }
        if !matches!(site, Site::Source) {
            let mut missing = document(PAIRS);
            reference(&mut missing, site)["selected"] = json!(null);
            assert!(!call_valid(missing, site));
        }
    }
}

#[test]
fn callable_layout_requires_an_unadjusted_fixed_arity_receiver() {
    for (field, value) in [
        ("receiver", json!(null)),
        ("parameters", json!(["type:usize"])),
        ("modifiers", json!(["variadic"])),
    ] {
        let mut changed = document(PAIRS);
        definition(&mut changed, Site::Initializer)["callable"][field] = value;
        assert!(!call_valid(changed, Site::Initializer), "{field}");
    }
    for (site, field, value) in [
        (Site::Initializer, "kind", json!("tuple")),
        (Site::Initializer, "children", json!([])),
        (
            Site::Initializer,
            "adjustments",
            json!([{"kind":"auto_ref"}]),
        ),
        (Site::Source, "adjustments", json!([{"kind":"auto_ref"}])),
    ] {
        let mut changed = document(PAIRS);
        node(&mut changed, site)[field] = value;
        assert!(!call_valid(changed, Site::Initializer), "{site:?}: {field}");
    }
}

#[test]
fn instantiated_signatures_must_name_every_argument_and_result() {
    for signature in [
        json!(null),
        json!({"parameters":[],"result":null}),
        json!({"parameters":[null],"result":null}),
        json!({"parameters":["type:usize"],"result":"type:usize"}),
    ] {
        let mut changed = document(PAIRS);
        reference(&mut changed, Site::Initializer)["signature"] = signature;
        assert!(!call_valid(changed, Site::Initializer));
    }
    for (field, value) in [
        ("parameters", json!([null])),
        ("parameters", json!(["absent"])),
        ("result", json!(null)),
        ("result", json!("absent")),
    ] {
        let mut changed = document(PAIRS);
        reference(&mut changed, Site::Initializer)["signature"][field] = value;
        assert!(!call_valid(changed, Site::Initializer), "{field}");
    }
    let mut incomplete = document(PAIRS);
    node(&mut incomplete, Site::Receiver)["adjustments"][1]["target"] = json!(null);
    assert!(!call_valid(incomplete, Site::Source));
    let mut unknown = document(PAIRS);
    node(&mut unknown, Site::Source)["type"] = json!("absent");
    reference(&mut unknown, Site::Initializer)["signature"]["parameters"][0] = json!("absent");
    assert!(!call_valid(unknown, Site::Initializer));
}

#[test]
fn signature_validation_rejects_missing_child_and_unknown_type_ids() {
    with_collector(document(PAIRS), |collector| {
        let root = collector.facts.body.root().unwrap();
        let local = collector.facts.body.child(root, 0).unwrap();
        let call = collector.facts.body.child(local, 1).unwrap();
        let reference = collector.facts.reference(call).unwrap();
        let mut missing_child = call.clone();
        missing_child.children[0] = "absent".into();
        assert!(!collector.deferred_signature(&missing_child, reference));
        let mut missing_result = call.clone();
        missing_result.ty = Some("absent".into());
        let mut signature = reference.clone();
        signature.signature.as_mut().unwrap().result = Some("absent".into());
        assert!(!collector.deferred_signature(&missing_result, &signature));
    });
}

fn state_valid(value: Value, site: Site) -> bool {
    let id = id(&value, site);
    with_collector(value, |collector| {
        collector.deferred_state(collector.facts.body.node(&id).unwrap())
    })
}

#[test]
fn iterator_state_requires_a_standard_nominal_record_without_drop() {
    for site in [Site::Source, Site::Initializer] {
        assert!(state_valid(document(PAIRS), site));
        for (field, value) in [
            ("kind", json!("opaque")),
            ("definition", json!(null)),
            ("definition", json!("absent")),
        ] {
            let mut changed = document(PAIRS);
            state_type(&mut changed, site)[field] = value;
            assert!(!state_valid(changed, site), "{field}");
        }
        for value in [json!(null), json!("absent")] {
            let mut changed = document(PAIRS);
            node(&mut changed, site)["type"] = value;
            assert!(!state_valid(changed, site));
        }
    }
}

#[test]
fn iterator_state_definition_requires_explicit_library_cleanup_facts() {
    for (field, value) in [
        ("kind", json!("function")),
        ("standard", json!(false)),
        ("scope", json!("unit")),
        ("destructor", json!(true)),
    ] {
        let mut changed = document(PAIRS);
        let nominal = state_type(&mut changed, Site::Initializer)["definition"]
            .as_str()
            .unwrap()
            .to_owned();
        item(&mut changed["unit"]["definitions"], "id", &nominal)[field] = value;
        assert!(!state_valid(changed, Site::Initializer), "{field}");
    }
}

fn source_valid(value: Value) -> bool {
    let source = id(&value, Site::Source);
    with_collector(value, |collector| {
        collector
            .deferred_source(collector.facts.body.node(&source).unwrap())
            .unwrap()
    })
}

#[test]
fn deferred_sources_require_a_stable_formal_place_and_traversal_contract() {
    for (site, field, value) in [
        (Site::Source, "children", json!([])),
        (
            Site::Receiver,
            "adjustments",
            json!([{"kind":"pointer_coercion"}]),
        ),
        (Site::Receiver, "kind", json!("literal")),
    ] {
        let mut changed = document(PAIRS);
        node(&mut changed, site)[field] = value;
        assert!(!source_valid(changed), "{site:?}: {field}");
    }
    let mut role = document(PAIRS);
    definition(&mut role, Site::Source)["operation"] = json!("rust:slice.len");
    assert!(!source_valid(role));
    let mut unresolved = document(PAIRS);
    reference(&mut unresolved, Site::Source)["status"] = json!("unresolved");
    assert!(!source_valid(unresolved));
    let value = document(PAIRS);
    let source = id(&value, Site::Source);
    with_collector(value, |collector| {
        collector.formals.clear();
        assert!(
            !collector
                .deferred_source(collector.facts.body.node(&source).unwrap())
                .unwrap()
        );
    });
}

#[test]
fn resolved_source_signatures_do_not_authorize_nonstandard_receiver_adjustments() {
    let mut value = document(PAIRS);
    let source = id(&value, Site::Source);
    node(&mut value, Site::Receiver)["adjustments"][0]["kind"] = json!("workspace_deref");
    with_collector(value, |collector| {
        let source = collector.facts.body.node(&source).unwrap();
        assert!(has_role(collector.facts, source, ExtentRole::Traversal));
        assert!(collector.deferred_call(source));
        assert!(!collector.deferred_source(source).unwrap());
    });
}

#[test]
fn initializer_chain_flags_shape_and_arguments_must_be_certified() {
    let value = document(PAIRS);
    let init = id(&value, Site::Initializer);
    with_frame(value, |collector| {
        let init = collector.facts.body.node(&init).unwrap();
        let mut chain = collector.chain(init).unwrap().unwrap();
        chain.standard = false;
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
        chain.standard = true;
        chain.initialized = true;
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
        chain.initialized = false;
        chain.stages.clear();
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
        chain.stages.push(Stage::FlatMap(chain.root));
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
        chain.stages[0] = Stage::Pass;
        chain.arguments.push(init);
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
        chain.arguments.clear();
        chain.stages.push(Stage::Pass);
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
        chain.root = init;
        assert!(!collector.deferred_initializer(&chain, init).unwrap());
    });
}

#[test]
fn deferred_initializer_rejects_non_opted_in_and_incomplete_stage_facts() {
    for (field, value) in [
        ("operation", json!("rust:Iterator.take")),
        ("operation", json!("rust:Iterator.unknown")),
    ] {
        let mut changed = document(PAIRS);
        let init = id(&changed, Site::Initializer);
        definition(&mut changed, Site::Initializer)[field] = value;
        assert!(!prepared(changed.clone()));
        with_frame(changed, |collector| {
            let init = collector.facts.body.node(&init).unwrap();
            let source = collector.facts.body.child(init, 0).unwrap();
            let mut chain = collector.chain_source(source).unwrap().unwrap();
            chain.stages.push(Stage::Pass);
            assert!(!collector.deferred_initializer(&chain, init).unwrap());
        });
    }
    for (field, value) in [("item", json!(null)), ("item", json!("absent"))] {
        let mut changed = document(PAIRS);
        reference(&mut changed, Site::Initializer)[field] = value;
        assert!(!prepared(changed));
    }
}

fn capture_valid(value: Value) -> bool {
    let callback = id(&value, Site::InitialCallback);
    with_collector(value, |collector| {
        collector.deferred_capture(Callback::Closure(
            collector.facts.body.node(&callback).unwrap(),
        ))
    })
}

#[test]
fn initializer_closures_require_explicit_simple_capture_environments() {
    assert!(capture_valid(document(PRODUCERS)));
    assert!(capture_valid(document(CLASSES)));
    for (field, value) in [
        ("captures", json!(null)),
        ("modifiers", json!(["async"])),
        ("adjustments", json!([{"kind":"auto_ref"}])),
    ] {
        let mut changed = document(PRODUCERS);
        node(&mut changed, Site::InitialCallback)[field] = value;
        assert!(!capture_valid(changed), "{field}");
    }
    let mut owning = document(PRODUCERS);
    node(&mut owning, Site::InitialCallback)["captures"][0]["mode"] = json!("value");
    assert!(!prepared(owning));
    for (field, value) in [
        ("mode", json!("value")),
        ("mode", json!("exclusive")),
        ("mode", json!("unique")),
        ("path", json!([{"kind":"deref","type":"type:usize"}])),
        ("symbol", json!("absent")),
        ("type", json!("absent")),
    ] {
        let mut changed = document(PRODUCERS);
        node(&mut changed, Site::InitialCallback)["captures"][0][field] = value;
        assert!(!capture_valid(changed), "{field}");
    }
}

#[test]
fn captures_require_the_same_immutable_primitive_formal_symbol() {
    for (field, value) in [("mutable", json!(true)), ("type", json!("type:bool"))] {
        let mut changed = document(PRODUCERS);
        let symbol = node(&mut changed, Site::InitialCallback)["captures"][0]["symbol"]
            .as_str()
            .unwrap()
            .to_owned();
        item(&mut function(&mut changed)["symbols"], "id", &symbol)[field] = value;
        assert!(!capture_valid(changed), "{field}");
    }
    let mut local = document(PRODUCERS);
    let symbol = function(&mut local)["symbols"][2]["id"].clone();
    node(&mut local, Site::InitialCallback)["captures"][0]["symbol"] = symbol;
    assert!(!capture_valid(local));
    for kind in ["opaque", "reference"] {
        let mut changed = document(PRODUCERS);
        item(&mut changed["unit"]["types"], "id", "type:usize")["kind"] = json!(kind);
        assert!(!capture_valid(changed));
    }
    with_collector(document(PRODUCERS), |collector| {
        assert!(collector.deferred_capture(Callback::Function("live_function")));
    });
}

fn inert(collector: &Collector<'_, '_, '_>, value: Value) -> bool {
    let ty: Type = serde_json::from_value(value).unwrap();
    collector.for_item_type(Some(&ty), 32, &mut 256)
}

#[test]
fn crossing_values_require_complete_reference_modes_and_targets() {
    with_collector(document(PAIRS), |collector| {
        for mode in ["shared", "exclusive"] {
            assert!(inert(
                collector,
                json!({
                    "id":"test","kind":"reference","spelling":"test",
                    "mode":mode,"target":"type:usize"
                })
            ));
        }
        for (mode, target) in [
            (json!(null), json!("type:usize")),
            (json!("raw"), json!("type:usize")),
            (json!("shared"), json!(null)),
            (json!("shared"), json!("absent")),
        ] {
            assert!(!inert(
                collector,
                json!({
                    "id":"test","kind":"reference","spelling":"test",
                    "mode":mode,"target":target
                })
            ));
        }
        assert!(!collector.for_item_type(None, 32, &mut 256));
        let ty = collector.facts.type_of("type:usize");
        assert!(!collector.for_item_type(ty, 0, &mut 256));
        assert!(!collector.for_item_type(ty, 32, &mut 0));
    });
}

#[test]
fn inert_aggregate_cleanup_requires_known_bounded_contents() {
    with_collector(document(PAIRS), |collector| {
        for kind in [
            "bool", "char", "integer", "float", "string", "unit", "bottom",
        ] {
            assert!(inert(
                collector,
                json!({"id":"test","kind":kind,"spelling":"test"})
            ));
        }
        for contents in [
            json!({"kind":"tuple","items":["type:usize"]}),
            json!({"kind":"sequence","fixed":true,"element":"type:usize"}),
        ] {
            let mut value = contents;
            value["id"] = json!("test");
            value["spelling"] = json!("test");
            assert!(inert(collector, value));
        }
        for contents in [
            json!({"kind":"tuple"}),
            json!({"kind":"tuple","items":["absent"]}),
            json!({"kind":"sequence","fixed":false,"element":"type:usize"}),
            json!({"kind":"sequence","fixed":true}),
            json!({"kind":"opaque"}),
            json!({"kind":"integer","definition":"absent"}),
            json!({"kind":"integer","arguments":["type:usize"]}),
        ] {
            let mut value = contents;
            value["id"] = json!("test");
            value["spelling"] = json!("test");
            assert!(!inert(collector, value));
        }
    });
}

#[test]
fn downstream_owned_environments_fail_while_owned_results_remain_supported() {
    assert!(prepared(document(CLASSES)));
    for ty in ["absent", "type:std::option::Option<usize>"] {
        let mut changed = document(PAIRS);
        node(&mut changed, Site::Callback)["captures"] = json!([{
            "symbol":"sff0e90ca0.2","mode":"value","path":[],"type":ty
        }]);
        assert!(!prepared(changed), "{ty}");
    }
    let mut missing = document(PAIRS);
    node(&mut missing, Site::Callback)["captures"] = json!(null);
    assert!(!prepared(missing));
    let value = document(PAIRS);
    let callback = id(&value, Site::Callback);
    let init = id(&value, Site::Initializer);
    with_frame(value, |collector| {
        let mut chain = collector
            .chain(collector.facts.body.node(&init).unwrap())
            .unwrap()
            .unwrap();
        chain
            .stages
            .push(Stage::Map(Callback::Function("live_function")));
        assert!(collector.deferred_environments(&chain));
        chain.stages.push(Stage::FlatMap(
            collector.facts.body.node(&callback).unwrap(),
        ));
        assert!(!collector.deferred_environments(&chain));
    });
}

#[test]
fn resumed_pipeline_still_checks_standard_dispatch_and_supported_environments() {
    let mut flattened = document(PAIRS);
    definition(&mut flattened, Site::Adapter)["operation"] = json!("rust:Iterator.flat_map");
    assert!(!prepared(flattened));
    let mut dispatched = document(PAIRS);
    definition(&mut dispatched, Site::Adapter)["operation"] = json!("rust:Iterator.cloned");
    definition(&mut dispatched, Site::Adapter)["callable"]["parameters"] = json!([]);
    node(&mut dispatched, Site::Adapter)["children"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    reference(&mut dispatched, Site::Adapter)["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    reference(&mut dispatched, Site::Adapter)["instance_arguments"][1] = json!("absent");
    assert!(!prepared(dispatched));
}

fn continuation_valid(value: Value) -> bool {
    let tail = id(&value, Site::Tail);
    let used = id(&value, Site::Use);
    with_collector(value, |collector| {
        collector
            .deferred_continuation(
                collector.facts.body.node(&tail).unwrap(),
                collector.facts.body.node(&used).unwrap(),
            )
            .unwrap()
    })
}

#[test]
fn continuation_requires_a_consumer_and_certified_crossing_item() {
    for (site, field, value) in [
        (Site::Tail, "status", json!("dependent")),
        (Site::Adapter, "status", json!("dependent")),
        (Site::Adapter, "item", json!(null)),
        (Site::Adapter, "item", json!("absent")),
    ] {
        let mut changed = document(PAIRS);
        reference(&mut changed, site)[field] = value;
        assert!(!continuation_valid(changed), "{site:?}: {field}");
    }
    for site in [Site::Tail, Site::Adapter] {
        let mut changed = document(PAIRS);
        definition(&mut changed, site)["operation"] = json!("rust:Iterator.unknown");
        assert!(!continuation_valid(changed));
    }
    let mut value = document(PAIRS);
    let tail = id(&value, Site::Tail);
    let used = id(&value, Site::Use);
    node(&mut value, Site::Tail)["children"] = json!([]);
    with_collector(value, |collector| {
        assert!(
            !collector
                .deferred_continuation(
                    collector.facts.body.node(&tail).unwrap(),
                    collector.facts.body.node(&used).unwrap(),
                )
                .unwrap()
        );
    });
}

#[test]
fn known_continuation_contracts_still_require_complete_call_signatures() {
    for site in [Site::Tail, Site::Adapter] {
        let mut value = document(PAIRS);
        let tail = id(&value, Site::Tail);
        let used = id(&value, Site::Use);
        let adapter = id(&value, Site::Adapter);
        reference(&mut value, site)["signature"] = json!(null);
        with_collector(value, |collector| {
            let tail = collector.facts.body.node(&tail).unwrap();
            let adapter = collector.facts.body.node(&adapter).unwrap();
            let children = collector.facts.body.children(tail).unwrap();
            assert!(collector.consumer(tail, &children[1..]).unwrap().is_some());
            assert!(
                collector
                    .iterator_method(adapter)
                    .and_then(|key| Contracts::embedded().stage(key))
                    .is_some()
            );
            assert_eq!(collector.deferred_call(tail), !matches!(site, Site::Tail));
            assert_eq!(
                collector.deferred_call(adapter),
                !matches!(site, Site::Adapter)
            );
            assert!(
                !collector
                    .deferred_continuation(tail, collector.facts.body.node(&used).unwrap(),)
                    .unwrap()
            );
        });
    }
}

#[test]
fn a_complete_consumer_call_still_requires_proven_standard_dispatch() {
    let mut value = document(PAIRS);
    let tail = id(&value, Site::Tail);
    let used = id(&value, Site::Use);
    reference(&mut value, Site::Tail)["instance_arguments"][1] = json!("absent");
    with_collector(value, |collector| {
        let tail = collector.facts.body.node(&tail).unwrap();
        let children = collector.facts.body.children(tail).unwrap();
        let (_, role) = collector.consumer(tail, &children[1..]).unwrap().unwrap();
        assert!(collector.deferred_call(tail));
        assert!(
            !collector
                .standard_dispatch(tail, children[0], &role.dispatch)
                .unwrap()
        );
        assert!(
            !collector
                .deferred_continuation(tail, collector.facts.body.node(&used).unwrap(),)
                .unwrap()
        );
    });
}

fn consumer_callback(document: &mut Value, operation: &str, capture: Value) {
    let mut callback = node(document, Site::Callback).clone();
    callback["id"] = json!("test:consumer-callback");
    callback["children"] = json!([]);
    callback["captures"] = capture;
    let ty = callback["type"].clone();
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(callback);
    node(document, Site::Tail)["children"]
        .as_array_mut()
        .unwrap()
        .push(json!("test:consumer-callback"));
    reference(document, Site::Tail)["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(ty);
    let definition = definition(document, Site::Tail);
    definition["operation"] = json!(operation);
    definition["callable"]["parameters"] = json!(["type:usize"]);
}

#[test]
fn each_and_search_consumers_require_safe_callback_cleanup() {
    for operation in ["rust:Iterator.for_each", "rust:Iterator.any"] {
        let mut borrowed = document(PAIRS);
        consumer_callback(&mut borrowed, operation, json!([]));
        assert!(continuation_valid(borrowed));
        for captures in [
            json!(null),
            json!([{"symbol":"sff0e90ca0.2","mode":"value","type":"absent","path":[]}]),
            json!([{"symbol":"sff0e90ca0.2","mode":"mystery","type":"type:usize","path":[]}]),
        ] {
            let mut owned = document(PAIRS);
            consumer_callback(&mut owned, operation, captures);
            assert!(!continuation_valid(owned));
        }
    }
}

#[test]
fn callback_cleanup_accounts_for_capture_modes_and_work_limits() {
    let mut value = document(PAIRS);
    node(&mut value, Site::Callback)["captures"] = json!([
        {"symbol":"sff0e90ca0.2","mode":"value","type":"type:usize","path":[]},
        {"symbol":"sff0e90ca0.2","mode":"exclusive","type":"absent","path":[]},
        {"symbol":"sff0e90ca0.2","mode":"unique","type":"absent","path":[]},
    ]);
    let callback = id(&value, Site::Callback);
    with_collector(value, |collector| {
        let callback = Callback::Closure(collector.facts.body.node(&callback).unwrap());
        assert!(collector.for_callback_cleanup(callback, &mut 256));
        assert!(!collector.for_callback_cleanup(callback, &mut 0));
    });
}

#[test]
fn local_initialization_only_uses_the_recorded_binding() {
    let value = document(PAIRS);
    let pattern = id(&value, Site::Pattern);
    let init = id(&value, Site::Initializer);
    let used = id(&value, Site::Use);
    with_frame(value, |collector| {
        let parts = Local {
            pattern: collector.facts.body.node(&pattern).unwrap(),
            init: Some(collector.facts.body.node(&init).unwrap()),
            otherwise: None,
        };
        assert_eq!(collector.initialize_deferred(&parts, 991).unwrap(), None);
        collector
            .prepare_deferred(collector.facts.body.root().unwrap())
            .unwrap();
        assert!(
            collector
                .initialize_deferred(&parts, 991)
                .unwrap()
                .is_some()
        );
        collector.deferred.as_mut().unwrap().symbol = "different";
        assert_eq!(collector.initialize_deferred(&parts, 991).unwrap(), None);
        collector.deferred = Some(crate::cpg::bounds::deferred::Binding {
            symbol: collector.binding_symbol(parts.pattern).unwrap(),
            init: parts.pattern,
            use_node: collector.facts.body.node(&used).unwrap(),
        });
        assert_eq!(collector.initialize_deferred(&parts, 991).unwrap(), None);
        assert!(
            collector
                .chain_source(collector.facts.body.node(&used).unwrap())
                .unwrap()
                .is_none()
        );
    });
}

fn extend_continuation(value: &mut Value, count: usize) -> String {
    let mut receiver = id(value, Site::Use);
    let adapter = id(value, Site::Adapter);
    let prototype = node(value, Site::Initializer).clone();
    let mut prototype_ref = reference(value, Site::Initializer).clone();
    prototype_ref["signature"]["parameters"][0] = prototype["type"].clone();
    for index in 0..count {
        let id = format!("test:stage:{index}");
        let mut node = prototype.clone();
        node["id"] = json!(id);
        node["children"] = json!([receiver]);
        let mut reference = prototype_ref.clone();
        reference["node"] = json!(id);
        function(value)["nodes"].as_array_mut().unwrap().push(node);
        function(value)["refs"]
            .as_array_mut()
            .unwrap()
            .push(reference);
        receiver = id;
    }
    item(&mut function(value)["nodes"], "id", &adapter)["children"] = json!([]);
    node(value, Site::Tail)["children"] = json!([receiver]);
    reference(value, Site::Tail)["signature"]["parameters"][0] = prototype["type"].clone();
    receiver
}

#[test]
fn continuation_walk_has_a_fixed_work_bound() {
    let mut value = document(PAIRS);
    let used = id(&value, Site::Use);
    let tail = id(&value, Site::Tail);
    extend_continuation(&mut value, MAX_STAGES + 1);
    with_collector(value, |collector| {
        let tail = collector.facts.body.node(&tail).unwrap();
        let children = collector.facts.body.children(tail).unwrap();
        let (_, role) = collector.consumer(tail, &children[1..]).unwrap().unwrap();
        assert!(
            collector
                .standard_dispatch(tail, children[0], &role.dispatch)
                .unwrap()
        );
        let mut current = tail;
        for _ in 0..MAX_STAGES + 2 {
            assert!(collector.deferred_call(current), "{}", current.id);
            assert!(collector.deferred_item(current, &mut 256));
            current = collector.facts.body.child(current, 0).unwrap();
        }
        assert_eq!(current.id, used);
        assert!(
            !collector
                .deferred_continuation(tail, collector.facts.body.node(&used).unwrap(),)
                .unwrap()
        );
    });
}

#[test]
fn combined_pipeline_depth_is_bounded_across_the_local_boundary() {
    for count in [MAX_STAGES - 1, MAX_STAGES] {
        let mut value = document(PAIRS);
        let init = id(&value, Site::Initializer);
        let used = id(&value, Site::Use);
        let pattern = id(&value, Site::Pattern);
        let outer = extend_continuation(&mut value, count);
        with_frame(value, |collector| {
            collector.deferred = Some(crate::cpg::bounds::deferred::Binding {
                symbol: collector
                    .binding_symbol(collector.facts.body.node(&pattern).unwrap())
                    .unwrap(),
                init: collector.facts.body.node(&init).unwrap(),
                use_node: collector.facts.body.node(&used).unwrap(),
            });
            let chain = collector
                .chain(collector.facts.body.node(&outer).unwrap())
                .unwrap();
            assert_eq!(chain.is_some(), count < MAX_STAGES);
            if let Some(chain) = chain {
                assert_eq!(chain.stages.len(), MAX_STAGES);
                assert!(chain.initialized);
            }
        });
    }
}

#[test]
fn resuming_stored_state_discards_only_already_evaluated_inputs() {
    let mut value = document(PAIRS);
    let used = id(&value, Site::Use);
    let init = id(&value, Site::Initializer);
    let pattern = id(&value, Site::Pattern);
    definition(&mut value, Site::Initializer)["operation"] = json!("rust:Iterator.skip");
    function(&mut value)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"test:count","kind":"literal","original_kind":"literal","children":[],
            "type":"type:usize","provenance":{"origin":"source"}
        }));
    node(&mut value, Site::Initializer)["children"]
        .as_array_mut()
        .unwrap()
        .push(json!("test:count"));
    with_frame(value, |collector| {
        let init = collector.facts.body.node(&init).unwrap();
        assert_eq!(collector.chain(init).unwrap().unwrap().arguments.len(), 1);
        collector.deferred = Some(crate::cpg::bounds::deferred::Binding {
            symbol: collector
                .binding_symbol(collector.facts.body.node(&pattern).unwrap())
                .unwrap(),
            init,
            use_node: collector.facts.body.node(&used).unwrap(),
        });
        let resumed = collector
            .chain_source(collector.facts.body.node(&used).unwrap())
            .unwrap()
            .unwrap();
        assert!(resumed.initialized && resumed.arguments.is_empty());
        assert_eq!(resumed.stages.len(), 1);
    });
}

fn input_marker(document: &mut Value, name: &str) {
    let id = format!("test:{name}");
    let callee = format!("{id}:callee");
    let ty = format!("{id}:type");
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":id,"kind":"call","original_kind":"call","children":[callee],
            "type":"type:usize","provenance":{"origin":"source"}
        }));
    function(document)["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":callee,"kind":"identifier","original_kind":"identifier","children":[],
            "type":ty,"provenance":{"origin":"source"}
        }));
    document["unit"]["types"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":ty,"kind":"function","spelling":name,"definition":id,
            "parameters":[],"result":"type:usize"
        }));
    function(document)["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node":id,"status":"resolved","dispatch":"static",
            "target":{"kind":"definition","id":id},
            "calls_back":{"value":false,"reason":"direct leaf"}
        }));
    function(document)["refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "node":callee,"status":"resolved","target":{"kind":"definition","id":id}
        }));
    document["unit"]["definitions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":id,"name":name,"stable_identity":name,"kind":"function","scope":"unit",
            "callable":{"parameters":[],"result":"type:usize","generics":0}
        }));
}

fn input_calls(collector: &Collector<'_, '_, '_>, mut entry: u32) -> Vec<String> {
    let mut calls = Vec::new();
    while entry != 991 {
        let node = &collector.program.nodes[entry as usize];
        if let Action::Call { callee, target, .. } = &node.action {
            assert!(matches!(target, crate::runtime_bounds::Callee::Resolved(_)));
            calls.push(callee.clone());
        } else {
            panic!("unexpected input action: {:?}", node.action);
        }
        let Flow::Goto(next) = node.flow else {
            panic!("input evaluation must continue to its scan");
        };
        entry = next;
        assert!(calls.len() <= 3);
    }
    calls
}

#[test]
fn initialized_inputs_skip_the_source_but_still_evaluate_new_arguments() {
    let mut value = document(PAIRS);
    input_marker(&mut value, "source");
    input_marker(&mut value, "argument");
    node(&mut value, Site::Source)["children"] = json!(["test:source"]);
    let source = id(&value, Site::Source);
    with_collector(value, |collector| {
        let mut chain = Chain {
            root: collector.facts.body.node(&source).unwrap(),
            limit: crate::runtime_bounds::Value::Const(7),
            bits: 64,
            stages: Vec::new(),
            arguments: vec![collector.facts.body.node("test:argument").unwrap()],
            standard: true,
            initialized: false,
        };
        let first = collector.chain_inputs(&chain, 991).unwrap();
        assert_eq!(input_calls(collector, first), ["source", "argument"]);
        chain.initialized = true;
        let resumed = collector.chain_inputs(&chain, 991).unwrap();
        assert_eq!(input_calls(collector, resumed), ["argument"]);
        chain.initialized = false;
        chain.root = collector.facts.body.node("test:source").unwrap();
        let direct = collector.chain_inputs(&chain, 991).unwrap();
        assert_eq!(input_calls(collector, direct), ["source", "argument"]);
    });
}

fn callback_plan(value: &mut Value, site: Site) -> Value {
    let callback = node(value, site);
    if callback["kind"] == "closure" {
        json!({"kind":"closure","id":callback["id"]})
    } else {
        assert_eq!(callback["kind"], "identifier");
        let target = reference(value, site)["target"].clone();
        assert_eq!(target["kind"], "definition");
        json!({"kind":"definition","id":target["id"]})
    }
}

fn install_named_plans(value: &mut Value) {
    let mut runs = Vec::new();
    if node(value, Site::Initializer)["children"]
        .as_array()
        .unwrap()
        .len()
        == 2
    {
        runs.push(callback_plan(value, Site::InitialCallback));
        reference(value, Site::Initializer)["calls_back"]["runs"] = json!(runs);
    }
    runs.push(callback_plan(value, Site::Callback));
    reference(value, Site::Adapter)["calls_back"]["runs"] = json!(runs);
    if node(value, Site::Tail)["children"]
        .as_array()
        .unwrap()
        .len()
        == 2
    {
        runs.push(callback_plan(value, Site::ConsumerCallback));
    }
    reference(value, Site::Tail)["calls_back"]["runs"] = json!(runs);
}

fn named_call_valid(value: Value, site: Site) -> bool {
    let call = id(&value, site);
    with_frame(value, |collector| {
        let root = collector.facts.body.root().unwrap();
        let local = collector.facts.body.child(root, 0).unwrap();
        let tail = collector.facts.body.child(root, 1).unwrap();
        collector.deferred = binding(collector.facts, local, tail).unwrap();
        assert!(collector.deferred.is_some());
        collector.deferred_call(collector.facts.body.node(&call).unwrap())
    })
}

#[test]
fn production_bindings_accept_exact_live_callback_plans() {
    for source in [PAIRS, PRODUCERS, CONSUMERS, CLASSES] {
        let mut value = document(source);
        install_named_plans(&mut value);
        assert!(prepared(value.clone()));
        for site in [Site::Initializer, Site::Adapter, Site::Tail] {
            assert!(named_call_valid(value.clone(), site), "{site:?}");
        }
    }
}

#[test]
fn named_callback_plans_match_kinds_ids_and_complete_cumulative_sets() {
    let mut baseline = document(PRODUCERS);
    install_named_plans(&mut baseline);
    for site in [Site::Initializer, Site::Adapter, Site::Tail] {
        let expected = reference(&mut baseline, site)["calls_back"]["runs"]
            .as_array()
            .unwrap()
            .clone();
        for runs in [
            Vec::new(),
            vec![json!({"kind":"closure","id":"absent"})],
            [expected.clone(), vec![expected[0].clone()]].concat(),
            [
                expected.clone(),
                vec![json!({"kind":"definition","id":"absent"})],
            ]
            .concat(),
            vec![expected[0].clone(); MAX_STAGES + 2],
        ] {
            let mut changed = baseline.clone();
            reference(&mut changed, site)["calls_back"]["runs"] = json!(runs);
            assert!(
                !named_call_valid(changed.clone(), site),
                "{site:?}: {runs:?}"
            );
            assert!(!prepared(changed));
        }
        let mut wrong_kind = baseline.clone();
        reference(&mut wrong_kind, site)["calls_back"]["runs"][0]["kind"] = json!("definition");
        assert!(!named_call_valid(wrong_kind, site));
        let mut false_plan = baseline.clone();
        reference(&mut false_plan, site)["calls_back"]["value"] = json!(false);
        assert!(!named_call_valid(false_plan, site));
    }
}

#[test]
fn downstream_plans_include_initializer_callbacks_and_ignore_roster_order() {
    for site in [Site::Adapter, Site::Tail] {
        let mut missing = document(PRODUCERS);
        install_named_plans(&mut missing);
        reference(&mut missing, site)["calls_back"]["runs"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert!(!named_call_valid(missing, site));
        let mut reversed = document(PRODUCERS);
        install_named_plans(&mut reversed);
        reference(&mut reversed, site)["calls_back"]["runs"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(named_call_valid(reversed.clone(), site));
        assert!(prepared(reversed));
    }
}

#[test]
fn named_function_callback_plans_preserve_the_resolved_definition_identity() {
    let mut baseline = document(CLASSES);
    install_named_plans(&mut baseline);
    let function = callback_plan(&mut baseline, Site::Callback);
    assert_eq!(function["kind"], "definition");
    for site in [Site::Adapter, Site::Tail] {
        let mut wrong_kind = baseline.clone();
        reference(&mut wrong_kind, site)["calls_back"]["runs"][1]["kind"] = json!("closure");
        assert!(!named_call_valid(wrong_kind, site));
        let mut wrong_id = baseline.clone();
        reference(&mut wrong_id, site)["calls_back"]["runs"][1]["id"] = json!("absent");
        assert!(!named_call_valid(wrong_id, site));
    }
}

#[test]
fn consumer_named_plans_include_the_pipeline_and_the_consumer_callback() {
    for operation in ["rust:Iterator.for_each", "rust:Iterator.any"] {
        let mut baseline = document(PRODUCERS);
        consumer_callback(&mut baseline, operation, json!([]));
        install_named_plans(&mut baseline);
        assert!(named_call_valid(baseline.clone(), Site::Tail));
        assert!(prepared(baseline.clone()));
        for removed in [0, 1, 2] {
            let mut missing = baseline.clone();
            reference(&mut missing, Site::Tail)["calls_back"]["runs"]
                .as_array_mut()
                .unwrap()
                .remove(removed);
            assert!(!named_call_valid(missing.clone(), Site::Tail));
            assert!(!prepared(missing));
        }
    }
}

#[test]
fn named_plans_require_a_resolvable_supported_pipeline() {
    for operation in ["rust:Iterator.unknown", "rust:Iterator.flat_map"] {
        let mut value = document(PRODUCERS);
        install_named_plans(&mut value);
        definition(&mut value, Site::Adapter)["operation"] = json!(operation);
        assert!(!named_call_valid(value.clone(), Site::Adapter));
        assert!(!prepared(value));
    }
    let mut nonstandard = document(PAIRS);
    install_named_plans(&mut nonstandard);
    definition(&mut nonstandard, Site::Adapter)["operation"] = json!("rust:Iterator.cloned");
    definition(&mut nonstandard, Site::Adapter)["callable"]["parameters"] = json!([]);
    node(&mut nonstandard, Site::Adapter)["children"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    reference(&mut nonstandard, Site::Adapter)["signature"]["parameters"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    reference(&mut nonstandard, Site::Adapter)["instance_arguments"][1] = json!("absent");
    assert!(!named_call_valid(nonstandard.clone(), Site::Adapter));
    assert!(!prepared(nonstandard));
}
