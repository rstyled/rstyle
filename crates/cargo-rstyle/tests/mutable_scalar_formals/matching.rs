//! Similarity uses unchanged defaults; strict memory order remains independent.
use super::*;
use rstyle_ccgraph::{CcgraphConfig, Decision, ImportedFunction};
use rstyle_pdg::{producer::Pair, unit_store::EvidenceFacts};
use std::collections::BTreeMap;

#[test]
fn mutable_scalar_original_alpha_and_reversed_stores_compose_with_default_matching() {
    let selection: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../eval/coverage-development/backtrack-match/selection.json"
    )))
    .unwrap();
    let original = selection["function"]["source"].as_str().unwrap();
    let alpha = alpha(original);
    let reversed = reverse_stores(original);
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{original}\n{alpha}\n{reversed}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let imports = ["backtrack_match", "backtrack_alpha", "backtrack_reversed"]
        .map(|name| check_function(named(&report, name), name));
    check_equivalent_matching(&report, ["backtrack_match", "backtrack_alpha"]);
    check_default_matching(&report, &imports);
}

fn alpha(original: &str) -> String {
    original
        .replacen("fn backtrack_match(", "fn backtrack_alpha(", 1)
        .replace("literal_start", "start_limit")
        .replace("candidate", "probe")
        .replace("input", "haystack")
        .replace("source", "window")
        .replace("cur", "cursor")
}

fn reverse_stores(original: &str) -> String {
    let stores = "        *cur -= 1;\n        *candidate -= 1;";
    assert_eq!(original.matches(stores).count(), 1);
    original
        .replacen("fn backtrack_match(", "fn backtrack_reversed(", 1)
        .replace(stores, "        *candidate -= 1;\n        *cur -= 1;")
}

fn named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

fn check_function(function: &Value, name: &str) -> ImportedFunction {
    assert_eq!(
        function["outcome"], "admitted",
        "{name}: {}",
        function["reasons"]
    );
    let (store, index) = data_unit(function);
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&store, function);
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
    })
    .unwrap();
    check_store_order(pair, name);
    check_renamed_and_historical(pair, &facts);
    let mut imported = rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
    imported.key = function["key"].as_str().unwrap().into();
    imported
}

fn check_store_order(pair: &Pair, name: &str) {
    let stores: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("data.mutable-scalar-store"))
        .collect();
    let expected = if name == "backtrack_reversed" {
        [4, 1]
    } else {
        [1, 4]
    };
    assert_eq!(
        stores
            .iter()
            .map(|node| node.attributes.position.unwrap())
            .collect::<Vec<_>>(),
        expected
    );
    let record = &pair.pdg.extensions[memory_checks::KEY];
    let second_check = pair
        .pdg
        .nodes
        .iter()
        .find(|node| {
            node.category == rstyle_pdg::NodeCategory::Computation
                && node.source_span == stores[1].source_span
                && node
                    .attributes
                    .operator
                    .as_ref()
                    .and_then(rstyle_pdg::StringFact::known_value)
                    == Some("-")
        })
        .unwrap();
    let failure = record["exits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|exit| exit["cfg_node_id"] == format!("P:{}", second_check.id))
        .unwrap();
    let first_location = stores[0].attributes.writes[0].location_id.as_ref().unwrap();
    let state = failure["reaching"]
        .as_array()
        .unwrap()
        .iter()
        .find(|root| root["location_id"] == first_location.as_str())
        .unwrap();
    assert_eq!(state["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(state["definitions"][0]["node_id"], stores[0].id);
    assert_eq!(state["definitions"][0]["loop_carried"], false);
}

fn check_default_matching(report: &Value, imports: &[ImportedFunction]) {
    let expected = rstyle_ccgraph::evaluate_all(imports, &CcgraphConfig::default());
    assert!(!expected.incomplete);
    assert_eq!(expected.pairs.len(), 3);
    assert_eq!(
        report["matching"]["effective_config"],
        serde_json::to_value(&expected.effective_config).unwrap()
    );
    for pair in &expected.pairs {
        let actual = report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|actual| {
                (actual["first"] == pair.first && actual["second"] == pair.second)
                    || (actual["first"] == pair.second && actual["second"] == pair.first)
            })
            .unwrap();
        assert_eq!(actual["evaluation_status"], "evaluated");
        assert_eq!(
            actual["decision"],
            serde_json::to_value(pair.decision).unwrap()
        );
        assert_eq!(
            actual["displayed_score"],
            serde_json::to_value(&pair.displayed_score).unwrap()
        );
        if pair.first == imports[0].key && pair.second == imports[1].key
            || pair.first == imports[1].key && pair.second == imports[0].key
        {
            assert_eq!(pair.decision, Decision::Accepted);
            assert_eq!(pair.displayed_score.as_deref(), Some("1"));
        }
    }
    // A similarity decision for reversed stores does not certify equivalence;
    // the independent store-root and failure-state checks above remain required.
}

fn check_renamed_and_historical(pair: &Pair, facts: &dyn EvidenceFacts) {
    let mut names: BTreeMap<_, _> = pair
        .pdg
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), format!("renamed-pdg:{index}")))
        .collect();
    for (index, node) in pair.cfg.nodes.iter().enumerate() {
        names
            .entry(node.id.clone())
            .or_insert_with(|| format!("renamed-cfg:{index}"));
        for operation in &node.operations {
            names.insert(operation.id.clone(), format!("renamed-operation:{index}"));
        }
    }
    let mut wire = serde_json::to_value(pair).unwrap();
    rename(&mut wire, &names);
    let renamed: Pair = serde_json::from_value(wire).unwrap();
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&renamed.serialize().unwrap(), facts)
    })
    .unwrap();
    let historical = shared_evidence::with_profile_version(pair, "3.92.0");
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&historical.serialize().unwrap(), facts)
        })
        .is_err()
    );
}

fn rename(value: &mut Value, names: &BTreeMap<String, String>) {
    match value {
        Value::String(name) => {
            if let Some(replacement) = names.get(name) {
                *name = replacement.clone();
            }
        }
        Value::Array(values) => values.iter_mut().for_each(|value| rename(value, names)),
        Value::Object(values) => values.values_mut().for_each(|value| rename(value, names)),
        _ => {}
    }
}
