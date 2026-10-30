use super::*;

const ORIGINAL: &str = "pub fn nested(value:u32)->u32 { if value > 0 { let next = value + 1; next + 1 } else { 0 } }\n";
const FIRST: &str = "|first:u32| { if first > 0 { let next = first + 1; next + 1 } else { 0 } }";
const SECOND: &str =
    "|second:u32| { if second > 0 { let next = second + 2; next + 2 } else { 0 } }";
const OPTIONS: &[&str] = &[
    "--extraction-profile",
    "hir-data-v3",
    "--compiler-advisory",
    "--compiler-advisory-only",
    "--compiler-advisory-config",
    "compiler.toml",
    "--guard-clauses",
    "--guard-plans",
    "--review-file",
    "reviews.json",
];

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("compiler.toml"), "[fn_spacing]\nmin_statements=100\n[mutation_budget]\nmax_mutations_per_target=100\nmax_mutated_targets=100\n").unwrap();
    directory
}

fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = OPTIONS.to_vec();
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn key(report: &Value) -> &str {
    report["analysis"]["functions"][0]["key"].as_str().unwrap()
}

fn counts(report: &Value, hidden: usize, visible: usize) {
    let review = &report["guard_review"];
    assert_eq!(review["schema"], "rstyle.guard-review/1");
    assert_eq!(review["findings"], hidden + visible);
    assert_eq!(review["hidden_findings"], hidden);
    assert_eq!(review["visible_findings"], visible);
}

fn state(report: &Value, index: usize, expected: &str) {
    let decision = report["guard_review"]["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|decision| decision["key"] == key(report) && decision["opportunity_index"] == index)
        .unwrap();
    assert_eq!(decision["state"], expected);
}

fn unchanged(raw: &Value, reviewed: &Value) {
    for field in [
        "analysis",
        "guard_planning",
        "edit_coordination",
        "accepted_pairs",
        "functions",
    ] {
        assert_eq!(reviewed[field], raw[field], "{field}");
    }
    assert_eq!(reviewed["matching_execution"], "not_requested");
    assert_eq!(reviewed["strict_assessment_execution"], "not_requested");
}

struct Replay<'a> {
    directory: &'a Path,
    original: &'a Value,
}

impl Replay<'_> {
    fn command(&self, format: &str) -> Command {
        let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
        command
            .current_dir(self.directory)
            .args(["clones", "--format", format])
            .args(OPTIONS)
            .args([
                "--replay-run",
                self.original["run_directory"].as_str().unwrap(),
            ])
            .env("PATH", self.directory.join("no-tools"))
            .env("CARGO", self.directory.join("no-cargo"))
            .env("RUSTC", self.directory.join("no-rustc"))
            .env("RSTYLE_DRIVER", self.directory.join("no-driver"));
        command
    }

    fn run(&self, extra: &[&str]) -> Output {
        diagnostic_process::output(self.command("json").args(extra)).unwrap()
    }

    fn dismiss(&self, index: &str, code: i32) -> Value {
        let output = self.run(&[
            "--dismiss-guard",
            key(self.original),
            index,
            "--review-reason",
            "Reviewed this exact guard opportunity.",
        ]);
        checked(&output, code);
        parsed(&output)
    }
}

#[test]
fn exact_guard_review_filters_only_text_and_exit_and_replays_without_sources_or_tools() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    checked(&output, 1);
    counts(&original, 0, 1);
    state(&original, 0, "visible_new");
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    let compact = parsed(&replay.run(&["--report-detail", "compact"]));
    let hidden = replay.dismiss("0", 0);
    counts(&hidden, 1, 0);
    state(&hidden, 0, "hidden_exact");
    unchanged(&original, &hidden);
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    for path in ["src/lib.rs", "Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let output = replay.run(&["--fail-on-findings", "--report-detail", detail]);
        checked(&output, 0);
        let report = parsed(&output);
        counts(&report, 1, 0);
        unchanged(
            if detail == "full" {
                &original
            } else {
                &compact
            },
            &report,
        );
        assert_eq!(report["analysis"]["findings"], 1);
    }
    let output =
        diagnostic_process::output(replay.command("text").arg("--fail-on-findings")).unwrap();
    checked(&output, 0);
    assert!(!stdout(&output).contains("\nguard_clause "));
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn guard_movement_retains_exact_review_but_changed_condition_and_fallback_are_stale() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("0", 0);
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("// moved π\n\n{ORIGINAL}"),
    )
    .unwrap();
    let (output, moved) = capture(directory.path(), &["--fail-on-findings"]);
    checked(&output, 0);
    counts(&moved, 1, 0);
    state(&moved, 0, "hidden_exact");
    for source in [
        ORIGINAL.replace("value > 0", "value > 1"),
        ORIGINAL.replace("else { 0 }", "else { 9 }"),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (output, changed) = capture(directory.path(), &["--fail-on-findings"]);
        checked(&output, 1);
        counts(&changed, 0, 1);
        state(&changed, 0, "stale");
    }
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

