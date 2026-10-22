//! Complete selected flag/error caller and exact propagation tampering.
use super::*;

const NEXT: &str =
    include_str!("../../../../../eval/coverage-development/result-propagation-selection.json");

fn source() -> String {
    let selection: Value = serde_json::from_str(NEXT).unwrap();
    let functions = selection["functions"].as_array().unwrap();
    let owners = selection["owners"].as_array().unwrap();
    let declarations = owners
        .iter()
        .map(|owner| owner["source"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let available = functions[0]["source"].as_str().unwrap();
    format!(
        "{}\n{declarations}\nimpl Look {{ {} }}\nimpl LookSet {{ {}\n{}\n{available}\n{} }}\n{EXTRA}",
        selected_source(),
        functions[1]["source"].as_str().unwrap(),
        functions[2]["source"].as_str().unwrap(),
        functions[3]["source"].as_str().unwrap(),
        available.replace("fn available(", "fn alpha_available("),
    )
}

const EXTRA: &str = r#"
pub fn wrapper(value: LookSet) -> Result<(), UnicodeWordBoundaryError> { value.available() }
pub fn alpha_wrapper(value: LookSet) -> Result<(), UnicodeWordBoundaryError> { value.alpha_available() }
pub fn sequential(value: LookSet) -> Result<(), UnicodeWordBoundaryError> { value.available()?; UnicodeWordBoundaryError::check()?; Ok(()) }
pub fn alpha_sequential(value: LookSet) -> Result<(), UnicodeWordBoundaryError> { value.available()?; UnicodeWordBoundaryError::check()?; Ok(()) }
pub fn nested() -> Result<(), UnicodeWordBoundaryError> { { UnicodeWordBoundaryError::check()?; UnicodeWordBoundaryError::check() }?; Ok(()) }
pub fn alpha_nested() -> Result<(), UnicodeWordBoundaryError> { { UnicodeWordBoundaryError::check()?; UnicodeWordBoundaryError::check() }?; Ok(()) }
pub struct OtherError(());
impl From<UnicodeWordBoundaryError> for OtherError { fn from(_: UnicodeWordBoundaryError) -> Self { OtherError(()) } }
pub fn converted() -> Result<(), OtherError> { UnicodeWordBoundaryError::check()?; Ok(()) }
pub fn nonunit() -> Result<u8, UnicodeWordBoundaryError> { UnicodeWordBoundaryError::check()?; Ok(1) }
pub fn saved() -> Result<(), UnicodeWordBoundaryError> { let value = UnicodeWordBoundaryError::check(); value?; Ok(()) }
pub fn matched() -> Result<(), UnicodeWordBoundaryError> { match UnicodeWordBoundaryError::check() { Ok(()) => Ok(()), Err(error) => Err(error) } }
pub fn option(value: Option<usize>) -> Option<usize> { Some(value?) }
"#;

#[test]
fn original_flag_result_caller_connects_same_error_propagation_in_each_feature_context() {
    for options in [
        vec![],
        vec!["--no-default-features"],
        vec![
            "--no-default-features",
            "--features",
            "unicode-word-boundary",
        ],
    ] {
        let (_directory, report) = capture(&source(), &options);
        for name in [
            "LookSet::available",
            "LookSet::alpha_available",
            "wrapper",
            "alpha_wrapper",
            "sequential",
            "alpha_sequential",
            "nested",
            "alpha_nested",
        ] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        }
        for name in ["saved", "matched", "option", "converted", "nonunit"] {
            assert_eq!(function(&report, name)["outcome"], "excluded", "{name}");
        }
        let unit = super::super::word_classifiers::verify_admissions(&report);
        accepted_pair(&report, "LookSet::available", "LookSet::alpha_available");
        accepted_pair(&report, "wrapper", "alpha_wrapper");
        accepted_pair(&report, "sequential", "alpha_sequential");
        accepted_pair(&report, "nested", "alpha_nested");
        for name in ["sequential", "nested"] {
            assert_propagation_sites(&unit, function(&report, name), 2);
        }
        if options.is_empty() {
            reject_witness_attacks(&unit, function(&report, "LookSet::available"));
            reject_graph_attacks(&unit, function(&report, "LookSet::available"));
            reject_graph_policy(&unit, function(&report, "wrapper"));
        }
    }
}

fn try_node(node: &mut Value) -> Option<&mut Value> {
    if node["kind"] == "expr_try_desugar" {
        return Some(node);
    }
    node["children"]
        .as_array_mut()?
        .iter_mut()
        .find_map(try_node)
}

struct WitnessAttacks<'a> {
    source: &'a rstyle_pdg::HirNode,
    witness: rstyle_pdg::result_propagation::capture::Witness,
    facts: &'a dyn rstyle_pdg::unit_store::EvidenceFacts,
}

