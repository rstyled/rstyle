use super::*;
mod boundaries;
mod corruption;

#[test]
fn accelerator_packing_composes_through_strict_capture_and_matching() {
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
            check_context(target, no_std);
        }
    }
}

fn check_context(target: &str, no_std: bool) {
    let (_directory, report) = captured(Some(target), no_std);
    check_family(&report, target);
}

fn check_family(report: &Value, target: &str) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 19);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let expected = if matches!(
            name,
            "Accel::len"
                | "Accel::as_accel_tys"
                | "getter"
                | "first"
                | "saved"
                | "selected"
                | "scalar_alias"
                | "array_alias"
                | "compare_order"
                | "debug_order"
                | "saved_result"
                | "raw"
                | "raw_caller"
        ) {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(
            function["outcome"], expected,
            "{target} {name}: {}",
            function["reasons"]
        );
    }
    check_read_contract(functions);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{target} {flag}");
    }
    check_aggregate_documents(report);
}

fn captured(target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    let source =
        include_str!("../../../../tests/fixtures/function-families/accelerator-packing.rs");
    let source = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let mut args = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        args.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &args);
    assert!(
        output.status.success(),
        "{}; {}",
        String::from_utf8_lossy(&output.stderr),
        report["matching"]["diagnostics"]
    );
    (directory, report)
}

fn check_read_contract(functions: &[Value]) {
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let count = match name {
            "Accel::as_accel_tys" | "getter" | "first" | "saved" => 2,
            "selected" => 4,
            _ => continue,
        };
        let (unit, index) = data_unit(function);
        let pair = unit.pair(index).unwrap();
        let reads: Vec<_> = pair
            .pdg
            .nodes
            .iter()
            .flat_map(|node| &node.attributes.reads)
            .filter(|read| read.location_id.is_some())
            .collect();
        assert_eq!(reads.len(), count, "{name}");
        assert_eq!(
            function["extraction_evidence"]["cfg"]["status"], "partial",
            "{name}"
        );
        for read in reads {
            let location = pair
                .pdg
                .memory_locations
                .iter()
                .find(|location| Some(&location.id) == read.location_id.as_ref())
                .unwrap();
            assert_eq!(
                location.abstraction, "rstyle.formal-byte-window/1",
                "{name}"
            );
            let evidence = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY];
            assert_eq!(evidence["type"]["value"]["length"], 4, "{name}");
            if name != "selected" {
                assert_eq!(evidence["parameter"], 0, "{name}");
            }
        }
    }
}

#[test]
fn packing_release_capture_omits_debug_comparison_operand_effects() {
    let (directory, _) = captured(None, false);
    let (output, report) = clones(
        directory.path(),
        &["--release", "--extraction-profile", "hir-data-v3"],
    );
    assert!(
        output.status.success(),
        "{}",
        report["matching"]["diagnostics"]
    );
    check_family(&report, "release");
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "debug_order")
        .unwrap();
    let (unit, index) = data_unit(function);
    let pair = unit.pair(index).unwrap();
    assert!(
        pair.pdg
            .nodes
            .iter()
            .all(
                |node| node.attributes.construct.as_deref() != Some("core::debug_assert_eq")
                    && node.original_kind.known_value() != Some("expr_assign_op")
            )
    );
}
