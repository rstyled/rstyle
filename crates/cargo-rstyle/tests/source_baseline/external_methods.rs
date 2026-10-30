use super::*;

#[test]
fn external_method_source_baselines_keep_three_families_across_layout_and_replay() {
    check_baselines(&format!("pub struct Unit;\nimpl Unit {{\n{ORIGINAL}}}\n"));
}

#[test]
fn external_lifetime_owner_baselines_keep_three_numeric_families_in_strict_replay() {
    check_baselines(&format!(
        "pub struct Unit<'a>(pub &'a u32);\nimpl Unit<'_> {{\n{ORIGINAL}}}\n"
    ));
}

fn check_baselines(source: &str) {
    let directory = workspace("pub mod external;\n");
    std::fs::write(directory.path().join("src/external.rs"), source).unwrap();
    let (output, _) = requested(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    checked(&output, 0);
    let saved = stored(directory.path());
    let entries = saved["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    for entry in entries {
        assert_eq!(entry["finding"]["locator"]["source"], "src/external.rs");
        assert_eq!(
            entry["finding"]["locator"]["qualified_name"],
            "crate::external::Unit::many"
        );
        assert_eq!(entry["finding"]["locator"]["declaration_kind"], "method");
    }

    std::fs::write(
        directory.path().join("src/external.rs"),
        format!("// Move the original method.\n\n{source}"),
    )
    .unwrap();
    let (output, moved) = requested(directory.path(), BASELINE);
    checked(&output, 0);
    counts(&moved, 3, 0);
    assert_eq!(moved["analysis"]["findings"], 3);
    for path in ["src/lib.rs", "src/external.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay(
            directory.path(),
            &moved,
            &[
                "--source-baseline",
                "baseline.json",
                "--fail-on-new-source",
                "--report-detail",
                detail,
            ],
        );
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, 3, 0);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["analysis"], moved["analysis"]);
    }
    assert_eq!(stored(directory.path()), saved);
}
