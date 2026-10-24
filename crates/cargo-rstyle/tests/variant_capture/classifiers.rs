use super::*;

#[test]
fn classifiers_connect_complete_constructor_projection_and_caller_chains() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/tagged-units.rs"),
        include_str!("../../../../eval/coverage-development/tagged-unit-classifiers.rs")
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 23);
    for function in functions {
        let expected_positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Unit::new_byte"
                | "Unit::new_end"
                | "Unit::byte"
                | "Unit::end"
                | "roundtrip"
                | "saved"
                | "Unit::code"
                | "Unit::is_end"
                | "byte_code"
                | "end_code"
                | "byte_is_end"
                | "end_is_end"
                | "narrowing"
        );
        if function["facts"]["name"] == "narrowing" {
            assert_eq!(function["extraction_evidence"]["cfg"]["status"], "partial");
        }
        assert_eq!(
            function["outcome"],
            if expected_positive {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
}

#[test]
fn classifier_summaries_reject_changed_semantics_and_operands() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn widen(x: u16) -> usize { usize::from(x) }\n\
         pub fn tag(x: Option<u16>) -> bool { x.is_some() }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "admitted");
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        for mutation in ["summary", "operand", "policy"] {
            let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
            corrupt_classifier(&mut pair, mutation);
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
}

fn corrupt_classifier(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    match mutation {
        "summary" => {
            for extensions in [&mut pair.pdg.extensions, &mut pair.cfg.extensions] {
                extensions.get_mut(rstyle_pdg::call_summaries::KEY).unwrap()[0]["summary"]["may_panic"] =
                    true.into();
            }
        }
        "operand" => {
            pair.pdg
                .edges
                .retain(|edge| edge.subkind != "value_operand");
        }
        "policy" => {
            pair.pdg.profile.version = "3.47.0".into();
            pair.cfg.profile.version = "3.47.0".into();
        }
        _ => unreachable!(),
    }
}

#[test]
fn classifier_library_authority_is_pinned_across_registered_targets() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn widen(x: u16) -> usize { usize::from(x) }\n\
         pub fn tag(x: Option<u16>) -> bool { x.is_some() }",
    )
    .unwrap();
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            eprintln!("target std not installed: {target}");
            continue;
        }

        let (output, report) = clones(
            directory.path(),
            &["--extraction-profile", "hir-data-v3", "--target", target],
        );
        assert!(output.status.success(), "{report}");
        for function in report["functions"].as_array().unwrap() {
            assert_eq!(function["outcome"], "admitted", "{target}: {function}");
        }
        check_aggregate_documents(&report);
    }
}
