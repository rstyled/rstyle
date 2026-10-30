use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";
const COMPARE: &[&str] = &[
    "--compiler-pattern-baseline",
    "baseline.json",
    "--fail-on-new-compiler-patterns",
];

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn baseline(report: &Value) -> &Value {
    &report["compiler_advisory"]["pattern_baseline"]
}
fn exported(source: &str) -> (TempDir, Value) {
    let (directory, output, original) = capture(source, &["--compiler-patterns"]);
    checked(&output, 0);
    let (output, report) = replay(
        directory.path(),
        &original,
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 0);
    assert_eq!(proposals(&report), proposals(&original));
    (directory, report)
}
fn current(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-patterns",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn raw(directory: &Path, original: Option<&Value>, extra: &[&str]) -> Output {
    let mut args = vec![
        "clones",
        "--format",
        "json",
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-patterns",
    ];
    if let Some(original) = original {
        args.extend(["--replay-run", original["run_directory"].as_str().unwrap()]);
    }
    args.extend_from_slice(extra);
    rstyle(directory, &args)
}

#[test]
fn proposal_baseline_covers_five_families_and_preserves_raw_reports() {
    for (source, expected) in [(FAMILIES, 3), (ANIMALS, 2)] {
        let (directory, original) = exported(source);
        let bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
        let saved: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(saved["schema"], "rstyle.compiler-pattern-baseline/2");
        assert_eq!(saved["entries"].as_array().unwrap().len(), expected);
        for forbidden in ["sha256", "fingerprint", "candidate_index", "candidate_id"] {
            assert!(
                !String::from_utf8_lossy(&bytes).contains(forbidden),
                "{forbidden}"
            );
        }
        let (output, report) = replay(directory.path(), &original, COMPARE);
        checked(&output, 0);
        assert_eq!(baseline(&report)["existing_findings"], expected);
        assert_eq!(baseline(&report)["new_findings"], 0);
        assert_eq!(
            report["compiler_advisory"]["units"],
            original["compiler_advisory"]["units"]
        );
        assert_eq!(report["matching"], original["matching"]);
        assert!(!directory.path().join("reviews.json").exists());
        let mut args = COMPARE.to_vec();
        args.push("--fail-on-findings");
        checked(&replay(directory.path(), &original, &args).0, 1);
    }
}

#[test]
fn proposal_baseline_replays_full_and_compact_without_sources_or_tools() {
    let (directory, original) = exported(HELPERS);
    for path in ["src/lib.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
        command
            .current_dir(directory.path())
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--compiler-advisory",
                "--compiler-patterns",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
                "--report-detail",
                detail,
            ])
            .args(COMPARE)
            .env("PATH", directory.path().join("no-tools"));
        let output = diagnostic_process::output(&mut command).unwrap();
        checked(&output, 0);
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(baseline(&report)["existing_findings"], 1);
        assert_eq!(report["matching"], original["matching"]);
    }
}

#[test]
fn movement_is_existing_changed_graphs_fail_and_absent_proposals_stay_unavailable() {
    let (directory, _) = exported(HELPERS);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!(
            "\n// moved\n{}",
            HELPERS
                .replace("a:u128", "renamed:u128")
                .replace("a.wrapping_add", "renamed.wrapping_add")
        ),
    )
    .unwrap();
    let (output, moved) = current(directory.path(), COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&moved)["existing_findings"], 1);
    std::fs::write(
        directory.path().join("src/lib.rs"),
        HELPERS.replace("wrapping_add(8)", "wrapping_add(9)"),
    )
    .unwrap();
    let (output, changed) = current(directory.path(), COMPARE);
    checked(&output, 1);
    assert_eq!(baseline(&changed)["current"][0]["state"], "changed");
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn first(a:u128)->u128 {a}",
    )
    .unwrap();
    let (output, absent) = current(directory.path(), COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&absent)["retained"][0]["state"], "unavailable");
    assert_eq!(baseline(&absent)["new_findings"], 0);
}

#[test]
fn empty_proposal_headers_detect_policy_and_compilation_context_drift() {
    let (directory, original) = exported("pub fn clean(a:u128)->u128 {a}");
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_length]\nmax_lines=99\n",
    )
    .unwrap();
    let mut args = COMPARE.to_vec();
    args.extend(["--compiler-advisory-config", "policy.toml"]);
    let (output, policy) = replay(directory.path(), &original, &args);
    checked(&output, 1);
    assert_eq!(baseline(&policy)["scope_changed"], true);
    assert_eq!(baseline(&policy)["current"], serde_json::json!([]));
    let mut args = COMPARE.to_vec();
    args.push("--release");
    let (output, release) = current(directory.path(), &args);
    checked(&output, 1);
    assert_eq!(baseline(&release)["scope_changed"], true);
}

