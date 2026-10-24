//! Hand-authored branch graphs exercise replay independently of producer output.
//! These topology fixtures do not claim nominal/source authority or certify a whole chain.
use super::*;
use crate::{HirNode, HirRef, concrete_types::TypeTree};
use serde_json::{Value, json};

const IDS: [&str; 8] = [
    "saved",
    "selector",
    "ok",
    "err",
    "ctor",
    "ok_result",
    "err_result",
    "merge",
];

fn scaffold() -> Pair {
    let mut pdg: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor-compat/pdg.example.json"
    )))
    .unwrap();
    for edge in pdg["edges"].as_array_mut().unwrap() {
        edge["policy"] = json!(crate::ProfileId::hir_r0());
    }
    let conformance = pdg["extraction_evidence"]["conformance"].take();
    pdg["extraction_evidence"]["conformance"] = json!([{
        "profile": crate::ProfileId::hir_r0(), "status": conformance["status"],
        "reasons": conformance["reasons"]}]);
    pdg["function"]["reference_count"]["policy"] = json!(crate::ProfileId::hir_r0());
    Pair {
        pdg: serde_json::from_value(pdg).unwrap(),
        cfg: serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vendor-compat/cfg.example.json"
        )))
        .unwrap(),
    }
}

fn source() -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local: 1,
        },
        kind: "expr_method_call".into(),
        span: crate::ByteSpan { start: 0, end: 1 },
        snippet: String::new(),
        ty: None,
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: vec![],
    }
}

fn checked(source: &HirNode, maps_error: bool) -> CheckedMap<'_> {
    let base = product("Index", "0", "u32", TypeTree::Scalar("u32".into()));
    let err = product(
        "IndexError",
        "attempted",
        "u64",
        TypeTree::Scalar("u64".into()),
    );
    let wrapped = product("Wrapper", "0", "Index", base.clone());
    let (ok, new_ok, new_err) = if maps_error {
        (
            wrapped.clone(),
            wrapped,
            product("Failure", "0", "IndexError", err.clone()),
        )
    } else {
        (base, wrapped, err.clone())
    };
    CheckedMap {
        source,
        receiver: source,
        callback: source,
        maps_error,
        input: TypeTree::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        },
        output: TypeTree::Result {
            ok: Box::new(new_ok),
            err: Box::new(new_err),
        },
    }
}

fn product(name: &str, field: &str, declaration: &str, ty: TypeTree) -> TypeTree {
    use crate::concrete_types::{Field, Nominal};
    let source: crate::SourceText = if field == "0" {
        format!("struct {name}({declaration});").into()
    } else {
        format!("struct {name} {{ {field}: {declaration} }}").into()
    };
    TypeTree::Struct {
        nominal: Nominal {
            source_identity: Some("replay-fixture-source".into()),
            package_name: "fixture".into(),
            package_version: "1.0.0".into(),
            manifest_digest: format!("sha256:{}", "0".repeat(64)),
            definition_path: format!("fixture::{name}"),
            declaration_digest: source.identity().into(),
        },
        source,
        fields: vec![Field {
            position: 0,
            name: field.into(),
            declaration: declaration.into(),
            ty,
        }],
    }
}

fn fixture(maps_error: bool) -> Pair {
    let mut pair = scaffold();
    let template = pair.pdg.nodes[0].clone();
    let metadata = metadata(&checked(&source(), maps_error)).unwrap();
    pair.pdg.nodes = IDS
        .into_iter()
        .map(|id| {
            let mut node = template.clone();
            node.id = id.into();
            node.attributes = serde_json::from_value(json!({})).unwrap();
            node.category = match id {
                "selector" => NodeCategory::Control,
                "merge" => NodeCategory::Merge,
                _ => NodeCategory::Computation,
            };
            if id == "selector" {
                node.attributes.outcomes = vec!["true".into(), "false".into()];
            }
            node.attributes.extensions.insert(
                "compat.rust.hir-r0:source-operation".into(),
                json!({"role": checked(&source(), maps_error).role()}),
            );
            node.annotations = BTreeMap::from([(KEY.into(), metadata.clone())]);
            node
        })
        .collect();
    data_edges(&mut pair, maps_error);
    control_edges(&mut pair, maps_error);
    value_site(&mut pair);
    pair
}

fn data_edges(pair: &mut Pair, maps_error: bool) {
    let template = pair.pdg.edges[0].clone();
    let selected = if maps_error { "err" } else { "ok" };
    let selected_result = if maps_error {
        "err_result"
    } else {
        "ok_result"
    };
    let unchanged = if maps_error {
        ("ok", "ok_result")
    } else {
        ("err", "err_result")
    };
    pair.pdg.edges = [
        ("saved", "selector"),
        ("saved", "ok"),
        ("saved", "err"),
        (selected, "ctor"),
        ("ctor", selected_result),
        unchanged,
        ("ok_result", "merge"),
        ("err_result", "merge"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (source, target))| {
        let mut edge = template.clone();
        edge.id = format!("data:{index}");
        edge.source = source.into();
        edge.target = target.into();
        edge.kind = crate::EdgeFamily::Data;
        edge.subkind = if target == "merge" {
            "value_alternative"
        } else {
            "value_operand"
        }
        .into();
        edge.operand_position = Some(0);
        edge
    })
    .collect();
}

