use super::*;

const RAW: &[&str] = &["--compiler-advisory-only"];
const WORKSPACE_CONFIG: &str = include_str!("../../../../rstyle.toml");

fn full_config() -> String {
    WORKSPACE_CONFIG
        .replace("max_lines = 50", "max_lines = 2")
        .replace("max_params = 4", "max_params = 1")
}

fn configure(directory: &Path, text: &str) {
    std::fs::write(directory.join("analysis.toml"), text).unwrap();
}

fn capture(directory: &Path, extra: &[&str]) -> Value {
    let mut flags = RAW.to_vec();
    flags.extend_from_slice(extra);
    let (output, report) = requested(directory, &flags);
    checked(&output, 0);
    source_only(&report);
    report
}

fn source_only(report: &Value) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    for field in [
        "compiler_advisory",
        "complexity_ranking",
        "parameter_planning",
        "guard_planning",
    ] {
        assert!(report.get(field).is_none(), "{field}: {}", report[field]);
    }
    for field in ["mutation_analysis", "mutation_effects", "guard_review"] {
        assert_eq!(report[field]["execution"], "not_requested", "{field}");
        assert_eq!(report[field]["findings"], 0, "{field}");
    }
    assert!(
        report["analysis"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|owner| {
                owner["assessments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|assessment| {
                        matches!(
                            assessment["rule_id"].as_str(),
                            Some("fn_length" | "fn_params" | "fn_spacing")
                        )
                    })
            })
    );
}

fn dismissed(directory: &Path, original: &Value) -> Value {
    let output = replay(
        directory,
        original,
        &[
            "--compiler-advisory-only",
            "--review-file",
            "reviews.json",
            "--dismiss-parameter",
            original["analysis"]["functions"][0]["key"]
                .as_str()
                .unwrap(),
            "--review-reason",
            "Reviewed source rule under the shared workspace configuration",
        ],
    );
    checked(&output, 0);
    let report = parsed(&output);
    assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    report
}

#[test]
fn full_workspace_configuration_applies_source_thresholds_without_requesting_other_analyses() {
    let directory = workspace(ORIGINAL);
    let aggressive = full_config()
        .replace("max_cyclomatic = 40", "max_cyclomatic = 0")
        .replace("max_cognitive = 32", "max_cognitive = 0")
        .replace(
            "max_mutations_per_target = 3",
            "max_mutations_per_target = 0",
        )
        .replace("max_mutated_targets = 3", "max_mutated_targets = 0")
        .replace("max_density_milli = 500", "max_density_milli = 0");
    configure(directory.path(), &aggressive);
    let report = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    assert_eq!(report["analysis"]["findings"], 3);
    let settings = &report["analysis"]["effective_config"];
    assert_eq!(settings["max_lines"], 2);
    assert_eq!(settings["max_params"], 1);
    assert_eq!(settings["min_statements"], 5);
    let entries = stored(directory.path())["entries"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|entry| matches!(
        entry["rule"].as_str(),
        Some("length" | "parameter" | "spacing")
    )));
}

#[test]
fn subset_and_full_configuration_share_source_baseline_and_review_identity() {
    let directory = workspace(ORIGINAL);
    let original = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    let baseline = std::fs::read(directory.path().join("baseline.json")).unwrap();
    configure(directory.path(), &full_config());
    let output = replay(directory.path(), &original, &[RAW, BASELINE].concat());
    checked(&output, 0);
    let full = parsed(&output);
    assert_eq!(full["analysis"], original["analysis"]);
    counts(&full, 3, 0);
    assert_eq!(full["source_baseline"]["scope_changed"], false);
    dismissed(directory.path(), &original);
    let reviews = std::fs::read(directory.path().join("reviews.json")).unwrap();
    let changed = full_config()
        .replace("max_cyclomatic = 40", "max_cyclomatic = 1")
        .replace("max_density_milli = 500", "max_density_milli = 1")
        .replace("caller_hops = 1", "caller_hops = 2");
    configure(directory.path(), &changed);
    let mut flags = [RAW, BASELINE].concat();
    flags.extend(["--review-file", "reviews.json", "--review-status"]);
    let output = replay(directory.path(), &original, &flags);
    checked(&output, 0);
    let report = parsed(&output);
    counts(&report, 2, 0);
    source_only(&report);
    assert_eq!(report["source_baseline"]["scope_changed"], false);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        reviews
    );
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        baseline
    );
}

