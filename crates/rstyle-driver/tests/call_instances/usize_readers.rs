//! Scoped compiler observations do not activate the complete widening reader.
use super::*;
use rstyle_pdg::{HirNode, HirRef, id_error_maps::capture::Environment};

const NAME: &str = "try_read_u32_as_usize";
const SCOPE: &str =
    "immediate-core-result-reader-callback-owning-body; nested owners separately scoped";
const ENVIRONMENT: &str = "compiler_usize_reader_callback_environment";

fn source() -> String {
    let selected: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/function-families/wire-usize-readers/selection.json"
    ))
    .unwrap();
    let original = selected["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "util::wire::try_read_u32_as_usize")
        .unwrap()["source"]
        .as_str()
        .unwrap();

    // Small diagnostic dependencies; the complete selected caller stays byte-exact.
    format!(
        "struct DeserializeError {{ what: &'static str }}\n\
         impl DeserializeError {{\n\
           fn invalid_usize(what: &'static str) -> Self {{ Self {{ what }} }}\n\
         }}\n\
         fn try_read_u32(slice: &[u8], what: &'static str)\n\
           -> Result<(u32, usize), DeserializeError> {{\n\
           if slice.len() < 4 {{ return Err(DeserializeError {{ what }}); }}\n\
           Ok((u32::from_ne_bytes([slice[0],slice[1],slice[2],slice[3]]),4))\n\
         }}\n{original}"
    )
}

#[test]
fn widening_callbacks_retain_real_environments_and_separate_nested_owners() {
    let source = source();

    for prelude in ["", "#![no_std]\n"] {
        capture_context(&format!("{prelude}{source}"));
    }
}

fn capture_context(source: &str) {
    let functions = captured_source("hir-data-v3", source);
    let facts = function(&functions, NAME);
    let nodes = closures(facts);
    assert_eq!(nodes.len(), 3);
    let [outer, map, error_map] = nodes.as_slice() else {
        unreachable!()
    };
    tuple_parameter(outer);
    environments(facts, [outer, map, error_map]);
    scoped_calls(facts, [outer, map, error_map]);
    assert!(rstyle_pdg::producer::produce(facts).is_err());

    let historical = captured_source("hir-data-v2", source);
    let facts = function(&historical, NAME);
    assert!(facts.build.frontend_options.get(KEY).is_none());
    let historical_closures = closures(facts);
    assert_eq!(historical_closures.len(), 1);
    assert!(historical_closures[0].children.is_empty());
    assert!(historical_closures[0].res.is_none());
}

#[test]
fn nested_explicit_return_keeps_widening_callbacks_unexpanded() {
    let source = source().replace("map(|n| (n, nr))", "map(|n| { return (n, nr); })");
    let functions = captured_source("hir-data-v3", &source);
    let facts = function(&functions, NAME);
    let nodes = closures(facts);
    assert_eq!(nodes.len(), 1);
    assert!(nodes[0].children.is_empty());
    assert!(facts.build.frontend_options[KEY].get("callbacks").is_none());

    assert!(rstyle_pdg::producer::produce(facts).is_err());
}

fn closures(facts: &HirFnFacts) -> Vec<&HirNode> {
    let mut nodes = Vec::new();
    rstyle_pdg::frontend::collect_kind(&facts.body, "expr_closure", &mut nodes);

    nodes
}

fn tuple_parameter(outer: &HirNode) {
    let parameter = &outer.children[0];
    assert_eq!(parameter.kind, "pat_scalar_tuple");
    assert_eq!(parameter.children.len(), 2);

    for (binding, name) in parameter.children.iter().zip(["n", "nr"]) {
        assert_eq!(binding.kind, "pat_binding");
        assert_eq!(binding.snippet, name);
        assert_eq!(binding.binding_mut, Some(false));
        assert!(binding.param_index.is_none());
    }
}