fn control_edges(pair: &mut Pair, maps_error: bool) {
    let selected = if maps_error {
        ("err", "err_result")
    } else {
        ("ok", "ok_result")
    };
    let unchanged = if maps_error {
        ("ok", "ok_result")
    } else {
        ("err", "err_result")
    };
    pair.cfg.entry_node_id = "entry".into();
    pair.cfg.exit_node_ids = vec!["exit".into()];
    pair.cfg.edges = [
        ("entry", "saved", "normal"),
        ("saved", "selector", "normal"),
        ("selector", "ok", "false"),
        ("selector", "err", "true"),
        (selected.0, "ctor", "normal"),
        ("ctor", selected.1, "normal"),
        (unchanged.0, unchanged.1, "normal"),
        ("ok_result", "done", "normal"),
        ("err_result", "done", "normal"),
        ("done", "exit", "return"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (source, target, kind))| {
        serde_json::from_value(json!({"id": format!("cfg:{index}"),
                "source": source, "target": target, "kind": kind}))
        .unwrap()
    })
    .collect();
    cfg_operations(pair);
}

fn cfg_operations(pair: &mut Pair) {
    let mapping: Vec<_> = IDS[..7]
        .iter()
        .map(|id| {
            json!({
        "cfg_node_id": id, "cfg_operation_id": format!("operation:{id}"), "pdg_node_id": id})
        })
        .collect();
    pair.pdg
        .extensions
        .insert(crate::OPERATION_MAP_KEY.into(), json!(mapping));
    pair.cfg.nodes = IDS[..7]
        .iter()
        .copied()
        .chain(["entry", "done", "exit"])
        .map(cfg_node)
        .collect();
}

fn cfg_node(id: &str) -> crate::CfgNode {
    let kind = match id {
        "entry" => "entry",
        "exit" => "exit",
        "selector" => "condition",
        _ => "block",
    };
    let operations = if IDS[..7].contains(&id) {
        vec![json!({
            "id": format!("operation:{id}"), "original_kind": "replay_fixture",
            "text": id, "source_span": {"source_id": "src:example", "start": 0, "end": 1}
        })]
    } else {
        vec![]
    };
    let mut node = json!({"id": id, "kind": kind,
        "origin": "hand-authored replay fixture", "operations": operations});
    if id == "exit" {
        node["exit_kind"] = json!("normal");
    }
    serde_json::from_value(node).unwrap()
}

fn value_site(pair: &mut Pair) {
    pair.pdg.extensions.insert("compat.rust.hir-r0:value-sites".into(), json!([{
        "pdg_node_id": "merge", "controller_pdg_node_id": "selector",
        "cfg_successor_node_id": "done", "construct_span": {"source_id": "src", "start": 0, "end": 1},
        "alternatives": [
            {"outcome": "false", "value_pdg_node_id": "ok_result"},
            {"outcome": "true", "value_pdg_node_id": "err_result"}]}]));
}

fn operations(pair: &Pair) -> Operations<'_> {
    let node = |id: &str| pair.pdg.nodes.iter().find(|node| node.id == id).unwrap();
    Operations {
        saved: node("saved"),
        selector: node("selector"),
        payloads: [node("ok"), node("err")],
        constructor: node("ctor"),
        results: [node("ok_result"), node("err_result")],
        merge: node("merge"),
    }
}

fn replay(pair: &Pair, maps_error: bool) -> Result<(), String> {
    let source = source();
    let checked = checked(&source, maps_error);
    let ops = operations(pair);
    effects(&checked, &ops)?;
    operands(pair.as_view(), &checked, &ops)?;
    let guards = super::super::conditional_payloads::Guards::new(pair.as_view())?;
    branches(&guards, &checked, &ops)?;
    order(pair.as_view(), &checked, &ops)?;
    merge(pair.as_view(), &ops)
}

#[test]
fn both_constructor_branches_conserve_exactly_one_selected_payload() {
    for maps_error in [false, true] {
        assert!(replay(&fixture(maps_error), maps_error).is_ok());
    }
}

#[test]
fn swapped_payload_whole_result_forwarding_and_surplus_consumers_are_rejected() {
    for mutation in 0..5 {
        let mut pair = fixture(false);
        match mutation {
            0 => pair.pdg.edges[3].source = "err".into(),
            1 => pair.pdg.edges[5].source = "saved".into(),
            2 => pair.pdg.edges.push(pair.pdg.edges[3].clone()),
            3 => {
                let mut edge = pair.pdg.edges[3].clone();
                edge.target = "err_result".into();
                pair.pdg.edges.push(edge);
            }
            _ => pair.pdg.edges[1].operand_position = Some(1),
        }
        assert!(replay(&pair, false).is_err(), "mutation {mutation}");
    }
}

