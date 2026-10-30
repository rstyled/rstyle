use super::*;

const HELPERS: &str = "pub fn first(a:u128)->u128 {std::hint::black_box(a.wrapping_add(7)).wrapping_mul(3)} pub fn second(a:u128)->u128 {std::hint::black_box(a.wrapping_add(8)).wrapping_mul(3)}";
const ONE_CALL: &str = r#"
pub struct Dog { values: Vec<u32> }
pub struct Cat { values: Vec<u32> }
impl Dog { pub fn dogs(&self)->usize { self.values.len() } }
impl Cat { pub fn cats(&self)->usize { self.values.len() } }
"#;

pub(super) fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(super) fn saved(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
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
        ])
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{error}: {}\n{}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output, report)
}

pub(super) fn remove_sources(directory: &Path) {
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
}

#[test]
fn single_call_option_reuses_substantial_work_guard_and_saved_graphs() {
    let (directory, output, original) = capture(ONE_CALL, &["--compiler-patterns"]);
    checked(&output, 0);
    assert!(candidate_at(&proposals(&original)["evaluation"], "Dog::dogs").is_none());
    remove_sources(directory.path());
    let (output, guarded) = saved(directory.path(), &original, &["--single-call-guard"]);
    checked(&output, 0);
    assert_eq!(
        candidate_at(&proposals(&guarded)["evaluation"], "Dog::dogs")
            .unwrap_or_else(|| panic!("{}", proposals(&guarded)))["kind"],
        "generic_fn"
    );
    assert_eq!(proposals(&guarded)["policy"]["single_call_guard"], true);
    assert_eq!(guarded["matching"], original["matching"]);
    assert_eq!(guarded["accepted_pairs"], original["accepted_pairs"]);
    let (output, defaults) = saved(directory.path(), &original, &[]);
    checked(&output, 0);
    assert_eq!(proposals(&defaults), proposals(&original));
}

#[test]
fn hole_limit_changes_helper_outcome_and_all_mining_parameters_are_reported() {
    let (directory, output, original) = capture(HELPERS, &["--compiler-patterns"]);
    checked(&output, 0);
    assert_eq!(
        candidate_at(&proposals(&original)["evaluation"], "first").unwrap()["kind"],
        "parameterize"
    );
    remove_sources(directory.path());
    let (output, limited) = saved(directory.path(), &original, &["--max-holes", "0"]);
    checked(&output, 0);
    assert!(candidate_at(&proposals(&limited)["evaluation"], "first").is_none());
    let (output, configured) = saved(
        directory.path(),
        &original,
        &[
            "--threshold",
            "500",
            "--min-coverage",
            "500",
            "--pair-coverage",
            "700",
            "--normalize",
            "cmp,try",
            "--max-holes",
            "4",
            "--hide-implements",
            "unrelated",
        ],
    );
    checked(&output, 0);
    let patterns = proposals(&configured);
    let policy = &patterns["policy"];
    assert_eq!(policy["threshold_milli"], 500);
    assert_eq!(policy["min_coverage_milli"], 500);
    assert_eq!(policy["pair_coverage_milli"], 700);
    assert_eq!(policy["normalize"]["cmp"], true);
    assert_eq!(policy["normalize"]["propagate"], true);
    assert_eq!(policy["max_holes"], 4);
    assert_eq!(
        patterns["hide_implements"],
        serde_json::json!(["unrelated"])
    );
    assert_eq!(configured["matching"], original["matching"]);
}