#[test]
fn shared_configuration_preserves_absent_explicit_and_disabled_spacing_requests() {
    let directory = workspace(ORIGINAL);
    let absent = full_config().replace("[fn_spacing]\nmin_statements = 5\n", "");
    configure(directory.path(), &absent);
    let original = capture(directory.path(), &[]);
    assert_eq!(original["analysis"]["findings"], 2);
    assert!(original["analysis"]["effective_config"]["min_statements"].is_null());
    assert_eq!(
        original["analysis"]["functions"][0]["assessments"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for (minimum, findings) in [(5, 3), (0, 2)] {
        configure(
            directory.path(),
            &full_config().replace(
                "min_statements = 5\n",
                &format!("min_statements = {minimum}\n"),
            ),
        );
        let output = replay(directory.path(), &original, RAW);
        checked(&output, 0);
        let report = parsed(&output);
        source_only(&report);
        assert_eq!(report["analysis"]["findings"], findings);
        let spacing = report["analysis"]["functions"][0]["assessments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|assessment| assessment["rule_id"] == "fn_spacing")
            .unwrap();
        assert_eq!(
            report["analysis"]["effective_config"]["min_statements"],
            minimum
        );
        if minimum == 0 {
            assert_eq!(spacing["availability"], "not_applicable");
            assert_eq!(spacing["outcome"], "evaluated_no_finding");
            assert!(spacing["actual"].is_null());
        } else {
            assert_eq!(spacing["outcome"], "finding");
            assert!(spacing["actual"].as_u64().unwrap() >= 5);
        }
    }
}

fn reject_without_tools(directory: &Path) -> Output {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory-only",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--write-source-baseline",
            "invalid.json",
        ])
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    diagnostic_process::output(&mut command).unwrap()
}

#[test]
fn shared_configuration_rejects_unknown_outer_and_nested_keys_before_capture() {
    let directory = workspace(ORIGINAL);
    for (text, expected) in [
        ("unexpected = true\n", "unexpected"),
        ("[fn_params]\nmax_param=1\n", "max_param"),
        ("[complexity]\nmax_cyclomati=1\n", "max_cyclomati"),
        ("[side_effects.weights]\nioo=3\n", "ioo"),
        (
            "[mutation_budget]\nmax_mutated_target=1\n",
            "max_mutated_target",
        ),
        ("[grouping]\ncaller_rules=['fn_param']\n", "fn_param"),
    ] {
        configure(directory.path(), text);
        let output = reject_without_tools(directory.path());
        checked(&output, 2);
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("no-cargo"), "{error}");
        assert!(!error.contains("no-driver"), "{error}");
        assert!(!directory.path().join("invalid.json").exists());
        assert!(!directory.path().join("target/rstyle/clones").exists());
    }
}

#[test]
fn full_configuration_source_baselines_and_reviews_replay_without_live_sources_or_tools() {
    let directory = workspace(ORIGINAL);
    configure(directory.path(), &full_config());
    let original = capture(
        directory.path(),
        &["--write-source-baseline", "baseline.json"],
    );
    let hidden = dismissed(directory.path(), &original);
    let reviews = std::fs::read(directory.path().join("reviews.json")).unwrap();
    let baseline = std::fs::read(directory.path().join("baseline.json")).unwrap();
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let mut flags = [RAW, BASELINE].concat();
        flags.extend(["--review-file", "reviews.json", "--report-detail", detail]);
        let output = replay(directory.path(), &original, &flags);
        checked(&output, 0);
        let report = parsed(&output);
        source_only(&report);
        counts(&report, 2, 0);
        assert_eq!(report["verification_policy"], "raw-replay");
        assert_eq!(report["analysis"], hidden["analysis"]);
        assert_eq!(report["parameter_review"]["hidden_findings"], 1);
    }
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        reviews
    );
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        baseline
    );
}