fn environments(facts: &HirFnFacts, nodes: [&HirNode; 3]) {
    let nr = &nodes[0].children[0].children[1].hir;
    let what = &facts.signature.params[1].pat.hir;

    for (node, root) in nodes.into_iter().zip([what, nr, what]) {
        let environment = environment(node);
        assert_eq!(&environment.captures[0].root, root);
        assert_eq!(
            environment.captures[0].mode,
            rstyle_pdg::option_callbacks::CaptureMode::Shared
        );
        assert!(environment.captures[0].projections.is_empty());
        assert_eq!(environment.signature_abi, "RustCall");
        assert_eq!(environment.signature_inputs.len(), 1);
    }

    assert_eq!(nodes[1].children[0].kind, "pat_binding");
    assert_eq!(nodes[2].children[0].kind, "pat_wild");
}

fn environment(node: &HirNode) -> Environment {
    let resolution = node.res.as_ref().unwrap();
    assert_eq!(resolution.kind, ENVIRONMENT);
    assert!(resolution.target.is_none());
    let environment: Environment = serde_json::from_str(&resolution.description).unwrap();
    assert_eq!(
        environment.identity,
        "rstyle.rust.usize-reader-callback-environment/1"
    );
    assert_eq!(environment.closure, node.hir);
    assert_eq!(environment.body, node.children.last().unwrap().hir);
    assert_eq!(environment.captures.len(), 1);

    environment
}

fn scoped_calls(facts: &HirFnFacts, nodes: [&HirNode; 3]) {
    let inventory = &facts.build.frontend_options[KEY];
    let roots = inventory["callbacks"].as_array().unwrap();
    assert_eq!(roots.len(), 1);
    let outer = &roots[0];
    scope(outer, nodes[0], None);
    let nested = outer["callbacks"].as_array().unwrap();
    assert_eq!(nested.len(), 2);

    for node in &nodes[1..] {
        let record = nested
            .iter()
            .find(|record| record["hir"] == serde_json::to_value(&node.hir).unwrap())
            .unwrap();
        scope(record, node, Some(&nodes[0].hir));
        assert!(record.get("callbacks").is_none());
        assert!(record["calls"].as_array().unwrap().is_empty());
    }

    conversion_site(inventory, outer, nodes[0]);
}

fn scope(record: &Value, node: &HirNode, parent: Option<&HirRef>) {
    assert_eq!(record["scope"], SCOPE);
    assert_eq!(record["complete"], true);
    assert_eq!(record["hir"], serde_json::to_value(&node.hir).unwrap());
    assert_eq!(record["span"], serde_json::to_value(node.span).unwrap());
    assert_eq!(
        record["body"],
        serde_json::to_value(&environment(node).body).unwrap()
    );
    assert_eq!(
        record["definition"]["definition_identity"],
        environment(node).definition_identity
    );
    assert_eq!(record["capture_count"], 1);

    if let Some(parent) = parent {
        assert_eq!(
            record["parent_callback"],
            serde_json::to_value(parent).unwrap()
        );
    } else {
        assert!(record.get("parent_callback").is_none());
    }
}

fn conversion_site(inventory: &Value, outer: &Value, node: &HirNode) {
    let mut calls = Vec::new();
    rstyle_pdg::frontend::collect_kind(node, "expr_call", &mut calls);
    let conversion = calls
        .iter()
        .find(|call| call.snippet == "usize::try_from(n)")
        .unwrap();
    let id = serde_json::to_value(&conversion.hir).unwrap();
    let matches = |call: &&Value| call["hir"] == id;
    let direct = outer["calls"].as_array().unwrap();
    assert_eq!(direct.iter().filter(matches).count(), 1);
    assert!(
        !inventory["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|call| call["hir"] == id)
    );
    let selected = direct.iter().find(matches).unwrap();
    assert_eq!(selected["status"], "resolved");
    assert_eq!(selected["signature"]["inputs"], serde_json::json!(["u32"]));
    assert!(
        selected["selected"]["path"]
            .as_str()
            .unwrap()
            .ends_with("::try_from")
    );
}
