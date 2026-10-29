use super::*;

const RAW: &[&str] = &["--compiler-advisory-only"];
const METHOD: &str = "fn default() -> Self {\n    let first = 1;\n    let second = first + 1;\n    let third = second + 1;\n    Self(third)\n}\n";

fn source(origin: &str) -> String {
    format!("pub struct Unit(pub u32);\nimpl {origin} for Unit {{\n{METHOD}}}\n")
}

fn fixture(external: bool, source: &str) -> TempDir {
    let directory = workspace(if external {
        "pub mod external;\n"
    } else {
        source
    });
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=2\n[fn_params]\nmax_params=100\n",
    )
    .unwrap();
    if external {
        std::fs::write(directory.path().join("src/external.rs"), source).unwrap();
    }
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> Value {
    let mut flags = RAW.to_vec();
    flags.extend_from_slice(extra);
    let (output, report) = requested(directory, &flags);
    checked(&output, 0);
    assert!(report.get("compiler_advisory").is_none());
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["matching_execution"], "not_requested");
    report
}

fn owner(report: &Value) -> &Value {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().contains("::default"))
        .unwrap()
}

fn dismiss(directory: &Path, original: &Value) -> Value {
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--dismiss-length",
            owner(original)["key"].as_str().unwrap(),
            "--review-reason",
            "Reviewed Default implementation length",
        ],
    );
    checked(&output, 0);
    let report = parsed(&output);
    assert_eq!(report["length_review"]["hidden_findings"], 1);
    report
}

fn review_store(directory: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(directory.join("reviews.json")).unwrap()).unwrap()
}

fn assert_opaque(directory: &Path, original: &Value) {
    let saved = review_store(directory);
    let locator = &saved["length_entries"][0]["finding"]["locator"];
    assert_eq!(locator["qualified_name"], owner(original)["name"]);
    assert_eq!(locator["declaration_kind"], "method");
    assert_eq!(
        stored(directory)["entries"][0]["finding"]["locator"],
        *locator
    );
}

fn source_free(directory: &Path, original: &Value) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    let external = directory.join("src/external.rs");
    if external.exists() {
        std::fs::remove_file(external).unwrap();
    }
    for detail in ["full", "compact"] {
        let mut flags = [RAW, BASELINE].concat();
        flags.extend(["--review-file", "reviews.json", "--report-detail", detail]);
        let output = replay(directory, original, &flags);
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, 0, 0);
        assert_eq!(report["verification_policy"], "raw-replay");
        assert!(report.get("compiler_advisory").is_none());
        assert_eq!(report["analysis"]["findings"], 1);
        assert_eq!(report["length_review"]["hidden_findings"], 1);
    }
}

