//! Complete checked-reference propagation with independently resealed attacks.
use super::*;
use rstyle_pdg::unit_store::{BundleManifest, UnitStore, transport_checksum};

const SELECTION: &str =
    include_str!("../../../../tests/fixtures/function-families/prefilter-prefix/selection.json");

fn source() -> String {
    let selected: Value = serde_json::from_str(SELECTION).unwrap();
    let mut source = selected["owners"]
        .as_array()
        .unwrap()
        .iter()
        .map(|owner| owner["source"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    source.push_str(
        "\ntrait PrefilterI { fn prefix(&self, haystack: &[u8], span: Span) -> Option<Span>; }\n",
    );
    for (index, function) in selected["functions"].as_array().unwrap().iter().enumerate() {
        let owner = ["Memchr", "Memchr2", "Memchr3"][index];
        let body = function["source"].as_str().unwrap();
        source.push_str(&format!(
            "impl PrefilterI for {owner} {{ {body} }}\nimpl {owner} {{ {} }}\n",
            body.replace("fn prefix(", "fn alpha_prefix(")
        ));
    }
    source.push_str(r#"
fn scalar(value: Option<usize>) -> Option<usize> { Some(value?) }
#[derive(Clone, Copy)] struct RefProduct<'a> { value: &'a [u8] }
fn reference_product(a: &[u8]) -> Option<RefProduct<'_>> { let _b = *a.get(0)?; Some(RefProduct { value: a }) }
#[derive(Clone, Copy)] struct ArrayProduct { value: [u8; 4] }
fn array_product() -> Option<ArrayProduct> { None }
struct NonCopy { value: u8 }
fn noncopy(a: &[u8]) -> Option<NonCopy> { let b = *a.get(0)?; Some(NonCopy { value: b }) }
fn other(a: &[u16]) -> Option<Span> { let _b = *a.get(0)?; Some(Span { start: 0, end: 1 }) }
fn mutable(a: &mut [u8]) -> Option<Span> { let _b = *a.get_mut(0)?; Some(Span { start: 0, end: 1 }) }
"#);
    source
}

fn entry<'a>(report: &'a Value, source: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["source"]["bytes"] == source)
        .unwrap()
}

#[test]
fn original_prefixes_connect_checked_reference_question_mark_to_copy_product_results() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let selected: Value = serde_json::from_str(SELECTION).unwrap();
    for function in selected["functions"].as_array().unwrap() {
        let original = function["source"].as_str().unwrap();
        let first = entry(&report, original);
        let second = entry(&report, &original.replace("fn prefix(", "fn alpha_prefix("));
        for item in [first, second] {
            assert_eq!(item["outcome"], "admitted", "{}", item["reasons"]);
        }
        let comparison = report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pair| {
                (pair["first"] == first["key"] && pair["second"] == second["key"])
                    || (pair["second"] == first["key"] && pair["first"] == second["key"])
            })
            .unwrap();
        assert_eq!(comparison["decision"], "Accepted", "{comparison}");
    }
    for function in report["functions"].as_array().unwrap() {
        if matches!(
            function["facts"]["name"].as_str().unwrap(),
            "scalar" | "noncopy" | "other" | "mutable" | "reference_product" | "array_product"
        ) {
            assert_eq!(function["outcome"], "excluded", "{function}");
        }
    }
    let unit = super::word_classifiers::verify_admissions(&report);
    let first = entry(
        &report,
        selected["functions"][0]["source"].as_str().unwrap(),
    );
    reject_witnesses(&unit, first);
    reject_graphs(&unit, first);
    reject_resealed(first);
}

fn find_try(node: &rstyle_pdg::HirNode) -> Option<&rstyle_pdg::HirNode> {
    if node.kind == "expr_try_desugar" {
        return Some(node);
    }
    node.children.iter().find_map(find_try)
}

