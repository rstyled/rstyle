//! Hand-authored topology adversaries, independent of producer replay.
use super::*;
use serde_json::{Value, json};

const IDS: [&str; 11] = [
    "saved", "selector", "payload", "binding", "read", "wildcard", "ok", "error", "err", "merge",
    "returned",
];

fn scaffold() -> Pair {
    let pdg: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor-compat/pdg.example.json"
    )))
    .unwrap();
    Pair {
        pdg: serde_json::from_value(pdg).unwrap(),
        cfg: serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vendor-compat/cfg.example.json"
        )))
        .unwrap(),
    }
}

pub(super) fn source() -> HirNode {
    let facts: crate::HirFnFacts =
        serde_json::from_str(include_str!("../../copy_tagged_values/tests/1.json")).unwrap();
    facts.body.children[0].clone()
}

pub(super) fn fixture() -> Pair {
    let mut pair = scaffold();
    nodes(&mut pair);
    data(&mut pair);
    control(&mut pair);
    sites(&mut pair);
    pair
}

fn nodes(pair: &mut Pair) {
    let template = pair.pdg.nodes[0].clone();
    let source = source();
    pair.pdg.nodes = IDS
        .into_iter()
        .map(|id| {
            let mut node = template.clone();
            node.id = id.into();
            node.attributes = serde_json::from_value(json!({})).unwrap();
            node.category = match id {
                "selector" | "wildcard" => NodeCategory::Control,
                "merge" => NodeCategory::Merge,
                "returned" => NodeCategory::Return,
                "binding" => NodeCategory::Declaration,
                "read" | "saved" => NodeCategory::Read,
                _ => NodeCategory::Computation,
            };
            node.source_span.as_mut().unwrap().start = u64::from(source.span.start);
            node.source_span.as_mut().unwrap().end = u64::from(source.span.end);
            node.annotations = BTreeMap::from([(
                "compat.rust.hir-r0:type".into(),
                json!({"value":"fixture-type"}),
            )]);
            if id == "returned" {
                node.attributes
                    .extensions
                    .insert("compat.rust.hir-r0:return-form".into(), json!("implicit"));
            }
            node
        })
        .collect();
}

fn data(pair: &mut Pair) {
    let template = pair.pdg.edges[0].clone();
    pair.pdg.edges = [
        ("saved", "selector", "value_operand"),
        ("saved", "payload", "value_operand"),
        ("payload", "binding", "value_operand"),
        ("binding", "read", "reaching_definition"),
        ("read", "ok", "value_operand"),
        ("error", "err", "value_operand"),
        ("ok", "merge", "value_alternative"),
        ("err", "merge", "value_alternative"),
        ("merge", "returned", "value_operand"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (from, to, kind))| {
        let mut edge = template.clone();
        edge.id = format!("data:{index}");
        edge.source = from.into();
        edge.target = to.into();
        edge.subkind = kind.into();
        edge.kind = crate::EdgeFamily::Data;
        edge.operand_position = Some(0);
        edge
    })
    .collect();
}

fn control(pair: &mut Pair) {
    pair.cfg.entry_node_id = "entry".into();
    pair.cfg.exit_node_ids = vec!["exit".into()];
    pair.cfg.nodes = IDS
        .into_iter()
        .filter(|id| *id != "merge")
        .chain(["entry", "exit"])
        .map(|id| {
            serde_json::from_value(json!({"id":id,
            "kind":if id=="entry" {"entry"} else if id=="exit" {"exit"} else {"block"},
            "origin":"independent Copy branch fixture", "operations":[],
            "exit_kind":if id=="exit" {json!("normal")} else {Value::Null}}))
            .unwrap()
        })
        .collect();
    pair.cfg.edges = [
        ("entry", "saved", "normal"),
        ("saved", "selector", "normal"),
        ("selector", "payload", "true"),
        ("payload", "binding", "normal"),
        ("binding", "read", "normal"),
        ("read", "ok", "normal"),
        ("ok", "returned", "normal"),
        ("selector", "wildcard", "false"),
        ("wildcard", "error", "normal"),
        ("error", "err", "normal"),
        ("err", "returned", "normal"),
        ("returned", "exit", "return"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (from, to, kind))| {
        serde_json::from_value(
            json!({"id":format!("cfg:{index}"),"source":from,"target":to,"kind":kind}),
        )
        .unwrap()
    })
    .collect();
    pair.pdg.extensions.insert(
        crate::OPERATION_MAP_KEY.into(),
        json!(
            IDS.into_iter()
                .filter(|id| *id != "merge")
                .map(|id| json!({"cfg_node_id":id,
            "cfg_operation_id":format!("operation:{id}"),"pdg_node_id":id}))
                .collect::<Vec<_>>()
        ),
    );
}