impl WitnessAttacks<'_> {
    fn reject(&self, edit: impl FnOnce(&mut rstyle_pdg::result_propagation::capture::Witness)) {
        let mut node = self.source.clone();
        let mut witness = self.witness.clone();
        edit(&mut witness);
        node.res.as_mut().unwrap().description = serde_json::to_string(&witness).unwrap();
        assert!(rstyle_pdg::result_propagation::verify(&node, self.facts).is_err());
    }
}

fn reject_witness_attacks(unit: &UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let source = find_try(&facts.body).unwrap();
    rstyle_pdg::result_propagation::verify(source, &facts).unwrap();
    let attacks = WitnessAttacks {
        source,
        witness: serde_json::from_str(&source.res.as_ref().unwrap().description).unwrap(),
        facts: &facts,
    };
    for role in ["branch", "residual", "conversion"] {
        for authority in [
            "method",
            "implementation",
            "arguments",
            "implementation_arguments",
        ] {
            attacks.reject(|witness| {
                let selected = match role {
                    "branch" => &mut witness.branch,
                    "residual" => &mut witness.residual,
                    _ => &mut witness.conversion,
                };
                match authority {
                    "method" => selected.method.definition_hash = "forged".into(),
                    "implementation" => selected.implementation.definition_hash = "forged".into(),
                    "arguments" => selected.arguments[0] = "WrongType".into(),
                    _ => selected.implementation_arguments[0] = "WrongType".into(),
                }
            });
        }
    }
    attacks.reject(|witness| witness.operand.local = 999999);
    attacks.reject(|witness| witness.early_return.local = 999999);
    attacks.reject(|witness| witness.break_arm.binding.local = 999999);
    attacks.reject(|witness| witness.continue_arm.binding.local = 999999);
    attacks.reject(|witness| witness.shape.error = "WrongError".into());
    let mut reversed = source.clone();
    reversed.children.swap(1, 2);
    assert!(rstyle_pdg::result_propagation::verify(&reversed, &facts).is_err());
    reject_import_attacks(entry);
}

fn find_try(node: &rstyle_pdg::HirNode) -> Option<&rstyle_pdg::HirNode> {
    if node.kind == "expr_try_desugar" {
        return Some(node);
    }
    node.children.iter().find_map(find_try)
}

fn reject_import_attacks(entry: &Value) {
    // Import enforcement stays covered under strict and generated policies;
    // fine-grained mutations above exercise semantic rejection without reparsing a whole unit.
    reject_resealed(entry, |bundle, index| {
        let node = try_node(&mut bundle["functions"][index]["body"]).unwrap();
        let mut witness: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        witness["branch"]["implementation"]["definition_hash"] = "forged".into();
        node["res"]["description"] = serde_json::to_string(&witness).unwrap().into();
    });
    reject_resealed(entry, |bundle, index| {
        let node = try_node(&mut bundle["functions"][index]["body"]).unwrap();
        node["children"].as_array_mut().unwrap().swap(1, 2);
    });
}

fn assert_propagation_sites(unit: &UnitStore, entry: &Value, expected: usize) {
    let pair = unit.pair(shared_evidence::index(entry)).unwrap();
    for construct in ["try.result-is-err", "try.result-ok-unit"] {
        assert_eq!(
            pair.pdg
                .nodes
                .iter()
                .filter(|node| node.attributes.construct.as_deref() == Some(construct))
                .count(),
            expected
        );
    }
}

