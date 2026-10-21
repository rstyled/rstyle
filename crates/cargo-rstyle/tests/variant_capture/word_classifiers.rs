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
                "Unit::new_byte"
                    | "Unit::new_end"
                    | "Unit::byte"
                    | "scalar"
                    | "table::is_word_byte"
                    | "word_byte"
                    | "Unit::is_word_byte"
                    | "word_unit"
                    | "saved"
                    | "empty_default"
                    | "named"
                    | "named_none"
                    | "aliased"
            )
        } else {
            name != "byte_string"
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

pub(super) fn verify_admissions(report: &Value) -> rstyle_pdg::unit_store::UnitStore {
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
    drop(analysis);
    unit
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

#[test]
fn const_table_values_and_documents_agree_with_independent_ascii_membership() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), TABLE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "table::is_word_byte")
        .unwrap();
    assert_eq!(function["outcome"], "admitted", "{}", function["reasons"]);
    let (unit, index) = data_unit(function);
    let captured = unit.pair(index).unwrap();
    check_ascii_table_value(captured);
    check_table_documents(&unit, index);
}

fn check_ascii_table_value(captured: &rstyle_pdg::producer::Pair) {
    let node = captured
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("constant.bool-array"))
        .unwrap();
    let values: Vec<bool> = serde_json::from_str(
        node.attributes
            .literal
            .as_ref()
            .unwrap()
            .known_value()
            .unwrap(),
    )
    .unwrap();
    let alphabet = [0x03ff000000000000u64, 0x07fffffe87fffffe, 0, 0];
    assert_eq!(values.len(), 256);
    for (byte, value) in values.iter().enumerate() {
        assert_eq!(
            *value,
            alphabet[byte / 64] & (1 << (byte % 64)) != 0,
            "{byte}"
        );
    }
}

fn check_table_documents(unit: &rstyle_pdg::unit_store::UnitStore, index: usize) {
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let captured = unit.pair(index).unwrap();
    for mutation in [
        "value",
        "truncated",
        "type",
        "effect",
        "operand",
        "category",
        "policy",
    ] {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: captured.pdg.clone(),
            cfg: captured.cfg.clone(),
        };
        corrupt_table(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
    for mutation in [
        "initializer",
        "owner",
        "declaration",
        "missing",
        "use_value",
    ] {
        let mut forged = unit.functions()[index].clone();
        corrupt_table_facts(&mut forged, mutation);
        assert!(
            rstyle_pdg::data_frontend::verify(&forged).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt_table(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("constant.bool-array"))
        .unwrap();
    match mutation {
        "value" => {
            let literal = node.attributes.literal.as_mut().unwrap();
            let mut values: Vec<bool> =
                serde_json::from_str(literal.known_value().unwrap()).unwrap();
            values[95] = !values[95];
            literal.value = Some(serde_json::to_string(&values).unwrap());
        }
        "truncated" => node.attributes.literal.as_mut().unwrap().value = Some("[]".into()),
        "type" => {
            node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "bool".into()
        }
        "effect" => node.attributes.operator = Some(rstyle_pdg::StringFact::known("invented")),
        "operand" => {
            let mut edge = pair
                .pdg
                .edges
                .iter()
                .find(|edge| edge.subkind == "value_operand")
                .unwrap()
                .clone();
            edge.target = node.id.clone();
            pair.pdg.edges.push(edge);
        }
        "category" => node.category = rstyle_pdg::NodeCategory::Read,
        "policy" => {
            pair.pdg.profile.version = "3.55.0".into();
            pair.cfg.profile.version = "3.55.0".into();
        }
        _ => unreachable!(),
    }
}

fn corrupt_table_facts(facts: &mut rstyle_pdg::HirFnFacts, mutation: &str) {
    if mutation == "missing" {
        facts
            .body
            .children
            .retain(|node| node.kind != "stmt_const_array_fn");
        return;
    }
    let node = if mutation == "use_value" {
        facts
            .body
            .children
            .iter_mut()
            .find(|node| node.kind == "expr_index")
            .unwrap()
            .children
            .first_mut()
            .unwrap()
    } else {
        facts
            .body
            .children
            .iter_mut()
            .find(|node| node.kind == "stmt_bool_array")
            .unwrap()
    };
    let resolution = node.res.as_mut().unwrap();
    let mut record: Value = serde_json::from_str(&resolution.description).unwrap();
    match mutation {
        "initializer" => record["initializer"] = "unrelated::mkwordset".into(),
        "owner" => record["definition"] = "other::WORD".into(),
        "declaration" => record["declaration"] = "const WORD: [bool; 256] = unrelated();".into(),
        "use_value" => record["values"][0] = true.into(),
        _ => unreachable!(),
    }
    resolution.description = serde_json::to_string(&record).unwrap();
}

#[test]
fn lossless_cast_bounds_compose_with_helpers_without_signed_or_rebound_assumptions() {
    let directory = clone_workspace();
    let source = r#"
        pub fn leaf(a: [u8; 256], byte: u8) -> u8 { a[byte as usize] }
        pub fn caller(a: [u8; 256], byte: u8) -> u8 { leaf(a, byte) }
        pub fn saved(a: [u8; 256], byte: u8) -> u8 { let i = byte as usize; a[i] }
        pub fn saved_caller(a: [u8; 256], byte: u8) -> u8 { saved(a, byte) }
        pub fn short(a: [u8; 255], byte: u8) -> u8 { a[byte as usize] }
        pub fn short_caller(a: [u8; 255], byte: u8) -> u8 { short(a, byte) }
        pub fn signed(a: [u8; 256], byte: i8) -> u8 { a[byte as usize] }
        pub fn signed_caller(a: [u8; 256], byte: i8) -> u8 { signed(a, byte) }
        pub fn wide(a: [u8; 256], byte: u16) -> u8 { a[byte as usize] }
        pub fn wide_caller(a: [u8; 256], byte: u16) -> u8 { wide(a, byte) }
        pub fn rebound(a: [u8; 256], byte: u8, other: u16) -> u8 {
            let mut i = byte as usize; i = other as usize; a[i]
        }
        pub fn rebound_caller(a: [u8; 256], byte: u8, other: u16) -> u8 { rebound(a, byte, other) }
    "#;
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let admitted = !matches!(
            name,
            "short_caller" | "signed_caller" | "wide_caller" | "rebound_caller"
        );
        assert_eq!(
            function["outcome"],
            if admitted { "admitted" } else { "excluded" },
            "{name}: {}",
            function["reasons"]
        );
        if admitted {
            assert_eq!(
                function["extraction_evidence"]["cfg"]["status"],
                if matches!(name, "short" | "signed" | "wide" | "rebound") {
                    "partial"
                } else {
                    "complete"
                },
                "{name}"
            );
        }
    }
    verify_admissions(&report);
}
