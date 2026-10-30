use super::*;

const SOURCE: &str =
    include_str!("../../../../tests/fixtures/function-families/ascii-boundaries.rs");

fn captured(target: &str, no_std: bool) -> (tempfile::TempDir, Value) {
    let directory = clone_workspace();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{SOURCE}"),
    )
    .unwrap();
    let mut arguments = vec!["--extraction-profile", "hir-data-v3"];
    if !target.is_empty() {
        arguments.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &arguments);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn ascii_boundary_selection_completes_nested_indexed_callers() {
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
            continue;
        }
        for no_std in [false, true] {
            let (_directory, report) = captured(target, no_std);
            check_outcomes(&report);
            super::word_classifiers::verify_admissions(&report);
        }
    }
}

fn check_outcomes(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 19);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let positive = matches!(
            name,
            "Matcher::get_line_terminator"
                | "Matcher::is_word_ascii"
                | "Matcher::is_word_ascii_negate"
                | "repeated"
                | "Matcher::is_word_start_ascii"
                | "Matcher::is_word_end_ascii"
                | "Matcher::is_word_start_half_ascii"
                | "Matcher::is_word_end_half_ascii"
                | "utf8::is_word_byte"
                | "getter"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{name}: {}",
            function["reasons"]
        );
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(
            report[flag], true,
            "{flag}: {}",
            report["matching"]["diagnostics"]
        );
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 45);
    assert!(
        report["matching"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn borrowed_nested_copy_requires_original_child_authority_and_document_policy() {
    let (_directory, report) = captured("", false);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "Matcher::get_line_terminator")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["missing", "authority", "derive", "outer-copy"] {
        let mut changed = facts.clone();
        corrupt_origins(&mut changed, mutation);
        assert!(
            rstyle_pdg::producer::produce(&changed).is_err(),
            "{mutation}"
        );
        shared_evidence::reject_facts(function, &changed);
    }
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    pair.pdg.profile = rstyle_pdg::extraction_profile::ExtractionProfile::DataV2.policy();
    pair.cfg.profile.name = pair.pdg.profile.id.clone();
    pair.cfg.profile.version = pair.pdg.profile.version.clone();
    let error = rstyle_pdg::data_documents::verify(&pair, &facts).unwrap_err();
    assert!(
        error.contains("Borrowed Copy child fields require"),
        "{error}"
    );
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn corrupt_origins(facts: &mut rstyle_pdg::HirFnFacts, mutation: &str) {
    let mut origins = serde_json::to_value(
        facts
            .build
            .frontend_options
            .data::<rstyle_pdg::declaration_origins::Origins>()
            .unwrap()
            .as_ref(),
    )
    .unwrap();
    let declarations = origins["declarations"].as_array_mut().unwrap();
    let child = declarations
        .iter_mut()
        .find(|origin| origin["builtin_copy"].is_object())
        .unwrap();
    match mutation {
        "missing" => child["builtin_copy"] = Value::Null,
        "authority" => {
            child["builtin_copy"]["trait_definition"]["definition_hash"] = "other".into()
        }
        "derive" => child["builtin_copy"]["derive_span"]["start"] = 0.into(),
        "outer-copy" => {
            let copy = child["builtin_copy"].clone();
            declarations
                .iter_mut()
                .find(|origin| origin["builtin_copy"].is_null())
                .unwrap()["builtin_copy"] = copy;
        }
        _ => unreachable!(),
    }
    facts
        .build
        .frontend_options
        .insert(rstyle_pdg::declaration_origins::KEY.into(), origins);
}

#[test]
fn borrowed_nested_copy_proof_is_independent_of_lexical_sort_order() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub struct A { first: Z, second: Z }
        #[derive(Clone, Copy)] pub struct Z(u8);
        impl A { pub fn read(&self) -> bool { self.first.0 == self.second.0 } }
        pub fn caller(a: &A, b: &A) -> bool { a.read() == b.read() }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    for function in report["functions"].as_array().unwrap() {
        let positive = matches!(
            function["facts"]["name"].as_str(),
            Some("A::read" | "caller")
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{}: {}",
            function["facts"]["name"],
            function["reasons"]
        );
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(report["pair_evaluation_complete"], true);
}

#[test]
fn ascii_boundary_selection_does_not_activate_historical_profiles() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 19);
    for function in functions {
        assert_eq!(
            function["outcome"], "excluded",
            "{}: {}",
            function["facts"]["name"], function["reasons"]
        );
    }
}
