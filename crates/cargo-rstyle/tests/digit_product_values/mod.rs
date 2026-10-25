//! One Cargo capture of the unchanged selected method bodies; the original
//! whole-crate admission/oracle run remains a separate acceptance requirement.
mod corruption;
mod summaries;

use super::*;
use rstyle_pdg::{NodeCategory, PdgNode, producer::Pair};

const ORIGINS: &str = "rstyle.data:digit-product-origins";

fn selected_sources() -> Value {
    serde_json::from_str(include_str!(
        "../../../../eval/coverage-development/time-digit-prefixes/selection.json"
    ))
    .unwrap()
}

fn source(selection: &Value) -> String {
    let mut source = String::from(
        "#[derive(Debug)]\npub(crate) struct ParsedItem<'a, T>(pub(crate) &'a [u8], pub(crate) T);\n\
         pub(crate) struct ExactlyNDigits<const N: u8>;\n",
    );
    source.push_str(
        "mod sealed { pub trait Sealed {} } impl sealed::Sealed for core::time::Duration {}\n",
    );
    for (index, function) in selection["functions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        source.push_str(&format!("impl ExactlyNDigits<{}> {{\n", index + 1));
        source.push_str(function["source"].as_str().unwrap());
        source.push_str("\n}\n");
    }
    source
}

#[test]
fn original_digit_product_values_compose_and_reject_forged_documents() {
    let selection = selected_sources();
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source(&selection)).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "status={} stderr={} capture_failures={} handoff_failures={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        report["capture_failures"],
        report["handoff_failures"]
    );
    let functions: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            function["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.ends_with("::parse"))
        })
        .collect();
    assert_eq!(functions.len(), 3);
    for function in &functions {
        assert_eq!(
            function["outcome"], "admitted",
            "{}: {}",
            function["facts"]["name"], function["reasons"]
        );
        let (store, index) = data_unit(function);
        let analysis = store.analysis();
        let facts = analysis.function(index).unwrap();
        let selected = selection["functions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|selected| selected["source"].as_str() == Some(facts.source.bytes.as_ref()))
            .unwrap();
        let pair = rstyle_pdg::producer::produce(&facts).unwrap();
        rstyle_pdg::producer::verify(&pair, &facts).unwrap();
        rstyle_pdg::data_documents::verify(&pair, &facts).unwrap();
        rstyle_ccgraph::admit_produced(&pair, &facts).unwrap();
        expected_components(&pair, selected + 1);
        if selected == 2 {
            corruption::reject_source_ownership(function);
            renamed_nodes(&pair, &facts);
            corruption::reject_all(&pair, &facts);
        }
    }
    check_complete_data_pairs(&report, 3);
}

fn node<'a>(pair: &'a Pair, construct: &str) -> &'a PdgNode {
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap()
}

fn expected_components(pair: &Pair, digits: usize) {
    assert_eq!(pair.pdg.extensions[ORIGINS]["prefix"]["consumed"], digits);
    let some = &origin_value(pair, "data.digit-some")["footprint"]["cases"][0];
    assert_eq!(some["tag"], "Some");
    assert_eq!(
        some["components"][0]["selection"],
        serde_json::json!([0, 0])
    );
    assert_eq!(some["components"][0]["origins"][0]["parameter"], 0);
    assert_eq!(
        some["components"][0]["origins"][0]["path"],
        serde_json::json!([
            {"kind":"SharedReferent"}, {"kind":"PrefixSuffix", "start":digits}
        ])
    );
    assert_eq!(
        some["components"][1]["selection"],
        serde_json::json!([0, 1])
    );
    assert_eq!(some["components"][1]["origins"], serde_json::json!([]));
    let none = &origin_value(pair, "data.digit-none")["footprint"]["cases"][0];
    assert_eq!(none["tag"], "None");
    assert_eq!(none["components"], serde_json::json!([]));
    let read = node(pair, "data.indirect-deref");
    let selector = node(pair, "slice.prefix-byte");
    assert_ne!(read.id, selector.id);
    assert_eq!(read.category, NodeCategory::Read);
    assert_eq!(read.attributes.reads.len(), 1);
    assert_eq!(selector.attributes.reads.len(), 1);
    assert_eq!(
        read.attributes.reads[0].location_id,
        selector.attributes.reads[0].location_id
    );
}

fn origin_value<'a>(pair: &'a Pair, construct: &str) -> &'a Value {
    let id = &node(pair, construct).id;
    &pair.pdg.extensions[ORIGINS]["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["pdg_node_id"].as_str() == Some(id.as_str()))
        .unwrap()["value"]
}

fn renamed_nodes(pair: &Pair, facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    let names: std::collections::BTreeMap<_, _> = pair
        .pdg
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), format!("renamed-node:{index}")))
        .collect();
    let mut document = serde_json::to_value(pair).unwrap();
    rename_location_references(&mut document, &names);
    let renamed: Pair = serde_json::from_value(document).unwrap();
    rstyle_pdg::data_documents::verify(&renamed, facts).unwrap();
    rstyle_pdg::producer::verify(&renamed, facts).unwrap();
    rstyle_ccgraph::admit_produced(&renamed, facts).unwrap();
}
