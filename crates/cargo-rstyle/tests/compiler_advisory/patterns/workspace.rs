use super::*;
use std::collections::BTreeSet;

const LEFT: &str = r#"
pub struct Dog;
impl Dog {
    pub fn sound(&self)->String { "woof".into() }
    pub fn steps(&self)->u32 { 4 }
}
pub fn dogs(values:&[Dog],out:&mut Vec<String>)->u32 {
    let mut n=0; for value in values {out.push(value.sound()); n+=value.steps();} n
}
pub fn tag_left(value:u128)->u128 {
    std::hint::black_box(value.wrapping_add(7)).wrapping_mul(3)
}
pub fn plain_left(value:u32)->u32 {value}
pub fn forward_left(value:u32)->u32 {plain_left(value)}
"#;

const RIGHT: &str = r#"
pub struct Cat;
impl Cat {
    pub fn noise(&self)->String { "meow".into() }
    pub fn stride(&self)->u32 { 2 }
}
pub fn cats(values:&[Cat],out:&mut Vec<String>)->u32 {
    let mut n=0; for value in values {out.push(value.noise()); n+=value.stride();} n
}
pub fn tag_right(value:u128)->u128 {
    std::hint::black_box(value.wrapping_add(8)).wrapping_mul(3)
}
pub fn plain_right(value:u32)->u32 {value}
pub fn forward_right(value:u32)->u32 {plain_right(value)}
"#;

fn fixture() -> TempDir {
    let directory = propagation::workspace(LEFT);
    std::fs::write(directory.path().join("caller/src/lib.rs"), RIGHT).unwrap();
    directory
}

fn run(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-patterns",
        "--compiler-advisory-only",
    ];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

fn populations(report: &Value) -> Vec<&Value> {
    report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|unit| unit.get("patterns").is_some_and(Value::is_object))
        .collect()
}

fn checked(output: &Output, code: i32) {
    mining_options::checked(output, code);
}

fn only_patterns(report: &Value) {
    assert_eq!(report["matching_execution"], "not_requested");
    assert_eq!(report["strict_assessment_execution"], "not_requested");
    assert_eq!(
        report["compiler_advisory"]["diagnostic_execution"],
        "not_requested"
    );
}

fn opportunities(patterns: &Value) {
    let evaluation = &patterns["evaluation"];
    assert!(!serde_json::to_string(evaluation).unwrap().contains("@u"));
    for (name, other, kind) in [
        ("dogs", "cats", "capability_set"),
        ("tag_left", "tag_right", "parameterize"),
    ] {
        let candidate = candidate_at(evaluation, name).expect("cross-package opportunity");
        assert_eq!(candidate["kind"], kind);
        for field in ["inference", "possible_refactor"] {
            assert!(
                !candidate[field].as_str().unwrap().contains('@'),
                "{candidate}"
            );
        }
        let names = site_names(candidate, "sites");
        assert_eq!(names.len(), 2);
        assert!(names.iter().any(|site| site == other));
        let paths: BTreeSet<_> = candidate["sites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|site| site["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            ["caller/src/lib.rs", "leaf/src/lib.rs"]
                .into_iter()
                .collect()
        );
        for site in candidate["sites"]
            .as_array()
            .unwrap()
            .iter()
            .chain(candidate["definitions"].as_array().unwrap())
        {
            assert!(
                patterns["owners"]
                    .get(site["id"].as_str().unwrap())
                    .is_some()
            );
            assert!(!site["locator"].as_str().unwrap().is_empty());
        }
    }
    for name in ["plain_left", "plain_right", "forward_left", "forward_right"] {
        assert!(
            candidate_at(evaluation, name).is_none(),
            "{name}: {evaluation}"
        );
    }
}

#[test]
fn workspace_patterns_find_capabilities_and_helpers_only_across_selected_packages() {
    let directory = fixture();
    let (output, report) = run(directory.path(), &["--fail-on-findings"]);
    checked(&output, 1);
    only_patterns(&report);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    let rows = populations(&report);
    assert_eq!(rows.len(), 1);
    let patterns = &rows[0]["patterns"];
    assert_eq!(patterns["member_units"].as_array().unwrap().len(), 2);
    opportunities(patterns);
    for package in ["leaf", "caller"] {
        let (output, isolated) = run(directory.path(), &["-p", package, "--fail-on-findings"]);
        checked(&output, 0);
        let rows = populations(&isolated);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0]["patterns"]["member_units"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            rows[0]["patterns"]["evaluation"]["candidates"],
            serde_json::json!([])
        );
    }
}