#[test]
fn prelude_default_length_baseline_and_review_replay_without_live_sources_or_numeric_analysis() {
    for external in [false, true] {
        let directory = fixture(external, &source("Default"));
        let original = capture(
            directory.path(),
            &["--write-source-baseline", "baseline.json"],
        );
        assert_eq!(original["analysis"]["findings"], 1);
        assert_eq!(
            stored(directory.path())["entries"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        dismiss(directory.path(), &original);
        assert_opaque(directory.path(), &original);
        let before = std::fs::read(directory.path().join("reviews.json")).unwrap();
        source_free(directory.path(), &original);
        assert_eq!(
            std::fs::read(directory.path().join("reviews.json")).unwrap(),
            before
        );
    }
}

#[test]
fn explicit_and_import_aliased_default_traits_reuse_the_compiler_named_review() {
    for external in [false, true] {
        let directory = fixture(external, &source("core::default::Default"));
        let original = capture(
            directory.path(),
            &["--write-source-baseline", "baseline.json"],
        );
        dismiss(directory.path(), &original);
        let original_name = owner(&original)["name"].clone();
        let path = if external {
            "src/external.rs"
        } else {
            "src/lib.rs"
        };
        std::fs::write(
            directory.path().join(path),
            format!("use core::default::Default as Build;\n{}", source("Build")),
        )
        .unwrap();
        let changed = capture(
            directory.path(),
            &[
                "--review-file",
                "reviews.json",
                "--review-status",
                "--source-baseline",
                "baseline.json",
                "--fail-on-new-source",
            ],
        );
        assert_eq!(owner(&changed)["name"], original_name);
        assert_eq!(
            changed["review_status"]["records"][0]["state"],
            "hidden_exact"
        );
        counts(&changed, 0, 0);
        assert_opaque(directory.path(), &changed);
    }
}

#[test]
fn compiler_named_default_review_survives_impl_movement_and_stales_on_changed_length() {
    let original_source = source("Default");
    let directory = fixture(false, &original_source);
    let original = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    dismiss(directory.path(), &original);
    let prefix =
        "// Original implementation moved.\n\npub struct Other; impl Other { pub fn clean() {} }\n";
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{original_source}"),
    )
    .unwrap();
    let options = [
        "--review-file",
        "reviews.json",
        "--review-status",
        "--source-baseline",
        "baseline.json",
        "--fail-on-new-source",
    ];
    let moved = capture(directory.path(), &options);
    assert_eq!(
        moved["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
    counts(&moved, 0, 0);
    let changed = original_source.replace(
        "    Self(third)",
        "    let fourth = third + 1;\n    let fifth = fourth + 1;\n    Self(fifth)",
    );
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{changed}"),
    )
    .unwrap();
    let mut flags = RAW.to_vec();
    flags.extend(options);
    let (output, stale) = requested(directory.path(), &flags);
    checked(&output, 1);
    assert_eq!(owner(&stale)["name"], owner(&original)["name"]);
    assert_eq!(stale["review_status"]["records"][0]["state"], "stale");
    assert_eq!(stale["length_review"]["hidden_findings"], 0);
    counts(&stale, 0, 1);
}

fn raw_wire(report: &Value) -> Value {
    let bundle = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    serde_json::from_slice(&std::fs::read(bundle).unwrap()).unwrap()
}

fn unavailable(directory: &Path, original: &Value) {
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--write-source-baseline",
            "invalid.json",
        ],
    );
    checked(&output, 2);
    assert!(!directory.join("invalid.json").exists());
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "unavailable.json",
            "--dismiss-length",
            owner(original)["key"].as_str().unwrap(),
            "--review-reason",
            "Unavailable compiler named method",
        ],
    );
    checked(&output, 2);
    assert!(!directory.join("unavailable.json").exists());
}

fn prune_output(directory: &Path, original: &Value) -> Output {
    replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--review-status",
            "--review-prune",
            "--review-prune-dry-run",
        ],
    )
}

fn retained_unavailable(directory: &Path, original: &Value, saved: &[u8]) {
    let output = prune_output(directory, original);
    checked(&output, 2);
    let report = parsed(&output);
    assert_eq!(report["length_review"]["hidden_findings"], 0);
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(
        std::fs::read(directory.join("reviews.json")).unwrap(),
        saved
    );
}

fn retained_context_rejected(directory: &Path, original: &Value, saved: &[u8]) {
    let output = prune_output(directory, original);
    checked(&output, 2);
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains(
            "Review pruning requires a successful capture and assessment; no entries changed"
        ),
        "{error}"
    );
    assert_eq!(
        std::fs::read(directory.join("reviews.json")).unwrap(),
        saved
    );
}

