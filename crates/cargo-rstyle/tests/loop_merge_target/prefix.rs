use super::*;

#[test]
fn cyclic_prefix_family_completes_capture_verification_admission_and_matching() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/cyclic-prefix-scans.rs"),
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
            if matches!(name, "write" | "value_break") {
                "excluded"
            } else {
                "admitted"
            },
            "{function}"
        );
    }
    check_complete_matching(&report);
    check_aggregate_documents(&report);
    for function in functions.iter().filter(|function| {
        matches!(
            function["facts"]["name"].as_str(),
            Some("skip_ignore" | "skip_initial_padding")
        )
    }) {
        reject_scanner_document_changes(function);
    }
    for function in functions
        .iter()
        .filter(|function| function["outcome"] == "admitted")
    {
        outside::reject_cyclic_site_mutations(function);
    }
}

#[test]
fn cyclic_prefix_runtime_oracle_retains_independent_scan_expectations() {
    let directory = tempfile::tempdir().unwrap();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/cyclic-prefix-scans.rs"),
        include_str!("../../../../eval/coverage-development/cyclic-prefix-oracle.rs")
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

fn reject_scanner_document_changes(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["bounds", "read", "operand"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        let node = pair
            .pdg
            .nodes
            .iter_mut()
            .find(|node| node.annotations.contains_key("rstyle.data:index-bounds"))
            .unwrap();
        match mutation {
            "bounds" => {
                node.annotations.remove("rstyle.data:index-bounds");
            }
            "read" => node.attributes.reads.clear(),
            _ => {
                let edge = pair
                    .pdg
                    .edges
                    .iter_mut()
                    .find(|edge| {
                        edge.target == node.id
                            && edge.subkind == "value_operand"
                            && edge.operand_position == Some(1)
                    })
                    .unwrap();
                edge.operand_position = Some(0);
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

fn check_complete_matching(report: &Value) {
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
