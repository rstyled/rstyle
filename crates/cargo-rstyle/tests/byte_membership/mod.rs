use super::*;

#[test]
fn fixed_scalar_array_copy_owner_composes_through_bounded_getter_and_old_policy_boundary() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        #[derive(Clone, Copy)] pub struct ByteSet { bits: BitSet }
        #[derive(Clone, Copy)] struct BitSet([u128; 2]);
        impl ByteSet {
            pub fn empty() -> Self { ByteSet { bits: BitSet([0; 2]) } }
            pub fn contains(&self, byte: u8) -> bool {
                let bucket = byte / 128;
                let bit = byte % 128;
                self.bits.0[usize::from(bucket)] & (1 << bit) > 0
            }
        }
        pub fn caller(byte: u8) -> bool { ByteSet::empty().contains(byte) }
        "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    let constructor = functions
        .iter()
        .find(|f| f["facts"]["name"] == "ByteSet::empty")
        .unwrap();
    assert_eq!(constructor["outcome"], "admitted", "{constructor}");
    check_aggregate_documents(&report);
    let (unit, index) = data_unit(constructor);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    pair.pdg.profile.version = "3.45.0".into();
    pair.cfg.profile.version = "3.45.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
    for name in ["ByteSet::contains", "caller"] {
        let function = functions
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "admitted", "{function}");
        assert_eq!(
            function["extraction_evidence"]["cfg"]["status"], "complete",
            "{function}"
        );
    }
}

#[test]
fn array_fields_do_not_invent_copy_for_owned_destructors_or_shared_fields() {
    for source in [
        "pub struct Owner([u8; 2]); impl Drop for Owner { fn drop(&mut self) {} } pub fn make() -> Owner { Owner([0; 2]) }",
        "pub struct Owner([u8; 2]); pub fn make() -> Owner { Owner([0; 2]) }",
        "#[derive(Clone, Copy)] pub struct Owner([&'static u8; 2]); pub fn make(a: &'static u8) -> Owner { Owner([a; 2]) }",
    ] {
        let directory = clone_workspace();
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{source} enum Neighbor {{ A, B }}"),
        )
        .unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{report}");
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == "make")
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}

#[test]
fn concrete_u8_usize_from_requires_its_own_document_policy() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn widen(byte: u8) -> usize { usize::from(byte) }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "admitted", "{function}");
    check_aggregate_documents(&report);
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    reject_conversion_identity_changes(&facts);
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    pair.pdg.profile.version = "3.45.0".into();
    pair.cfg.profile.version = "3.45.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn reject_conversion_identity_changes(facts: &dyn rstyle_pdg::unit_store::EvidenceFacts) {
    for field in ["path", "crate_hash", "definition_hash", "missing"] {
        let mut changed = facts.facts().clone();
        let call = &mut changed.body.children[0];
        assert_eq!(call.kind, "expr_call");
        let resolution = call.res.as_mut().unwrap();
        let mut description: Value = serde_json::from_str(&resolution.description).unwrap();
        if field == "missing" {
            description
                .as_object_mut()
                .unwrap()
                .remove("implementation");
        } else {
            description["implementation"][field] = "unreviewed".into();
        }
        resolution.description = serde_json::to_string(&description).unwrap();
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{field}");
    }
}

#[test]
fn byte_membership_family_keeps_partial_and_unsupported_compositions_distinct() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/byte-membership.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 15);
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let complete = match name {
            "ByteSet::empty" | "ByteSet::contains" | "empty_contains" | "contains_words"
            | "contains_saved" => Some(true),
            "ByteSet::unbounded" | "rebound" | "merged" | "narrower" | "remainder"
            | "narrower_caller" | "remainder_caller" => Some(false),
            "unbounded_caller" => None,
            name if name.contains(" as ") && name.ends_with("::clone") => None,
            name => panic!("unexpected function {name}"),
        };
        if let Some(complete) = complete {
            assert_eq!(function["outcome"], "admitted", "{function}");
            assert_eq!(
                function["extraction_evidence"]["cfg"]["status"],
                if complete { "complete" } else { "partial" },
                "{function}"
            );
        } else {
            assert_eq!(function["outcome"], "excluded", "{function}");
        }
        if matches!(name, "narrower_caller" | "remainder_caller") {
            shared_evidence::check_partial_helper(function);
        }
        if name == "contains_words" {
            reject_membership_read_changes(function);
        }
    }
    check_aggregate_documents(&report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{report}");
    }
}

fn reject_membership_read_changes(function: &Value) {
    reject_helper(function, "effects");
    reject_helper(function, "source");
    for change in [
        "missing",
        "extra",
        "access",
        "parameter",
        "path",
        "type",
        "effect",
        "identity",
    ] {
        shared_evidence::reject_read_summary(function, change);
    }
}

#[test]
fn bounded_shift_policy_cannot_be_labeled_as_historical_constant_only_proof() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn bounded(byte: u8) -> u128 { 1u128 << (byte % 128) }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = &report["functions"][0];
    assert_eq!(
        function["extraction_evidence"]["cfg"]["status"], "complete",
        "{function}"
    );
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    pair.pdg.profile.version = "3.45.0".into();
    pair.cfg.profile.version = "3.45.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn byte_membership_composition_uses_each_installed_pinned_ci_target() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        if !library.status.success()
            || !Path::new(String::from_utf8_lossy(&library.stdout).trim()).exists()
        {
            continue;
        }
        let directory = clone_workspace();
        std::fs::write(
            directory.path().join("src/lib.rs"),
            include_str!("../../../../eval/coverage-development/byte-membership.rs"),
        )
        .unwrap();
        let (output, report) = clones(
            directory.path(),
            &["--target", target, "--extraction-profile", "hir-data-v3"],
        );
        assert!(output.status.success(), "{target}: {report}");
        for name in [
            "ByteSet::empty",
            "ByteSet::contains",
            "empty_contains",
            "contains_words",
            "contains_saved",
        ] {
            let function = report["functions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["facts"]["name"] == name)
                .unwrap();
            assert_eq!(function["outcome"], "admitted", "{target}: {function}");
            assert_eq!(
                function["extraction_evidence"]["cfg"]["status"], "complete",
                "{target}: {function}"
            );
        }
        check_aggregate_documents(&report);
        assert_eq!(report["pair_evaluation_complete"], true, "{report}");
    }
}