fn siblings() -> String {
    format!("pub fn outer() {{ let first={FIRST}; let second={SECOND}; }}\n")
}

#[test]
fn sibling_closure_reviews_are_separate_and_cannot_complete_the_enclosing_owner() {
    let directory = workspace(&siblings());
    let (output, original) = capture(directory.path(), &["--fail-on-findings"]);
    checked(&output, 2);
    counts(&original, 0, 2);
    assert_eq!(original["analysis"]["findings"], 1);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    let first = replay.dismiss("0", 2);
    counts(&first, 1, 1);
    state(&first, 0, "hidden_exact");
    state(&first, 1, "visible_new");
    let both = replay.dismiss("1", 2);
    counts(&both, 2, 0);
    unchanged(&original, &both);
    assert_eq!(both["analysis"]["scope"]["coverage"], "incomplete");
    let output = replay.run(&["--fail-on-findings"]);
    checked(&output, 2);
    let edited = siblings().replace("|first:u32| {", "|first:u32| { let extra=0u32;");
    std::fs::write(directory.path().join("src/lib.rs"), edited).unwrap();
    let (output, changed) = capture(directory.path(), &["--fail-on-findings"]);
    checked(&output, 2);
    counts(&changed, 1, 1);
    state(&changed, 0, "stale");
    state(&changed, 1, "hidden_exact");
}

#[test]
fn old_guard_review_schema_lookup_does_not_upgrade_or_rewrite_the_file() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    let file = directory.path().join("reviews.json");
    let bytes = b"{\"schema_version\":9,\"entries\":[]}\n";
    std::fs::write(&file, bytes).unwrap();
    let output = Replay {
        directory: directory.path(),
        original: &original,
    }
    .run(&["--review-status", "--fail-on-findings"]);
    checked(&output, 1);
    counts(&parsed(&output), 0, 1);
    assert_eq!(
        parsed(&output)["review_status"]["records"],
        serde_json::json!([])
    );
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn absent_guard_opportunity_is_unavailable_and_cannot_prune_a_retained_review() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    Replay {
        directory: directory.path(),
        original: &original,
    }
    .dismiss("0", 0);
    let file = directory.path().join("reviews.json");
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn nested(value:u32)->u32 { value }\n",
    )
    .unwrap();
    let (output, negative) = capture(directory.path(), &["--review-status"]);
    checked(&output, 2);
    counts(&negative, 0, 0);
    assert_eq!(
        negative["review_status"]["records"][0]["state"],
        "unavailable"
    );
    let output = Replay {
        directory: directory.path(),
        original: &negative,
    }
    .run(&["--review-prune", "--review-prune-dry-run"]);
    checked(&output, 2);
    assert_eq!(parsed(&output)["review_prune"]["eligible_entries"], 0);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

fn invalid_selectors(replay: &Replay<'_>) {
    for selector in ["1", "999", "invalid", "-1"] {
        let output = replay.run(&[
            "--dismiss-guard",
            key(replay.original),
            selector,
            "--review-reason",
            "Invalid selection must not persist.",
        ]);
        checked(&output, 2);
        assert!(!replay.directory.join("reviews.json").exists());
    }
}

#[test]
fn guard_review_rejects_invalid_selectors_malformed_stores_and_duplicate_entries() {
    let directory = workspace(ORIGINAL);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    invalid_selectors(&replay);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--guard-clauses",
            "--review-file",
            "reviews.json",
            "--dismiss-guard",
            key(&original),
            "0",
            "--review-reason",
            "Saved run is required.",
        ],
    );
    checked(&output, 2);
    assert!(String::from_utf8_lossy(&output.stderr).contains("--replay-run"));
    let file = directory.path().join("reviews.json");
    std::fs::write(&file, b"{broken").unwrap();
    checked(&replay.run(&[]), 2);
    std::fs::remove_file(&file).unwrap();
    replay.dismiss("0", 0);
    let mut store: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    let entries = store["guard_entries"].as_array_mut().unwrap();
    entries.push(entries[0].clone());
    let bytes = serde_json::to_vec_pretty(&store).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    checked(&replay.run(&[]), 2);
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn ordinary_guard_review_requires_no_unrelated_analysis() {
    let directory = workspace(ORIGINAL);
    let args = [
        "--extraction-profile",
        "hir-data-v3",
        "--guard-clauses",
        "--review-file",
        "reviews.json",
    ];
    let (output, original) = clones(directory.path(), &args);
    checked(&output, 0);
    counts(&original, 0, 1);
    let mut dismissal = args.to_vec();
    dismissal.extend_from_slice(&[
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
        "--dismiss-guard",
        key(&original),
        "0",
        "--review-reason",
        "Reviewed ordinary guard advice.",
        "--fail-on-findings",
    ]);
    let (output, hidden) = clones(directory.path(), &dismissal);
    checked(&output, 0);
    counts(&hidden, 1, 0);
    assert_eq!(hidden["analysis"], original["analysis"]);
    assert_eq!(hidden["functions"], original["functions"]);
}
