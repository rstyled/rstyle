use super::*;

#[test]
fn compact_reports_preserve_all_outcomes_pairs_and_strict_bundle_evidence() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn first(x: u8) -> u8 { x }\npub fn same(x: u8) -> u8 { x }\npub fn unsupported() { unsafe { core::ptr::read_volatile(&0); } }\n",
    )
    .unwrap();
    for profile in ["r0", "hir-data-v3"] {
        let (output, full) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success());
        let (output, compact) = clones(
            directory.path(),
            &[
                "--extraction-profile",
                profile,
                "--report-detail",
                "compact",
            ],
        );
        assert!(output.status.success());
        assert_eq!(compact["report_detail"], "compact");
        assert_persisted_report(&compact);
        assert_eq!(full["matching"], compact["matching"]);
        assert_eq!(compact["functions"].as_array().unwrap().len(), 3);
        check_functions(&full, &compact, profile);
        for flag in [
            "capture_complete",
            "assessment_complete",
            "pair_enumeration_complete",
            "pair_evaluation_complete",
        ] {
            assert_eq!(compact[flag], true, "{flag}");
        }
        assert!(output.stdout.len() * 2 < serde_json::to_vec(&full).unwrap().len());
    }
}

fn check_functions(full: &Value, compact: &Value, profile: &str) {
    for (full, reference) in full["functions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(compact["functions"].as_array().unwrap())
    {
        for key in ["key", "compilation_unit", "outcome", "reasons"] {
            assert_eq!(reference[key], full[key], "{key}");
        }
        assert!(reference.get("facts").is_none());
        let (unit, index) = data_unit(reference);
        let facts = &unit.functions()[index];
        assert_eq!(reference["name"], facts.name);
        assert_eq!(
            reference["fn_span"],
            serde_json::to_value(facts.fn_span).unwrap()
        );
        assert_eq!(reference["source_digest"], facts.source.digest);
        let retained = serde_json::to_value(facts).unwrap();
        for key in ["source", "signature", "bindings", "body"] {
            assert_eq!(full["facts"][key], retained[key], "{key}");
        }
        if reference["outcome"] == "admitted" {
            check_strict(&unit, index, profile);
        } else {
            assert_eq!(reference["outcome"], "excluded");
            assert_eq!(reference["name"], "unsupported");
        }
    }
}

fn check_strict(unit: &rstyle_pdg::unit_store::UnitStore, index: usize, profile: &str) {
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    if profile == "hir-data-v3" {
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    }
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
    rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
}

#[test]
fn compact_report_retains_pair_budget_incompleteness() {
    let directory = clone_workspace();
    let (output, report) = clones(
        directory.path(),
        &["--report-detail", "compact", "--max-pairs", "0"],
    );
    assert!(!output.status.success());
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["pair_enumeration_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert_eq!(report["functions"].as_array().unwrap().len(), 4);
    assert_persisted_report(&report);
}
