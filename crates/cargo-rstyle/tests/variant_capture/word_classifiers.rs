use super::*;

const TABLE: &str = include_str!("../../../../eval/coverage-development/word-byte-classifiers.rs");
const SCALAR: &str =
    include_str!("../../../../eval/coverage-development/byte-scalar-classifiers.rs");

#[test]
fn word_classifier_source_first_selections_keep_complete_blocker_chains() {
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
            check_context(target, no_std, TABLE);
            check_context(target, no_std, SCALAR);
        }
    }
}

fn check_context(target: &str, no_std: bool, source: &str) {
    let directory = clone_workspace();
    let bytes = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
    std::fs::write(directory.path().join("src/lib.rs"), bytes).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--target", target],
    );
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), if source == TABLE { 20 } else { 11 });
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let admitted = if source == TABLE {
            matches!(
                name,
                "Unit::new_byte" | "Unit::new_end" | "Unit::byte" | "scalar"
            )
        } else {
            !matches!(name, "characters" | "byte_string")
        };
        assert_eq!(
            function["outcome"],
            if admitted { "admitted" } else { "excluded" },
            "{target}, no_std={no_std}, {name}: {}",
            function["reasons"]
        );
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    verify_admissions(&report);
}

fn verify_admissions(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| function["outcome"] == "admitted")
    {
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
        rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
    }
}

#[test]
fn byte_scalar_documents_reject_changed_values_ranges_types_effects_and_policy() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SCALAR).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for name in ["escaped", "is_word_byte", "exhaustive"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        for mutation in ["literal", "type", "operator", "category", "policy"] {
            let captured = unit.pair(shared_evidence::index(function)).unwrap();
            let mut pair = rstyle_pdg::producer::Pair {
                pdg: captured.pdg.clone(),
                cfg: captured.cfg.clone(),
            };
            corrupt_byte(&mut pair, mutation);
            assert!(
                rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
                "{name}: {mutation}"
            );
            assert!(
                rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
                "{name}: {mutation}"
            );
        }
    }
}

fn corrupt_byte(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| {
            node.original_kind.known_value() == Some("expr_lit")
                && node.attributes.literal.as_ref().is_some_and(|literal| {
                    literal
                        .known_value()
                        .is_some_and(|literal| literal.starts_with("b'"))
                })
                || matches!(
                    node.attributes.construct.as_deref(),
                    Some("match.range" | "match.or-patterns")
                )
        })
        .unwrap();
    match mutation {
        "literal" => node.attributes.literal.as_mut().unwrap().value = Some("0".into()),
        "type" => {
            node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "u16".into()
        }
        "operator" => node.attributes.operator = Some(rstyle_pdg::StringFact::known("invented")),
        "category" => node.category = rstyle_pdg::NodeCategory::Read,
        "policy" => {
            pair.pdg.profile.version = "3.54.0".into();
            pair.cfg.profile.version = "3.54.0".into();
        }
        _ => unreachable!(),
    }
}
