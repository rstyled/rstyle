//! Complete source-selected parser composition, strict replay and semantic attacks.
use super::*;
use rstyle_pdg::unit_store::{BundleManifest, UnitStore, transport_checksum};

const SELECTION: &str =
    include_str!("../../../../eval/coverage-development/uncompressed-size/selection.json");

fn selected() -> (String, String) {
    let selection: Value = serde_json::from_str(SELECTION).unwrap();
    (
        selection["owners"][0]["source"].as_str().unwrap().into(),
        selection["functions"][0]["source"].as_str().unwrap().into(),
    )
}

fn source() -> String {
    let (owner, original) = selected();
    let alpha = original
        .replace("uncompressed_size", "alpha")
        .replace("input", "bytes")
        .replace("let size", "let header")
        .replace("size.try_into()", "header.try_into()")
        .replace("*size", "*header")
        .replace("rest", "tail");
    let stored = original.replace("uncompressed_size", "stored").replace(
        "let size = input.get(..4).ok_or(DecompressError::ExpectedAnotherByte)?;",
        "let saved = input.get(..4).ok_or(DecompressError::ExpectedAnotherByte); let size = saved?;",
    );
    let inclusive = original
        .replace("uncompressed_size", "inclusive")
        .replace("&input[4..]", "&input[4..=5]");
    let (snapshot, endpoint) = endpoint_sources(&original);
    let enum_endpoint = original
        .replace("uncompressed_size", "enum_endpoint")
        .replace(
            "input.get(..4)",
            "input.get(..{ Boundary::Header as usize })",
        );
    let boundary = "#[derive(Clone, Copy)] enum Boundary { Header = 4 }";
    format!(
        "{owner}\n{boundary}\n{original}\n{alpha}\n{stored}\n{inclusive}\n{snapshot}\n{endpoint}\n{enum_endpoint}\n"
    )
}

fn endpoint_sources(original: &str) -> (String, String) {
    let snapshot = original.replace(
        "pub fn uncompressed_size(input: &[u8]) -> Result<(usize, &[u8]), DecompressError> {",
        "pub fn snapshot<'a>(input: &'a [u8], other: &'a [u8]) -> Result<(usize, &'a [u8]), DecompressError> { let mut selected = input;",
    ).replace("input.get(..4)", "selected.get(..{ selected = other; 4 })");
    let endpoint = original
        .replace("uncompressed_size", "saved_endpoint")
        .replace(
            "let size = input.get(..4)",
            "let mut end = 4usize; let size = input.get(..{ let saved = end; end = 0; saved })",
        );
    (snapshot, endpoint)
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

#[test]
fn original_borrowed_parser_connects_checked_header_to_suffix_result() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "capture={} handoff={} stderr={}",
        report["capture_failures"],
        report["handoff_failures"],
        String::from_utf8_lossy(&output.stderr)
    );
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    for name in [
        "uncompressed_size",
        "alpha",
        "snapshot",
        "saved_endpoint",
        "enum_endpoint",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    }
    for name in ["stored", "inclusive"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{}", entry["reasons"]);
    }
    check_matching(&report);
    let unit = super::word_classifiers::verify_admissions(&report);
    let entry = function(&report, "uncompressed_size");
    check_evidence(&unit, entry);
    for name in ["snapshot", "saved_endpoint", "enum_endpoint"] {
        check_evidence(&unit, function(&report, name));
    }
    check_endpoint_enum(&unit, function(&report, "enum_endpoint"));
    reject_witnesses(&unit, entry);
    reject_graphs(&unit, entry);
    reject_resealed(entry);
}

fn check_endpoint_enum(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let pair = shared_evidence::pair(unit, entry);
    for construct in ["data.fieldless-value", "data.fieldless-cast"] {
        assert_eq!(
            pair.pdg
                .nodes
                .iter()
                .filter(|node| { node.attributes.construct.as_deref() == Some(construct) })
                .count(),
            1,
            "{construct}"
        );
    }
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
}

fn check_matching(report: &Value) {
    let first = &function(report, "uncompressed_size")["key"];
    let second = &function(report, "alpha")["key"];
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (&pair["first"] == first && &pair["second"] == second)
                || (&pair["first"] == second && &pair["second"] == first)
        })
        .unwrap();
    assert_eq!(pair["decision"], "Accepted", "{pair}");
}