fn reviewed_inputs(directory: &Path, original: &Value) -> Vec<u8> {
    let candidate = &proposals(original)["evaluation"]["candidates"][0];
    let (output, reviewed) = saved(
        directory,
        original,
        &[
            "--max-holes",
            "4",
            "--hide-implements",
            "unrelated",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-pattern",
            "0",
            candidate["id"].as_str().unwrap(),
            "--review-reason",
            "Independent option-sensitive fixture review",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        reviewed["compiler_advisory"]["pattern_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    let path = directory.join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    let store: Value = serde_json::from_slice(&bytes).unwrap();
    let finding = &store["compiler_pattern_entries"][0]["finding"];
    assert_eq!(finding["policy"]["max_holes"], 4);
    assert_eq!(finding["hide_implements"], serde_json::json!(["unrelated"]));
    bytes
}

#[test]
fn nondefault_mining_inputs_are_retained_and_changed_inputs_stale_reviews() {
    let (directory, output, original) = capture(
        HELPERS,
        &[
            "--compiler-patterns",
            "--max-holes",
            "4",
            "--hide-implements",
            "unrelated",
        ],
    );
    checked(&output, 0);
    let bytes = reviewed_inputs(directory.path(), &original);
    let path = directory.path().join("reviews.json");
    remove_sources(directory.path());
    for (holes, hidden, expected) in [
        ("4", "unrelated", "hidden_exact"),
        ("5", "unrelated", "stale"),
        ("4", "other", "stale"),
    ] {
        let (output, current) = saved(
            directory.path(),
            &original,
            &[
                "--max-holes",
                holes,
                "--hide-implements",
                hidden,
                "--review-file",
                "reviews.json",
                "--review-status",
            ],
        );
        checked(&output, 0);
        assert_eq!(
            current["compiler_advisory"]["pattern_review"]["decisions"][0]["state"],
            expected
        );
        assert_eq!(current["review_status"]["records"][0]["state"], expected);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

fn exported_inputs(directory: &Path, original: &Value) -> Vec<u8> {
    let (output, _) = saved(
        directory,
        original,
        &[
            "--max-holes",
            "4",
            "--hide-implements",
            "unrelated",
            "--write-compiler-pattern-baseline",
            "baseline.json",
        ],
    );
    checked(&output, 0);
    let bytes = std::fs::read(directory.join("baseline.json")).unwrap();
    let baseline: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(baseline["entries"], serde_json::json!([]));
    assert_eq!(baseline["proposal_policy"]["max_holes"], 4);
    assert_eq!(
        baseline["hide_implements"],
        serde_json::json!(["unrelated"])
    );
    bytes
}

#[test]
fn empty_baselines_keep_nondefault_mining_inputs_and_detect_parameter_or_hide_drift() {
    let (directory, output, original) = capture(
        "pub fn clean(a:u128)->u128 {a}",
        &[
            "--compiler-patterns",
            "--max-holes",
            "4",
            "--hide-implements",
            "unrelated",
        ],
    );
    checked(&output, 0);
    let bytes = exported_inputs(directory.path(), &original);
    remove_sources(directory.path());
    for (holes, hidden, changed) in [
        ("4", "unrelated", false),
        ("5", "unrelated", true),
        ("4", "other", true),
    ] {
        let (output, current) = saved(
            directory.path(),
            &original,
            &[
                "--max-holes",
                holes,
                "--hide-implements",
                hidden,
                "--compiler-pattern-baseline",
                "baseline.json",
                "--fail-on-new-compiler-patterns",
            ],
        );
        checked(&output, i32::from(changed));
        assert_eq!(
            current["compiler_advisory"]["pattern_baseline"]["scope_changed"],
            changed
        );
        assert_eq!(
            current["compiler_advisory"]["pattern_baseline"]["current"],
            serde_json::json!([])
        );
        assert_eq!(
            std::fs::read(directory.path().join("baseline.json")).unwrap(),
            bytes
        );
    }
}

#[test]
fn mining_options_require_the_compiler_pattern_consumer() {
    let directory = clone_workspace();
    for options in [
        vec!["--threshold", "500"],
        vec!["--min-coverage", "500"],
        vec!["--pair-coverage", "700"],
        vec!["--normalize", "cmp"],
        vec!["--single-call-guard"],
        vec!["--max-holes", "4"],
        vec!["--hide-implements", "unrelated"],
    ] {
        let mut args = vec!["clones", "--compiler-advisory"];
        args.extend(options);
        let output = rstyle(directory.path(), &args);
        checked(&output, 2);
        assert!(String::from_utf8_lossy(&output.stderr).contains("--compiler-patterns"));
    }
}

#[test]
fn trait_hiding_reuses_the_legacy_oracle_and_default_trait_suppression() {
    let (directory, output, original) = capture(ANIMALS, &["--compiler-patterns"]);
    checked(&output, 0);
    assert!(candidate_at(&proposals(&original)["evaluation"], "emit_cow").is_none());
    let (output, hidden) = saved(directory.path(), &original, &["--hide-implements", "Speak"]);
    checked(&output, 0);
    assert_eq!(
        candidate_at(&proposals(&hidden)["evaluation"], "emit_cow").unwrap()["kind"],
        "trait_method"
    );
    assert_eq!(
        proposals(&hidden)["evaluation"],
        patterns_json(directory.path(), &["--hide-implements", "Speak"])
    );
}