fn reject_witnesses(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let source = find_try(&facts.body).unwrap();
    rstyle_pdg::option_propagation::verify(source, &facts).unwrap();
    let witness: rstyle_pdg::option_propagation::capture::Witness =
        serde_json::from_str(&source.res.as_ref().unwrap().description).unwrap();
    for attack in [
        "method",
        "implementation",
        "substitution",
        "residual",
        "output",
        "operand",
        "binding",
        "span",
        "arms",
    ] {
        let mut changed = source.clone();
        let mut witness = witness.clone();
        match attack {
            "method" => witness.branch.method.definition_hash = "forged".into(),
            "implementation" => witness.residual.implementation.definition_hash = "forged".into(),
            "substitution" => witness.residual.implementation_arguments[0] = "&u8".into(),
            "residual" => {
                witness.shape.residual = "std::option::Option<core::convert::Infallible>".into()
            }
            "output" => witness.shape.output = witness.shape.input.clone(),
            "operand" => witness.operand.local = 999999,
            "binding" => witness.continue_arm.binding.local = 999999,
            "span" => witness.operand_span.end = witness.operand_span.start,
            _ => changed.children.swap(1, 2),
        }
        changed.res.as_mut().unwrap().description = serde_json::to_string(&witness).unwrap();
        // Direct semantic checking has no transport seal to go stale.
        assert!(
            rstyle_pdg::option_propagation::verify(&changed, &facts).is_err(),
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
    let selector = operation(pair, "try.option-is-none");
    let payload = operation(pair, "try.option-some-reference");
    let none = operation(pair, "try.option-none");
    if matches!(attack, "guard" | "return") {
        corrupt_control_flow(pair, attack, &selector);
    } else if attack == "saved" || attack == "saved_value" {
        let edge = pair
            .pdg
            .edges
            .iter_mut()
            .find(|edge| edge.target == payload && edge.subkind == "value_operand")
            .unwrap();
        if attack == "saved" {
            edge.operand_position = Some(1);
        } else {
            edge.source = none;
        }
    } else if attack == "none_operand" {
        let mut edge = pair
            .pdg
            .edges
            .iter()
            .find(|edge| edge.target == payload && edge.subkind == "value_operand")
            .unwrap()
            .clone();
        edge.id = "forged-input-forwarded-to-output-none".into();
        edge.target = none;
        pair.pdg.edges.push(edge);
    } else if attack == "none_type" {
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.id == none)
            .unwrap();
        node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "usize".into();
    } else {
        pair.pdg.profile.version = "3.79.0".into();
        pair.cfg.profile.version = "3.79.0".into();
    }
}

fn corrupt_control_flow(pair: &mut rstyle_pdg::producer::Pair, attack: &str, selector: &str) {
    let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    if attack == "guard" {
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
        let returned = pair
            .pdg
            .nodes
            .iter()
            .find(|node| {
                node.category == rstyle_pdg::NodeCategory::Return
                    && node.original_kind.known_value() == Some("expr_ret")
            })
            .unwrap();
        let cfg = &mapping
            .iter()
            .find(|item| item.pdg_node_id == returned.id)
            .unwrap()
            .cfg_node_id;
        pair.cfg
            .edges
            .iter_mut()
            .find(|edge| &edge.source == cfg)
            .unwrap()
            .kind = rstyle_pdg::CfgEdgeKind::Normal;
    }
}

fn reject_graphs(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let original = unit.pair(shared_evidence::index(entry)).unwrap();
    for attack in [
        "guard",
        "return",
        "saved",
        "saved_value",
        "none_operand",
        "none_type",
        "policy",
    ] {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        corrupt(&mut pair, attack);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{attack}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{attack}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{attack}"
        );
    }
}

fn try_value(node: &mut Value) -> Option<&mut Value> {
    if node["kind"] == "expr_try_desugar" {
        return Some(node);
    }
    node["children"]
        .as_array_mut()?
        .iter_mut()
        .find_map(try_value)
}

fn reject_resealed(entry: &Value) {
    for graph_attack in [false, true] {
        let mut bundle = shared_evidence::wire(entry);
        UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
        let index = shared_evidence::index(entry);
        if graph_attack {
            let root = bundle["roots"][index].as_u64().unwrap() as usize;
            let node = bundle["graphs"][root]["pdg"]["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|node| node["attributes"]["construct"] == "try.option-none")
                .unwrap();
            node["attributes"]["construct"] = "try.option-some-reference".into();
        } else {
            let node = try_value(&mut bundle["functions"][index]["body"]).unwrap();
            let mut witness: Value =
                serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
            witness["residual"]["implementation"]["definition_hash"] = "forged".into();
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