fn try_node(node: &rstyle_pdg::HirNode) -> Option<&rstyle_pdg::HirNode> {
    if node.kind == "expr_try_desugar" {
        return Some(node);
    }
    node.children.iter().find_map(try_node)
}

fn check_evidence(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let checked =
        rstyle_pdg::borrowed_result_propagation::verify(try_node(&facts.body).unwrap(), &facts)
            .unwrap();
    let roots = rstyle_pdg::data_origins::origins(checked.operand, &facts).unwrap();
    assert_eq!(
        roots,
        vec![rstyle_pdg::data_origins::Origin {
            parameter: 0,
            path: vec![rstyle_pdg::field_locations::Projection::SharedReferent],
        }]
    );
    check_graph_evidence(shared_evidence::pair(unit, entry));
}

fn check_graph_evidence(pair: &rstyle_pdg::producer::Pair) {
    for construct in [
        "data.option-get-prefix",
        "data.option-ok-or",
        "try.borrowed-result-is-err",
        "try.borrowed-result-error",
        "try.borrowed-result-ok-reference",
        "data.result-ok",
    ] {
        assert!(
            pair.pdg
                .nodes
                .iter()
                .any(|node| node.attributes.construct.as_deref() == Some(construct)),
            "{construct}"
        );
    }
    let suffix = pair
        .pdg
        .nodes
        .iter()
        .find_map(|node| node.annotations.get("rstyle.data:subslice-bounds"))
        .unwrap();
    assert_eq!(
        suffix,
        &serde_json::json!({
            "rule":"start-less-or-equal-length", "base_operand":0,
            "start_operand":1, "implicit_end":"base-length", "exceptional_flow":"omitted"
        })
    );
    let success = operation(pair, "try.borrowed-result-ok-reference");
    assert_eq!(
        pair.pdg
            .edges
            .iter()
            .filter(|edge| edge.target == success && edge.subkind == "value_operand")
            .count(),
        1
    );
}

fn reject_witnesses(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let original = try_node(&facts.body).unwrap();
    for attack in ["input", "output", "error", "conversion", "binding", "span"] {
        let mut node = original.clone();
        let mut witness: rstyle_pdg::borrowed_result_propagation::capture::Witness =
            serde_json::from_str(&node.res.as_ref().unwrap().description).unwrap();
        match attack {
            "input" => witness.shape.input = witness.shape.output.clone(),
            "output" => witness.shape.output_payload = witness.shape.payload.clone(),
            "error" => witness.shape.error = "usize".into(),
            "conversion" => witness.conversion.implementation.definition_hash = "forged".into(),
            "binding" => witness.continue_arm.binding.local = u32::MAX,
            _ => witness.operand_span.end = witness.operand_span.start,
        }
        node.res.as_mut().unwrap().description = serde_json::to_string(&witness).unwrap();
        assert!(
            rstyle_pdg::borrowed_result_propagation::verify(&node, &facts).is_err(),
            "{attack}"
        );
    }
}

fn operation(pair: &rstyle_pdg::producer::Pair, construct: &str) -> String {
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap()
        .id
        .clone()
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, attack: &str) {
    if attack.starts_with("extra_") {
        return corrupt_consumers(pair, attack);
    }
    if attack.starts_with("revisit_") {
        return corrupt_revisit(pair, attack);
    }
    let success = operation(pair, "try.borrowed-result-ok-reference");
    if attack == "saved" {
        pair.pdg
            .edges
            .iter_mut()
            .find(|edge| edge.target == success && edge.subkind == "value_operand")
            .unwrap()
            .operand_position = Some(1);
    } else if attack == "guard" {
        let selector = operation(pair, "try.borrowed-result-is-err");
        let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
        let cfg = &mapping
            .iter()
            .find(|item| item.pdg_node_id == selector)
            .unwrap()
            .cfg_node_id;
        pair.cfg
            .edges
            .iter_mut()
            .find(|edge| &edge.source == cfg && edge.kind == rstyle_pdg::CfgEdgeKind::True)
            .unwrap()
            .kind = rstyle_pdg::CfgEdgeKind::False;
    } else {
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.annotations.contains_key("rstyle.data:subslice-bounds"))
            .unwrap();
        node.annotations.remove("rstyle.data:subslice-bounds");
    }
}

