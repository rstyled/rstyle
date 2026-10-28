use super::*;

const UPDATE: &str = "pub fn update(target:&mut u32){*target=1;}\n";
const COMPARE: &[&str] = &[
    "--density-baseline",
    "baseline.json",
    "--fail-on-new-density",
];

fn external(root: &str, sources: &[(&str, &str)]) -> TempDir {
    let directory = workspace(root, POLICY);
    for (path, source) in sources {
        let path = directory.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    directory
}

fn check(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "stdout: {}\nstderr: {}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn baseline_counts(report: &Value, existing: usize, new: usize) {
    let baseline = &report["density_review"]["baseline"];
    assert_eq!(baseline["coverage"], "complete", "{baseline}");
    assert_eq!(baseline["existing_findings"], existing, "{baseline}");
    assert_eq!(baseline["new_findings"], new, "{baseline}");
}

fn export(directory: &Path) -> Value {
    let (output, report) = requested(directory, &["--write-density-baseline", "baseline.json"]);
    check(&output, 0);
    report
}

fn retained(directory: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap()
}

fn replay_baseline(directory: &Path, original: &Value, extra: &[&str]) -> Value {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command.current_dir(directory).args([
        "clones",
        "--format",
        "json",
        "--extraction-profile",
        "hir-data-v3",
        "--effect-density",
        "--effect-config",
        "effects.toml",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ]);
    command
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    check(&output, 0);
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn external_module_density_baseline_survives_recapture_and_original_line_movement() {
    let directory = external("pub mod worker;\n", &[("src/worker.rs", UPDATE)]);
    export(directory.path());
    let stored = retained(directory.path());
    let entries = stored["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["locator"]["source"], "src/worker.rs");
    assert_eq!(
        entries[0]["locator"]["qualified_name"],
        "crate::worker::update"
    );
    for source in [
        UPDATE.to_owned(),
        format!("\n// moved inside its original module\n{UPDATE}"),
    ] {
        std::fs::write(directory.path().join("src/worker.rs"), source).unwrap();
        let (output, report) = requested(directory.path(), COMPARE);
        check(&output, 0);
        baseline_counts(&report, 1, 0);
        measured(&report, "worker::update", 1000, "finding");
    }
}

#[test]
fn external_module_density_baseline_rejects_changed_numeric_measurement() {
    let directory = external("pub mod worker;\n", &[("src/worker.rs", UPDATE)]);
    export(directory.path());
    std::fs::write(
        directory.path().join("src/worker.rs"),
        "pub fn update(target:&mut u32, other:&mut u32){*target=1;*other=2;}\n",
    )
    .unwrap();
    let (output, report) = requested(directory.path(), COMPARE);
    check(&output, 1);
    baseline_counts(&report, 0, 1);
    measured(&report, "worker::update", 1000, "finding");
    assert_eq!(
        report["density_review"]["baseline"]["current"][0]["state"],
        "changed"
    );
    assert_eq!(
        report["density_review"]["baseline"]["retained"][0]["state"],
        "changed"
    );
}

#[test]
fn external_sibling_and_mod_rs_inline_functions_have_distinct_durable_locations() {
    let directory = external(
        "pub mod worker; pub mod sibling; pub mod group;\n",
        &[
            ("src/worker.rs", UPDATE),
            ("src/sibling.rs", UPDATE),
            (
                "src/group/mod.rs",
                "pub mod nested { pub fn update(target:&mut u32){*target=1;} }\n",
            ),
        ],
    );
    let report = export(directory.path());
    for name in ["worker::update", "sibling::update", "group::nested::update"] {
        measured(&report, name, 1000, "finding");
    }
    let stored = retained(directory.path());
    let mut locations: Vec<_> = stored["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["locator"]["source"].as_str().unwrap(),
                entry["locator"]["qualified_name"].as_str().unwrap(),
            )
        })
        .collect();
    locations.sort();
    assert_eq!(
        locations,
        vec![
            ("src/group/mod.rs", "crate::group::nested::update"),
            ("src/sibling.rs", "crate::sibling::update"),
            ("src/worker.rs", "crate::worker::update"),
        ]
    );
    let (output, report) = requested(directory.path(), COMPARE);
    check(&output, 0);
    baseline_counts(&report, 3, 0);
}

#[test]
fn external_density_baselines_replay_without_any_original_source_or_compiler_tools() {
    let directory = external("pub mod worker;\n", &[("src/worker.rs", UPDATE)]);
    let original = export(directory.path());
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let mut arguments = COMPARE.to_vec();
        arguments.extend(["--report-detail", detail]);
        let report = replay_baseline(directory.path(), &original, &arguments);
        baseline_counts(&report, 1, 0);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(
            density(&report, "worker::update"),
            density(&original, "worker::update")
        );
        assert_replay_export(directory.path(), &original, detail);
    }
}

fn assert_replay_export(directory: &Path, original: &Value, detail: &str) {
    let name = format!("baseline-{detail}.json");
    replay_baseline(
        directory,
        original,
        &["--report-detail", detail, "--write-density-baseline", &name],
    );
    let report = replay_baseline(
        directory,
        original,
        &["--density-baseline", &name, "--fail-on-new-density"],
    );
    baseline_counts(&report, 1, 0);
}

#[test]
fn external_density_review_is_retained_after_original_owner_movement() {
    let directory = external("pub mod worker;\n", &[("src/worker.rs", UPDATE)]);
    let original = export(directory.path());
    let key = function(&original, "worker::update")["key"]
        .as_str()
        .unwrap();
    let report = replay_baseline(
        directory.path(),
        &original,
        &[
            "--review-file",
            "reviews.json",
            "--dismiss-density",
            key,
            "--review-reason",
            "Keep this explicit caller-visible write.",
        ],
    );
    assert_eq!(report["density_review"]["hidden_findings"], 1);
    std::fs::write(
        directory.path().join("src/worker.rs"),
        format!("\n\n{UPDATE}"),
    )
    .unwrap();
    let (output, report) = requested(
        directory.path(),
        &["--review-file", "reviews.json", "--fail-on-findings"],
    );
    check(&output, 0);
    assert_eq!(report["density_review"]["hidden_findings"], 1);
    assert_eq!(report["mutation_effects"]["findings"], 1);
    measured(&report, "worker::update", 1000, "finding");
}