#[test]
fn reviews_hide_new_proposals_but_export_keeps_raw_snapshots_and_selection_is_global() {
    let (directory, _) = exported("pub fn clean(a:u128)->u128 {a}");
    std::fs::write(directory.path().join("src/lib.rs"), HELPERS).unwrap();
    let (output, new) = current(directory.path(), COMPARE);
    checked(&output, 1);
    let candidate = &proposals(&new)["evaluation"]["candidates"][0];
    reviews::dismiss_candidate(directory.path(), &new, candidate, 0);
    std::fs::write(
        directory.path().join("selection.json"),
        "{\"schema\":\"rstyle.changed-files/1\",\"paths\":[]}",
    )
    .unwrap();
    let mut args = COMPARE.to_vec();
    args.extend(["--changed-files", "selection.json"]);
    checked(&replay(directory.path(), &new, &args).0, 1);
    args.extend(["--review-file", "reviews.json"]);
    let (output, hidden) = replay(directory.path(), &new, &args);
    checked(&output, 0);
    assert_eq!(baseline(&hidden)["new_findings"], 0);
    let (output, _) = replay(
        directory.path(),
        &new,
        &[
            "--review-file",
            "reviews.json",
            "--write-compiler-pattern-baseline",
            "raw.json",
        ],
    );
    checked(&output, 0);
    let raw: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("raw.json")).unwrap()).unwrap();
    assert_eq!(raw["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        proposals(&hidden)["evaluation"],
        proposals(&new)["evaluation"]
    );
}

#[test]
fn incomplete_generated_or_uncaptured_graph_inputs_cannot_export() {
    let (directory, output, generated) = capture(
        "macro_rules! make {()=>{pub fn generated(){}}} make!(); pub fn original(){}",
        &["--compiler-patterns"],
    );
    checked(&output, 2);
    let output = raw(
        directory.path(),
        Some(&generated),
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
    let (directory, _, missing) = capture(HELPERS, &[]);
    let (output, report) = replay(directory.path(), &missing, &[]);
    checked(&output, 2);
    assert_eq!(proposals(&report)["input_complete"], false);
    let output = raw(
        directory.path(),
        Some(&missing),
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert!(!directory.path().join("baseline.json").exists());
}

#[test]
fn check_test_proposals_remain_distinct_and_generated_harness_stays_incomplete() {
    let (directory, _) = exported(HELPERS);
    let mut args = COMPARE.to_vec();
    args.push("--all-targets");
    let (output, report) = current(directory.path(), &args);
    checked(&output, 2);
    let rows = baseline(&report)["current"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0]["key"]["unit"], rows[1]["key"]["unit"]);
    assert_eq!(baseline(&report)["coverage"], "incomplete");
}

#[test]
fn proposal_baselines_reject_invalid_snapshots_and_never_overwrite() {
    let (directory, original) = exported(HELPERS);
    let path = directory.path().join("baseline.json");
    let bytes = std::fs::read(&path).unwrap();
    let output = raw(
        directory.path(),
        Some(&original),
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 2);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    for change in 0..5 {
        let mut bad = original.clone();
        match change {
            0 => bad["entries"][0]["unknown"] = true.into(),
            1 => bad["entries"][0]["sites"][0]["body"]["graph"]["nodes"][0]["line"] = 1.into(),
            2 => {
                let duplicate = bad["entries"][0].clone();
                bad["entries"].as_array_mut().unwrap().push(duplicate);
            }
            3 => bad["contexts"] = serde_json::json!([]),
            _ => bad["proposal_policy"]["max_holes"] = 999.into(),
        }
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        checked(&raw(directory.path(), None, COMPARE), 2);
    }
    std::fs::write(&path, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    checked(&raw(directory.path(), None, COMPARE), 2);
}

#[test]
fn proposal_inventory_keeps_site_and_definition_roles_distinct_on_one_owner() {
    let (directory, original) = exported(HELPERS);
    let path = directory.path().join("baseline.json");
    let mut saved: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut alternate = saved["entries"][0].clone();
    let last = alternate["sites"].as_array_mut().unwrap().pop().unwrap();
    alternate["definitions"]
        .as_array_mut()
        .unwrap()
        .push(last["declaration"].clone());
    saved["entries"].as_array_mut().unwrap().push(alternate);
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let (output, report) = replay(directory.path(), &original, COMPARE);
    checked(&output, 0);
    assert_eq!(baseline(&report)["existing_findings"], 1);
    assert_eq!(baseline(&report)["retained"][0]["state"], "existing");
    assert_eq!(baseline(&report)["retained"][1]["state"], "unavailable");
}
