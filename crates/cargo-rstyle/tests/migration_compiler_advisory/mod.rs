use super::*;

mod complexity;
mod complexity_ranking;
mod extraction;
mod function_extraction;
mod instance_extraction;
mod mutations;
mod only;
mod only_effects;
mod only_mutations;
mod only_sources;
mod patterns;
mod planning;
mod propagation;
mod retained_owner_scope;
mod reviews;
mod selection;
mod shared_review_paths;
mod transport;
mod try_statements;
mod unit_extraction;

const GCD: &str =
    "pub fn gcd(mut a:u128,mut b:u128)->u128 {while b!=0 {(a,b)=(b,a%b);} if a==0 {1} else {a}}";

fn capture(source: &str, extra: &[&str]) -> (TempDir, Output, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let mut args = vec!["--extraction-profile", "hir-data-v3", "--compiler-advisory"];
    args.extend_from_slice(extra);
    let (output, report) = clones(directory.path(), &args);
    (directory, output, report)
}

#[test]
fn compiler_collectors_cover_u128_tuple_writes_without_strict_admission() {
    let source = format!("{GCD}\npub fn opaque(values:&mut Vec<u128>){{values.clear();}}");
    let (_directory, output, report) = capture(&source, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["metrics"], "rstyle.compiler-style-metrics/6");
    assert_eq!(advisory["complete"], true);
    let unit = &advisory["units"][0];
    assert_eq!(unit["expected_functions"], 2);
    let observation = &unit["observations"][0];
    assert_eq!(observation["direct_mutations"], 2);
    assert_eq!(observation["escaping_mutations"], 0);
    assert_eq!(unit["evaluation"]["summary"]["functions"], 2);
    let opaque = unit["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str().unwrap().ends_with("opaque"))
        .unwrap();
    assert_eq!(opaque["escaping_mutations"], 1);
    let raw = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == opaque["key"])
        .unwrap();
    assert_ne!(raw["outcome"], "admitted");
}

#[test]
fn core_policy_propagates_effects_and_builds_parameter_fix_plans() {
    let source = "fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"x\").ok();}\npub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){leaf(a,b,c,d,e);}";
    let (_directory, output, report) = capture(source, &[]);
    assert!(output.status.success(), "{}", stdout(&output));
    let evaluated = &report["compiler_advisory"]["units"][0]["evaluation"];
    assert_eq!(evaluated["summary"]["functions"], 2);
    assert_eq!(evaluated["summary"]["effect_counts"]["io"], 2);
    assert_eq!(
        evaluated["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|diagnostic| diagnostic["rule_id"] == "fn_params")
            .count(),
        2
    );
    assert!(!evaluated["fix_groups"].as_array().unwrap().is_empty());
    assert!(
        evaluated["fix_groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|group| !group["plan"].is_null())
    );
}

#[test]
fn advisory_replay_uses_captured_compiler_facts_and_new_policy_without_sources() {
    let (directory, output, original) = capture(GCD, &[]);
    assert!(output.status.success());
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    std::fs::write(
        directory.path().join("advisory.toml"),
        "[mutation_budget]\nmax_mutated_targets=1\n",
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--compiler-advisory-config",
            "advisory.toml",
            "--fail-on-findings",
            "--report-detail",
            "compact",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert_eq!(report["compiler_advisory"]["complete"], true);
    let diagnostics = report["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["rule_id"] == "mutated_targets"
                && diagnostic["actual"] == 2)
    );
    assert_eq!(report["accepted_pairs"], original["accepted_pairs"]);
}

#[test]
fn saved_runs_without_compiler_observations_remain_unavailable() {
    let directory = clone_workspace();
    let (output, original) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["compiler_advisory"]["complete"], false);
    let unit = &report["compiler_advisory"]["units"][0];
    assert_eq!(unit["observations"], serde_json::json!([]));
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), 4);
}

#[test]
fn generated_owners_keep_unavailability_instead_of_empty_body_metrics() {
    let source = "macro_rules! make {()=>{pub fn generated(){}}} make!(); pub fn original(){}";
    let (_directory, output, report) = capture(source, &[]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["capture_failures"], serde_json::json!([]));
    assert_eq!(report["handoff_failures"], serde_json::json!([]));
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["complete"], false);
    let unit = &advisory["units"][0];
    assert_eq!(unit["expected_functions"], 2);
    assert_eq!(unit["observations"].as_array().unwrap().len(), 1);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), 1);
}

#[test]
fn identical_functions_in_check_and_test_contexts_are_evaluated_separately() {
    let source = "pub fn original(){} #[cfg(test)] pub fn test_only(){}";
    let (_directory, _output, report) = capture(source, &["--all-targets"]);
    let units = report["compiler_advisory"]["units"].as_array().unwrap();
    assert_eq!(units.len(), 2);
    assert_ne!(units[0]["unit"], units[1]["unit"]);
    for unit in units {
        assert!(
            unit["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["name"] == "original")
        );
    }
    assert_eq!(
        units
            .iter()
            .filter(|unit| unit["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["name"] == "test_only"))
            .count(),
        1
    );
}