#[test]
fn colliding_compiler_method_names_never_share_a_source_baseline_or_review() {
    let source = format!(
        "{}{}",
        source("Default"),
        source("Default").replace("Unit", "Other")
    );
    let directory = fixture(false, &source);
    let original = capture(directory.path(), &[]);
    assert_eq!(original["analysis"]["findings"], 2);
    dismiss(directory.path(), &original);
    let saved = std::fs::read(directory.path().join("reviews.json")).unwrap();
    let wire = raw_wire(&original);
    let mut collision = wire.clone();
    collision["facts"][1]["name"] = collision["facts"][0]["name"].clone();
    let ledger = super::super::migration_compiler_advisory::only::ledger_path(&original);
    super::super::migration_compiler_advisory::only::reseal_raw(&original, &ledger, &collision);
    unavailable(directory.path(), &original);
    retained_unavailable(directory.path(), &original, &saved);
    super::super::migration_compiler_advisory::only::reseal_raw(&original, &ledger, &wire);
    checked(&replay(directory.path(), &original, RAW), 0);
}

#[test]
fn missing_unstable_names_and_missing_context_remain_unavailable_without_numeric_fallback() {
    let directory = fixture(false, &source("Default"));
    let original = capture(directory.path(), &[]);
    dismiss(directory.path(), &original);
    let saved = std::fs::read(directory.path().join("reviews.json")).unwrap();
    let wire = raw_wire(&original);
    let ledger = super::super::migration_compiler_advisory::only::ledger_path(&original);
    for damage in ["empty", "unstable", "context"] {
        let mut changed = wire.clone();
        match damage {
            "empty" => changed["facts"][0]["name"] = "  ".into(),
            "unstable" => changed["facts"][0]["name"] = "{impl#0}::default".into(),
            _ => {
                changed["facts"][0]["build"]["frontend_options"]
                    .as_object_mut()
                    .unwrap()
                    .remove(rstyle_pdg::nominal_boundary::CONTEXT_KEY);
            }
        }
        super::super::migration_compiler_advisory::only::reseal_raw(&original, &ledger, &changed);
        unavailable(directory.path(), &original);
        if damage == "context" {
            retained_context_rejected(directory.path(), &original, &saved);
        } else {
            retained_unavailable(directory.path(), &original, &saved);
        }
    }
    replay_without_numeric(directory.path(), &original, wire);
}

fn replay_without_numeric(directory: &Path, original: &Value, mut source_only: Value) {
    let ledger = super::super::migration_compiler_advisory::only::ledger_path(original);
    for function in source_only["facts"].as_array_mut().unwrap() {
        function["build"]["frontend_options"]
            .as_object_mut()
            .unwrap()
            .remove(rstyle_core::advisory::KEY);
    }
    super::super::migration_compiler_advisory::only::reseal_raw(original, &ledger, &source_only);
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--write-source-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    let output = replay(
        directory,
        original,
        &["--compiler-advisory-only", "--review-file", "reviews.json"],
    );
    checked(&output, 0);
    let report = parsed(&output);
    assert_eq!(report["length_review"]["hidden_findings"], 1);
    assert_eq!(
        report["length_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    source_free(directory, original);
}

#[test]
fn old_incorrect_source_named_default_review_does_not_hide_or_prune_compiler_named_finding() {
    let directory = fixture(false, &source("Default"));
    let original = capture(directory.path(), &[]);
    dismiss(directory.path(), &original);
    let path = directory.path().join("reviews.json");
    let mut store = review_store(directory.path());
    store["length_entries"][0]["finding"]["locator"]["qualified_name"] =
        "crate::<Unit as Default>::default".into();
    let saved = serde_json::to_vec(&store).unwrap();
    std::fs::write(&path, &saved).unwrap();
    let output = replay(
        directory.path(),
        &original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--review-status",
            "--review-prune",
            "--review-prune-dry-run",
        ],
    );
    checked(&output, 2);
    let report = parsed(&output);
    assert_eq!(report["length_review"]["hidden_findings"], 0);
    assert_eq!(report["length_review"]["visible_findings"], 1);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "not_observed"
    );
    assert_eq!(report["review_prune"]["eligible_entries"], 0);
    assert_eq!(report["review_prune"]["removed_entries"], 0);
    assert_eq!(std::fs::read(path).unwrap(), saved);
}