fn corrupt_revisit(pair: &mut rstyle_pdg::producer::Pair, attack: &str) {
    let success = operation(pair, "try.borrowed-result-ok-reference");
    let target = if attack == "revisit_selector" {
        operation(pair, "try.borrowed-result-is-err")
    } else {
        success.clone()
    };
    let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = |id: &str| {
        mapping
            .iter()
            .find(|item| item.pdg_node_id == id)
            .unwrap()
            .cfg_node_id
            .clone()
    };
    pair.cfg.edges.push(rstyle_pdg::CfgEdge {
        id: format!("forged-{attack}"),
        source: cfg(&success),
        target: cfg(&target),
        kind: rstyle_pdg::CfgEdgeKind::Normal,
        case_value: None,
        loop_role: None,
        reason: None,
    });
}

fn corrupt_consumers(pair: &mut rstyle_pdg::producer::Pair, attack: &str) {
    let success = operation(pair, "try.borrowed-result-ok-reference");
    let mut edge = pair
        .pdg
        .edges
        .iter()
        .find(|edge| edge.target == success && edge.subkind == "value_operand")
        .unwrap()
        .clone();
    if attack.contains("error") {
        edge.source = operation(pair, "try.borrowed-result-error");
    }
    edge.id = format!("forged-{attack}");
    edge.target = operation(pair, "data.result-ok");
    edge.kind = rstyle_pdg::EdgeFamily::Data;
    edge.subkind = if attack.ends_with("alternative") {
        "value_alternative"
    } else {
        "reaching_definition"
    }
    .into();
    edge.operand_position = None;
    pair.pdg.edges.push(edge);
}

fn reject_graphs(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let original = shared_evidence::pair(unit, entry);
    for attack in [
        "saved",
        "revisit_selector",
        "revisit_success",
        "guard",
        "suffix_bounds",
        "extra_input_alternative",
        "extra_input_definition",
        "extra_error_alternative",
        "extra_error_definition",
    ] {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        corrupt(&mut pair, attack);
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "strict replay: {attack}"
        );
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "independent verifier: {attack}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{attack}"
        );
    }
}

fn wire_try(node: &mut Value) -> Option<&mut Value> {
    if node["kind"] == "expr_try_desugar" {
        return Some(node);
    }
    node["children"]
        .as_array_mut()?
        .iter_mut()
        .find_map(wire_try)
}

fn reject_resealed(entry: &Value) {
    for graph_attack in [false, true] {
        let mut bundle = shared_evidence::wire(entry);
        let index = shared_evidence::index(entry);
        UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
        if graph_attack {
            let root = bundle["roots"][index].as_u64().unwrap() as usize;
            let node = bundle["graphs"][root]["pdg"]["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|node| node["attributes"]["construct"] == "try.borrowed-result-error")
                .unwrap();
            node["attributes"]["construct"] = "try.borrowed-result-ok-reference".into();
        } else {
            let node = wire_try(&mut bundle["functions"][index]["body"]).unwrap();
            let mut witness: Value =
                serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
            witness["conversion"]["implementation"]["definition_hash"] = "forged".into();
            node["res"]["description"] = serde_json::to_string(&witness).unwrap().into();
        }
        let bytes = serde_json::to_vec(&bundle).unwrap();
        let manifest = BundleManifest {
            schema: "rstyle.shared-unit.manifest/2".into(),
            checksum: transport_checksum(&bytes),
            bytes: bytes.len() as u64,
        };
        manifest.verify(&bytes).unwrap();
        assert!(
            UnitStore::from_bundle(&bytes).is_err(),
            "graph={graph_attack}"
        );
        assert!(
            rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err()
        );
    }
}

#[test]
fn borrowed_parser_rejects_drop_reference_and_result_storage() {
    let (owner, original) = selected();
    for attack in ["drop", "reference"] {
        let directory = clone_workspace();
        let owner = if attack == "drop" {
            format!("{owner}\nimpl Drop for DecompressError {{ fn drop(&mut self) {{}} }}")
        } else {
            owner.replace("expected: usize", "expected: &'static str")
        };
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{owner}\n{original}"),
        )
        .unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(
            output.status.success(),
            "capture={} handoff={} stderr={}",
            report["capture_failures"],
            report["handoff_failures"],
            String::from_utf8_lossy(&output.stderr)
        );
        let entry = function(&report, "uncompressed_size");
        assert_eq!(
            entry["outcome"], "excluded",
            "{attack}: {}",
            entry["reasons"]
        );
    }
}