#[test]
fn opposite_guards_and_constructor_before_payload_bypasses_are_rejected() {
    for mutation in 0..3 {
        let mut pair = fixture(false);
        match mutation {
            0 => {
                pair.cfg.edges[2].kind = CfgEdgeKind::True;
                pair.cfg.edges[3].kind = CfgEdgeKind::False;
            }
            1 => {
                let mut edge = pair.cfg.edges[0].clone();
                edge.target = "ctor".into();
                pair.cfg.edges.push(edge);
            }
            _ => {
                let mut edge = pair.cfg.edges[2].clone();
                edge.target = "ctor".into();
                pair.cfg.edges.push(edge);
            }
        }
        assert!(replay(&pair, false).is_err(), "mutation {mutation}");
    }
}

#[test]
fn merge_branch_replacement_unrelated_input_and_invented_effects_are_rejected() {
    for mutation in 0..5 {
        let mut pair = fixture(false);
        match mutation {
            0 => {
                pair.pdg
                    .extensions
                    .get_mut("compat.rust.hir-r0:value-sites")
                    .unwrap()[0]["alternatives"][0]["value_pdg_node_id"] = json!("err_result")
            }
            1 => {
                pair.pdg.edges.pop();
            }
            2 => {
                let mut edge = pair.pdg.edges[0].clone();
                edge.target = "merge".into();
                pair.pdg.edges.push(edge);
            }
            3 => pair.pdg.nodes[4].attributes.callee = Some(crate::StringFact::known("unrelated")),
            _ => {
                pair.pdg.nodes[4].annotations.remove(KEY);
            }
        }
        assert!(replay(&pair, false).is_err(), "mutation {mutation}");
    }
}

#[test]
fn identical_macro_spans_select_only_the_exact_stage_role() {
    let mut pair = fixture(false);
    let source = source();
    let stage = checked(&source, false);
    let selector = &mut pair.pdg.nodes[1];
    selector.attributes.construct = Some(CONSTRUCTS[0].into());
    selector.source_span = Some(crate::Span {
        source_id: "src:example".into(),
        start: 0,
        end: 1,
    });
    let mut neighbor = selector.clone();
    neighbor.id = "neighbor-selector".into();
    neighbor
        .attributes
        .extensions
        .get_mut("compat.rust.hir-r0:source-operation")
        .unwrap()["role"] = json!(checked(&source, true).role());
    pair.pdg.nodes.push(neighbor);
    let found = tagged_operation(pair.as_view(), &source, stage.role(), CONSTRUCTS[0]).unwrap();
    assert_eq!(found.id, "selector");
    assert!(operation_for(found, &stage));
    assert!(!operation_for(pair.pdg.nodes.last().unwrap(), &stage));
    pair.pdg
        .nodes
        .last_mut()
        .unwrap()
        .attributes
        .extensions
        .get_mut("compat.rust.hir-r0:source-operation")
        .unwrap()["role"] = json!(stage.role());
    assert!(tagged_operation(pair.as_view(), &source, stage.role(), CONSTRUCTS[0]).is_err());
}

#[test]
fn copied_metadata_cannot_replace_missing_or_opposite_stage_provenance() {
    for mutation in 0..3 {
        let mut pair = fixture(false);
        let node = if mutation == 2 {
            &mut pair.pdg.nodes[7]
        } else {
            &mut pair.pdg.nodes[1]
        };
        if mutation == 0 {
            node.attributes
                .extensions
                .remove("compat.rust.hir-r0:source-operation");
        } else {
            node.attributes
                .extensions
                .get_mut("compat.rust.hir-r0:source-operation")
                .unwrap()["role"] = json!(checked(&source(), true).role());
        }
        assert!(replay(&pair, false).is_err(), "mutation {mutation}");
    }
}

#[test]
fn source_routing_uses_checked_stage_and_rejects_changed_span_kind_or_role() {
    let mut pair = fixture(false);
    let success = source();
    let mut error = source();
    error.hir.local = 2;
    let stages = [checked(&success, false), checked(&error, true)];
    let node = &mut pair.pdg.nodes[1];
    node.original_kind = crate::StringFact::known("expr_method_call");
    node.attributes.construct = Some(CONSTRUCTS[0].into());
    node.source_span = Some(crate::Span {
        source_id: "src:example".into(),
        start: 0,
        end: 1,
    });
    assert_eq!(stage_source(node, &stages).unwrap().hir, success.hir);
    node.attributes
        .extensions
        .get_mut("compat.rust.hir-r0:source-operation")
        .unwrap()["role"] = json!(stages[1].role());
    assert_eq!(stage_source(node, &stages).unwrap().hir, error.hir);
    for mutation in 0..3 {
        let mut changed = node.clone();
        match mutation {
            0 => changed.source_span.as_mut().unwrap().end = 2,
            1 => changed.original_kind = crate::StringFact::known("expr_path"),
            _ => {
                changed
                    .attributes
                    .extensions
                    .remove("compat.rust.hir-r0:source-operation");
            }
        }
        assert!(
            stage_source(&changed, &stages).is_err(),
            "mutation {mutation}"
        );
    }
}
