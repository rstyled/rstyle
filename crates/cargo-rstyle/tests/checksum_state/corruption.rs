use super::*;

fn captured_conversion() -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn widen(value: u16) -> u32 { u32::from(value) }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"][0]["outcome"], "admitted", "{report}");
    (directory, report["functions"][0].clone())
}

#[test]
fn scalar_from_rejects_missing_wrong_or_nested_selected_implementations() {
    let (_directory, function) = captured_conversion();
    for mutation in [
        "missing",
        "core",
        "trait",
        "implementation",
        "nested",
        "signature",
    ] {
        let mut facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let call = &mut facts.body.children[0];
        let resolution = call.res.as_mut().unwrap();
        let mut callee: Value = serde_json::from_str(&resolution.description).unwrap();
        match mutation {
            "missing" => {
                callee.as_object_mut().unwrap().remove("implementation");
            }
            "core" => {
                callee["implementation"]["crate_hash"] = "wrong".into();
            }
            "trait" => {
                callee["definition_hash"] = "wrong".into();
            }
            "implementation" => {
                callee["implementation"]["definition_hash"] = "wrong".into();
            }
            "nested" => {
                callee["implementation"]["implementation"] = callee["implementation"].clone();
            }
            _ => {
                call.dispatch.as_mut().unwrap().operand_tys = vec!["u32".into()];
            }
        }
        resolution.description = serde_json::to_string(&callee).unwrap();
        shared_evidence::reject_facts(&function, &facts);
    }
}

#[test]
fn scalar_from_documents_reject_changed_summary_type_operands_and_policy() {
    let (_directory, function) = captured_conversion();
    let (unit, index) = data_unit(&function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["summary", "type", "operand", "policy"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let call = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.attributes.construct.as_deref() == Some("data.scalar-from"))
            .unwrap();
        match mutation {
            "summary" => {
                call.attributes.callee = Some(rstyle_pdg::StringFact::known("fabricated"));
            }
            "type" => {
                call.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] =
                    "u16".into();
            }
            "policy" => {
                pair.pdg.profile.version = "3.42.0".into();
                pair.cfg.profile.version = "3.42.0".into();
            }
            _ => {
                pair.pdg
                    .edges
                    .retain(|edge| !(edge.target == call.id && edge.subkind == "value_operand"));
            }
        }
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn scalar_from_capture_does_not_extend_historical_data_profiles() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn widen(value: u16) -> u32 { u32::from(value) }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "excluded", "{function}");
    let call = &function["facts"]["body"]["children"][0];
    let callee: Value = serde_json::from_str(call["res"]["description"].as_str().unwrap()).unwrap();
    assert!(callee["implementation"].is_null());
}