fn saved(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-patterns",
            "--compiler-advisory-only",
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

fn member_contexts(baseline: &Value) {
    assert!(!serde_json::to_string(baseline).unwrap().contains("@u"));
    let contexts = baseline["contexts"].as_array().unwrap();
    assert_eq!(contexts.len(), 2);
    for entry in baseline["entries"].as_array().unwrap() {
        let declarations: Vec<_> = entry["sites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|site| &site["declaration"])
            .chain(entry["definitions"].as_array().unwrap())
            .collect();
        let packages: BTreeSet<_> = declarations
            .iter()
            .map(|declaration| declaration["locator"]["package"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(packages, ["caller", "leaf"].into_iter().collect());
        for declaration in declarations {
            assert!(contexts.contains(&declaration["context"]));
        }
    }
}

fn remove_workspace_sources(directory: &Path) {
    for path in ["caller/src", "leaf/src", "src"] {
        std::fs::remove_dir_all(directory.join(path)).unwrap();
    }
    for path in [
        "Cargo.toml",
        "caller/Cargo.toml",
        "leaf/Cargo.toml",
        "Cargo.lock",
    ] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
}

fn dismiss_workspace_candidates(directory: &Path, original: &Value) {
    let rows = populations(original);
    let anchor = rows[0]["unit"].as_u64().unwrap().to_string();
    for candidate in rows[0]["patterns"]["evaluation"]["candidates"]
        .as_array()
        .unwrap()
    {
        let (output, reviewed) = saved(
            directory,
            original,
            &[
                "--review-file",
                "reviews.json",
                "--dismiss-compiler-pattern",
                &anchor,
                candidate["id"].as_str().unwrap(),
                "--review-reason",
                "Cross-package fixture review",
            ],
        );
        checked(&output, 0);
        assert_eq!(
            populations(&reviewed)[0]["patterns"]["evaluation"],
            rows[0]["patterns"]["evaluation"]
        );
    }
}

fn exact_workspace_reviews(directory: &Path, original: &Value) {
    let (output, reviewed) = saved(
        directory,
        original,
        &[
            "--review-file",
            "reviews.json",
            "--review-status",
            "--fail-on-findings",
        ],
    );
    checked(&output, 0);
    only_patterns(&reviewed);
    assert_eq!(
        populations(&reviewed)[0]["patterns"]["visible_candidates"],
        serde_json::json!([])
    );
    let decisions = reviewed["compiler_advisory"]["pattern_review"]["decisions"]
        .as_array()
        .unwrap();
    assert_eq!(decisions.len(), 2);
    assert!(decisions.iter().all(|row| row["state"] == "hidden_exact"));
}

fn unchanged_workspace_baseline(directory: &Path, original: &Value) {
    let (output, compared) = saved(
        directory,
        original,
        &[
            "--compiler-pattern-baseline",
            "baseline.json",
            "--fail-on-new-compiler-patterns",
        ],
    );
    checked(&output, 0);
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["existing_findings"],
        2
    );
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["new_findings"],
        0
    );
    assert_eq!(
        compared["compiler_advisory"]["pattern_baseline"]["scope_changed"],
        false
    );
    assert_eq!(
        populations(&compared)[0]["patterns"],
        populations(original)[0]["patterns"]
    );
}

#[test]
fn workspace_pattern_reviews_and_baselines_replay_every_member_without_sources_or_tools() {
    let directory = fixture();
    let (output, original) = run(
        directory.path(),
        &["--write-compiler-pattern-baseline", "baseline.json"],
    );
    checked(&output, 0);
    let baseline_bytes = std::fs::read(directory.path().join("baseline.json")).unwrap();
    let baseline: Value = serde_json::from_slice(&baseline_bytes).unwrap();
    assert_eq!(baseline["entries"].as_array().unwrap().len(), 2);
    member_contexts(&baseline);
    remove_workspace_sources(directory.path());
    dismiss_workspace_candidates(directory.path(), &original);
    let review_bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    exact_workspace_reviews(directory.path(), &original);
    unchanged_workspace_baseline(directory.path(), &original);
    assert_eq!(
        std::fs::read(directory.path().join("baseline.json")).unwrap(),
        baseline_bytes
    );
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        review_bytes
    );
}

#[test]
fn workspace_pattern_populations_do_not_pair_check_and_test_duplicates() {
    let directory = fixture();
    let (output, report) = run(directory.path(), &["--all-targets"]);
    checked(&output, 2);
    only_patterns(&report);
    let rows = populations(&report);
    assert_eq!(rows.len(), 2);
    let mut populations_seen = BTreeSet::new();
    for row in rows {
        let patterns = &row["patterns"];
        assert_eq!(patterns["member_units"].as_array().unwrap().len(), 2);
        opportunities(patterns);
        let mut modes = BTreeSet::new();
        for id in patterns["member_units"].as_array().unwrap() {
            let ordinal = id.as_u64().unwrap() as usize;
            let observed = report["unit_contexts"][ordinal]["observed_unit"]
                .as_u64()
                .unwrap() as usize;
            modes.insert(
                report["observed_units"][observed]["test"]
                    .as_bool()
                    .unwrap(),
            );
        }
        assert_eq!(modes.len(), 1);
        populations_seen.extend(modes);
    }
    assert_eq!(populations_seen, [false, true].into_iter().collect());
}