fn reject_graph_attacks(unit: &UnitStore, entry: &Value) {
    let index = shared_evidence::index(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    rstyle_pdg::data_documents::verify(original, &facts).unwrap();
    for attack in [
        "selector",
        "operand",
        "duplicate_operand",
        "extra_consumer",
        "duplicate_evaluation",
        "fallthrough",
        "return_and_fallthrough",
        "guard",
        "bypass",
        "effect",
        "policy",
    ] {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        corrupt_graph(&mut pair, attack);
        // Call the semantic verifier directly: rejection cannot come from a stale bundle seal.
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

fn reject_graph_policy(unit: &UnitStore, entry: &Value) {
    let index = shared_evidence::index(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.78.0".into();
    pair.cfg.profile.version = "3.78.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
}

struct Operations {
    selector: String,
    success: String,
    saved: String,
    returned: String,
}

fn operations(pair: &rstyle_pdg::producer::Pair) -> Operations {
    let find = |construct| {
        pair.pdg
            .nodes
            .iter()
            .find(|node| node.attributes.construct.as_deref() == Some(construct))
            .unwrap()
            .id
            .clone()
    };
    let selector = find("try.result-is-err");
    let success = find("try.result-ok-unit");
    let saved = pair
        .pdg
        .edges
        .iter()
        .find(|edge| edge.target == selector && edge.subkind == "value_operand")
        .unwrap()
        .source
        .clone();
    let returned = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Return
                && pair.pdg.edges.iter().any(|edge| {
                    edge.source == saved
                        && edge.target == node.id
                        && edge.subkind == "value_operand"
                })
        })
        .unwrap()
        .id
        .clone();
    Operations {
        selector,
        success,
        saved,
        returned,
    }
}

fn corrupt_graph(pair: &mut rstyle_pdg::producer::Pair, attack: &str) {
    let selected = operations(pair);
    match attack {
        "policy" => {
            pair.pdg.profile.version = "3.78.0".into();
            pair.cfg.profile.version = "3.78.0".into();
        }
        "guard" | "bypass" | "fallthrough" | "return_and_fallthrough" => {
            corrupt_cfg(pair, &selected, attack)
        }
        "operand" | "duplicate_operand" | "extra_consumer" => {
            corrupt_operands(pair, &selected, attack)
        }
        "duplicate_evaluation" => {
            let mut duplicate = pair
                .pdg
                .nodes
                .iter()
                .find(|node| node.id == selected.saved)
                .unwrap()
                .clone();
            duplicate.id = "forged-second-result-evaluation".into();
            pair.pdg.nodes.push(duplicate);
        }
        _ => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == selected.selector)
                .unwrap();
            if attack == "effect" {
                node.attributes.symbol_ids = vec!["invented".into()];
            } else {
                node.attributes.construct = Some("try.result-is-ok".into());
            }
        }
    }
}

fn corrupt_operands(pair: &mut rstyle_pdg::producer::Pair, selected: &Operations, attack: &str) {
    let index = pair
        .pdg
        .edges
        .iter()
        .position(|edge| {
            edge.target == selected.success
                && edge.source == selected.saved
                && edge.subkind == "value_operand"
        })
        .unwrap();
    if attack == "operand" {
        pair.pdg.edges[index].operand_position = Some(1);
        return;
    }
    let mut extra = pair.pdg.edges[index].clone();
    extra.id = format!("forged-{attack}");
    if attack == "extra_consumer" {
        extra.target = pair
            .pdg
            .nodes
            .iter()
            .find(|node| {
                node.category == rstyle_pdg::NodeCategory::Return && node.id != selected.returned
            })
            .unwrap()
            .id
            .clone();
    }
    pair.pdg.edges.push(extra);
}

fn corrupt_cfg(pair: &mut rstyle_pdg::producer::Pair, selected: &Operations, attack: &str) {
    use rstyle_pdg::CfgEdgeKind;
    let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = |id: &str| {
        mapping
            .iter()
            .find(|entry| entry.pdg_node_id == id)
            .unwrap()
            .cfg_node_id
            .clone()
    };
    let selector = cfg(&selected.selector);
    if attack == "guard" || attack == "bypass" {
        let index = pair
            .cfg
            .edges
            .iter()
            .position(|edge| edge.source == selector && edge.kind == CfgEdgeKind::True)
            .unwrap();
        if attack == "guard" {
            pair.cfg.edges[index].kind = CfgEdgeKind::False;
        } else {
            let mut bypass = pair.cfg.edges[index].clone();
            bypass.id = "forged-unconditional-error-path".into();
            bypass.kind = CfgEdgeKind::Normal;
            pair.cfg.edges.push(bypass);
        }
        return;
    }
    let returned = cfg(&selected.returned);
    let index = pair
        .cfg
        .edges
        .iter()
        .position(|edge| edge.source == returned && edge.kind == CfgEdgeKind::Return)
        .unwrap();
    if attack == "fallthrough" {
        pair.cfg.edges[index].kind = CfgEdgeKind::Normal;
    } else {
        let mut fallthrough = pair.cfg.edges[index].clone();
        fallthrough.id = "forged-return-also-continues".into();
        fallthrough.kind = CfgEdgeKind::Normal;
        fallthrough.target = cfg(&selected.success);
        pair.cfg.edges.push(fallthrough);
    }
}
