//! The fixed source-selected family through real Cargo and strict verification.
use super::*;
mod boundaries;
mod corruption;

fn captured(no_std: bool, target: Option<&str>) -> (TempDir, Value) {
    let directory = clone_workspace();
    let corpus =
        include_str!("../../../../tests/fixtures/function-families/native-prefix-readers.rs");
    if no_std {
        std::fs::write(
            directory.path().join("src/lib.rs"),
            "#![no_std]\npub mod reader;\n",
        )
        .unwrap();
        std::fs::write(directory.path().join("src/reader.rs"), corpus).unwrap();
    } else {
        std::fs::write(directory.path().join("src/lib.rs"), corpus).unwrap();
    }
    let mut args = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        args.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &args);
    assert!(output.status.success(), "{report}");
    (directory, report)
}

#[test]
fn unsigned_prefix_reader_family_composes_on_all_installed_ci_targets() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        if !Path::new(String::from_utf8(output.stdout).unwrap().trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            let (_directory, report) = captured(no_std, Some(target));
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 11);
    for function in functions {
        let name = function["facts"]["name"]
            .as_str()
            .unwrap()
            .rsplit("::")
            .next()
            .unwrap();
        assert_eq!(
            function["outcome"],
            match name {
                "signed" => "excluded",
                "caller_u16" | "read_u16" | "read_u32" | "read_u128" | "native_u16" | "choose"
                | "selected_u16" | "saved_u16" | "snapshot_u16" | "suffix" => "admitted",
                _ => panic!("Unexpected function outside the declared selection: {name}"),
            },
            "{function}"
        );
        if let Some(length) = match name {
            "read_u16" => Some(2),
            "read_u32" => Some(4),
            "read_u128" => Some(16),
            _ => None,
        } {
            check_reader_window(function, length);
        }
        if matches!(name, "snapshot_u16" | "saved_u16") {
            check_saved_base(function);
        }
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 45);
    assert!(
        pairs
            .iter()
            .all(|pair| pair["evaluation_status"] == "evaluated")
    );
    check_aggregate_documents(report);
}

fn check_saved_base(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let indexed = find_kind(&facts.body, "expr_index").unwrap();
    let roots = rstyle_pdg::data_origins::origins(&indexed.children[0], &facts).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].parameter, 0);
}

#[test]
fn prefix_and_scalar_size_do_not_extend_historical_data_capture() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), "pub fn width()->usize { core::mem::size_of::<u16>() }\npub fn prefix(a:&[u8], n:usize)->&[u8] { &a[..n] }\n").unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}

fn check_reader_window(function: &Value, length: u64) {
    let (unit, index) = data_unit(function);
    let pair = unit.pair(index).unwrap();
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    let location = &pair.pdg.memory_locations[0];
    assert_eq!(location.abstraction, "rstyle.formal-byte-window/1");
    let record = &location.extensions[rstyle_pdg::data_documents::LOCATION_KEY];
    assert_eq!(record["parameter"], 0);
    assert_eq!(
        record["type"],
        serde_json::json!({
            "kind":"array", "value":{"element":{"kind":"scalar","value":"u8"},"length":length}
        })
    );
    assert_eq!(
        record["path"].as_array().unwrap().last().unwrap(),
        &serde_json::json!({
            "kind":"ByteWindow", "operand_position":0, "length":length
        })
    );
    let copy = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("data.bytes.copy-result"))
        .unwrap();
    assert_eq!(copy.attributes.reads.len(), 1);
    assert_eq!(copy.annotations["rstyle.data:byte-call"]["reads_on"], "ok");
    let unwrap = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("data.bytes.unwrap"))
        .unwrap();
    assert_eq!(
        unwrap.annotations["rstyle.data:byte-call"]["normal_completion_requires"],
        "ok"
    );
}