fn sites(pair: &mut Pair) {
    pair.pdg.extensions.insert("compat.rust.hir-r0:value-sites".into(), json!([{
        "pdg_node_id":"merge", "controller_pdg_node_id":"selector", "cfg_successor_node_id":"returned",
        "rstyle.hir-coverage:selection-kind":"ordered-match/1", "alternatives":[
            {"outcome":"arm:0", "value_pdg_node_id":"ok", "completion_cfg_node_ids":["ok"]},
            {"outcome":"arm:1", "value_pdg_node_id":"err", "completion_cfg_node_ids":["err"]}]}]));
}

fn replay(pair: &Pair) -> Result<(), String> {
    let node = |id: &str| pair.pdg.nodes.iter().find(|node| node.id == id).unwrap();
    let ops = graph::Operations {
        saved: node("saved"),
        selector: node("selector"),
        payload: node("payload"),
        binding: node("binding"),
        read: node("read"),
        wildcard: node("wildcard"),
        ok: node("ok"),
        error: node("error"),
        err: node("err"),
        merge: node("merge"),
    };
    graph::verify(pair.as_view(), &ops)?;
    graph::returned(pair.as_view(), &source(), ops.merge)
}

#[test]
fn original_wildcard_topology_selects_complete_ok_or_exact_err_then_returns() {
    replay(&fixture()).unwrap();
}

#[test]
fn unreachable_or_bypassed_payload_binding_read_ok_wildcard_error_and_err_fail() {
    for detached in [
        "selector", "payload", "binding", "read", "ok", "wildcard", "error", "err", "returned",
    ] {
        let mut pair = fixture();
        for edge in &mut pair.cfg.edges {
            if edge.target == detached {
                edge.target = "exit".into();
            }
        }
        assert!(replay(&pair).is_err(), "detached {detached}");
        let mut pair = fixture();
        let mut bypass = pair.cfg.edges[0].clone();
        bypass.id = "bypass".into();
        bypass.target = detached.into();
        pair.cfg.edges.push(bypass);
        assert!(replay(&pair).is_err(), "bypassed {detached}");
    }
}

#[test]
fn same_typed_substitutions_and_extra_consumers_fail() {
    for original in [
        "saved", "payload", "binding", "read", "ok", "error", "err", "merge",
    ] {
        let mut pair = fixture();
        let edge = pair
            .pdg
            .edges
            .iter_mut()
            .find(|edge| edge.source == original)
            .unwrap();
        edge.target = "substitute".into();
        assert!(replay(&pair).is_err(), "substituted {original}");
        let mut pair = fixture();
        let mut extra = pair
            .pdg
            .edges
            .iter()
            .find(|edge| edge.source == original)
            .unwrap()
            .clone();
        extra.id = "extra".into();
        extra.target = "substitute".into();
        pair.pdg.edges.push(extra);
        assert!(replay(&pair).is_err(), "extra {original}");
    }
}

#[test]
fn altered_merge_controller_arm_value_completion_or_selection_kind_fails() {
    for (path, value) in [
        ("/0/controller_pdg_node_id", json!("wildcard")),
        ("/0/alternatives/0/outcome", json!("arm:1")),
        ("/0/alternatives/0/value_pdg_node_id", json!("err")),
        (
            "/0/alternatives/0/completion_cfg_node_ids",
            json!(["error"]),
        ),
        ("/0/rstyle.hir-coverage:selection-kind", json!("other")),
        ("/0/cfg_successor_node_id", json!("exit")),
    ] {
        let mut pair = fixture();
        *pair
            .pdg
            .extensions
            .get_mut("compat.rust.hir-r0:value-sites")
            .unwrap()
            .pointer_mut(path)
            .unwrap() = value;
        assert!(replay(&pair).is_err(), "{path}");
    }
}
