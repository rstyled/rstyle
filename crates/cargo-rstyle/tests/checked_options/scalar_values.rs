use super::*;

#[test]
fn scalar_option_family_completes_the_strict_pipeline() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/option-byte-widths.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 6);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(name, "nested" | "custom") {
                "excluded"
            } else {
                "admitted"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_evaluation_complete",
        "pair_enumeration_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 6);
    assert!(
        pairs
            .iter()
            .all(|pair| pair["evaluation_status"] == "evaluated")
    );
}

#[test]
fn scalar_option_runtime_oracle_checks_every_byte_and_saved_results() {
    let directory = tempfile::tempdir().unwrap();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/option-byte-widths.rs"),
        include_str!("../../../../eval/coverage-development/option-byte-width-oracle.rs")
    );
    let path = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&path, source).unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2021", "--test", "-C", "overflow-checks=on"])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn scalar_option_documents_reject_constructor_and_payload_corruption() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn wrap(n: u8) -> Option<u8> { Some(n) }\n\
         pub fn consume(n: Option<u8>) -> u8 { match n { Some(v) => v, None => 0 } }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    check_aggregate_documents(&report);
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted", "{function}");
        reject_option_document_changes(function);
    }
}

fn reject_option_document_changes(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["semantics", "kind", "type", "operand", "effects", "policy"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_option_document(&mut pair, mutation);
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
fn scalar_option_constructor_authority_and_arity_are_required() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn wrap(n: u8) -> Option<u8> { Some(n) }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "admitted", "{function}");
    for mutation in ["core", "variant", "arity"] {
        let mut facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let constructor = &mut facts.body.children[0];
        if mutation == "arity" {
            constructor.children.clear();
        } else {
            let resolution = constructor.res.as_mut().unwrap();
            let mut definition: Value = serde_json::from_str(&resolution.description).unwrap();
            definition[if mutation == "core" {
                "crate_hash"
            } else {
                "definition_hash"
            }] = "unregistered".into();
            resolution.description = serde_json::to_string(&definition).unwrap();
        }
        shared_evidence::reject_facts(function, &facts);
    }
}

fn corrupt_option_document(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            matches!(
                node.attributes.construct.as_deref(),
                Some("data.option-some" | "data.option-value-payload")
            )
        })
        .unwrap();
    match mutation {
        "policy" => {
            pair.pdg.profile.version = "3.41.0".into();
            pair.cfg.profile.version = "3.41.0".into();
        }
        "semantics" => {
            node.annotations.remove("rstyle.data:checked-option");
        }
        "kind" => {
            node.attributes.construct = Some("data.option-none".into());
        }
        "type" => {
            node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "u16".into();
        }
        "effects" => {
            node.attributes.callee = Some(rstyle_pdg::StringFact::known("fabricated"));
        }
        _ => {
            pair.pdg
                .edges
                .retain(|edge| !(edge.target == node.id && edge.subkind == "value_operand"));
        }
    }
}
