use super::propagation::{capture, checked, unit, workspace};
use super::*;

const LEAF: &str =
    "pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"x\").ok();}";
const CALLER: &str = "pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){renamed::leaf(a,b,c,d,e);}";

fn fixture() -> TempDir {
    let directory = workspace(LEAF);
    std::fs::write(directory.path().join("caller/src/lib.rs"), CALLER).unwrap();
    directory
}

fn planning(report: &Value) -> &Value {
    &report["compiler_advisory"]["planning"]
}

fn key(report: &Value, package: &str, name: &str) -> Value {
    unit(report, package)["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str().unwrap().ends_with(name))
        .unwrap()["key"]
        .clone()
}

#[test]
fn cross_unit_plans_reuse_core_order_bundles_and_pass_through_without_strict_admission() {
    let directory = fixture();
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    let plan = planning(&report);
    assert_eq!(plan["input_complete"], true);
    assert_eq!(plan["known_functions"], 2);
    let leaf = key(&report, "leaf", "leaf");
    let caller = key(&report, "caller", "caller");
    let group = &plan["groups"][0];
    assert_eq!(plan["groups"].as_array().unwrap().len(), 1);
    assert_eq!(group["members"], serde_json::json!([leaf, caller]));
    assert_eq!(group["step_owners"], group["members"]);
    assert_eq!(group["bundle_owners"][0].as_array().unwrap().len(), 2);
    let steps = &group["evaluation"]["plan"]["steps"];
    assert_eq!(steps[0]["incoming_sites"], 1);
    assert_eq!(steps[1]["outgoing_sites"], 1);
    assert_eq!(
        steps[1]["pass_through"],
        serde_json::json!(["a", "b", "c", "d", "e"])
    );
    assert_eq!(group["evaluation"]["plan"]["bundles"][0]["sites"], 1);
    assert!(report["accepted_pairs"].as_array().unwrap().is_empty());
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["outcome"] != "admitted")
    );
}

#[test]
fn plans_are_replay_only_optional_and_do_not_change_existing_reports() {
    let directory = fixture();
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert!(original["compiler_advisory"].get("planning").is_none());
    for path in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.path().join(path)).unwrap();
    }
    for detail in ["full", "compact"] {
        let (output, replayed) = capture(
            directory.path(),
            &[
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
                "--compiler-plans",
                "--max-pairs",
                "0",
                "--report-detail",
                detail,
            ],
        );
        checked(&output, 0);
        assert_eq!(planning(&replayed)["input_complete"], true);
        let mut advisory = replayed["compiler_advisory"].clone();
        advisory.as_object_mut().unwrap().remove("planning");
        assert_eq!(advisory, original["compiler_advisory"]);
        assert_eq!(replayed["matching"], original["matching"]);
    }
}

#[test]
fn missing_generated_and_excluded_callees_leave_plans_incomplete() {
    let directory = fixture();
    let (output, missing) = clones(
        directory.path(),
        &[
            "-p",
            "caller",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
        ],
    );
    checked(&output, 2);
    assert_eq!(planning(&missing)["input_complete"], false);
    assert_eq!(planning(&missing)["known_functions"], 1);
    assert_eq!(
        planning(&missing)["groups"][0]["step_owners"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    std::fs::write(
        directory.path().join("policy.toml"),
        "exclude=['leaf/**']\n",
    )
    .unwrap();
    let (output, excluded) = capture(
        directory.path(),
        &[
            "--compiler-plans",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 2);
    assert_eq!(planning(&excluded)["input_complete"], false);
    assert_eq!(planning(&excluded)["known_functions"], 1);
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        format!("macro_rules! make {{()=>{{{LEAF}}}}} make!();"),
    )
    .unwrap();
    let (output, generated) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    assert_eq!(planning(&generated)["input_complete"], false);
    assert_eq!(planning(&generated)["known_functions"], 1);
}

#[test]
fn core_policy_and_original_suppressions_drive_shared_planning() {
    let directory = fixture();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!("// rstyle-allow(fn_params): fixture forwarding boundary\n{CALLER}"),
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    let group = &planning(&report)["groups"][0];
    assert_eq!(
        group["members"],
        serde_json::json!([key(&report, "leaf", "leaf")])
    );
    assert_eq!(
        group["step_owners"],
        serde_json::json!([
            key(&report, "leaf", "leaf"),
            key(&report, "caller", "caller")
        ])
    );
    assert_eq!(group["evaluation"]["plan"]["steps"][1]["signature"], false);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[fn_params]\nmax_params=5\n",
    )
    .unwrap();
    let (output, clean) = capture(
        directory.path(),
        &[
            "--replay-run",
            report["run_directory"].as_str().unwrap(),
            "--compiler-plans",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 0);
    assert_eq!(planning(&clean)["groups"], serde_json::json!([]));
    assert_eq!(planning(&clean)["input_complete"], true);
}

#[test]
fn ambiguous_check_test_dependencies_do_not_acquire_cross_context_steps() {
    let directory = fixture();
    let (output, report) = clones(
        directory.path(),
        &[
            "--all-targets",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
        ],
    );
    checked(&output, 2);
    assert_eq!(planning(&report)["input_complete"], false);
    for group in planning(&report)["groups"].as_array().unwrap() {
        let members = group["members"].as_array().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(&group["step_owners"], &group["members"]);
    }
    assert_eq!(planning(&report)["groups"].as_array().unwrap().len(), 4);
}

#[test]
fn single_unit_planning_keeps_known_empty_results_distinct() {
    let (_directory, output, report) = super::capture("pub fn empty(){}", &["--compiler-plans"]);
    checked(&output, 0);
    assert_eq!(planning(&report)["input_complete"], true);
    assert_eq!(planning(&report)["known_functions"], 1);
    assert_eq!(planning(&report)["groups"], serde_json::json!([]));
}

#[test]
fn text_displays_core_plans_and_cli_requires_advisory() {
    let directory = fixture();
    let invalid = rstyle(directory.path(), &["clones", "--compiler-plans"]);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--compiler-advisory"));
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let text = stdout(&output);
    assert!(text.contains("Compiler planning:"));
    assert!(text.contains("Captured callers only"));
    assert!(text.contains("pass-through: a, b, c, d, e"));
    assert!(text.contains("bundle (1 sites)"));
}

#[test]
fn local_and_foreign_namesakes_never_share_a_planning_target() {
    let directory = fixture();
    std::fs::write(directory.path().join("caller/Cargo.toml"),
        "[package]\nname='caller'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='leaf'\n[dependencies]\nrenamed={package='leaf',path='../leaf'}\n").unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        format!("{LEAF}\n{CALLER}"),
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    assert_eq!(planning(&report)["input_complete"], false);
    assert_eq!(planning(&report)["groups"].as_array().unwrap().len(), 3);
    for group in planning(&report)["groups"].as_array().unwrap() {
        assert_eq!(group["members"].as_array().unwrap().len(), 1);
        assert_eq!(group["step_owners"], group["members"]);
    }
}

fn reviewed_fixture() -> (TempDir, Value) {
    let directory = fixture();
    let (output, original) = capture(
        directory.path(),
        &["--compiler-plans", "--compiler-patterns"],
    );
    checked(&output, 0);
    let leaf = unit(&original, "leaf");
    let ordinal = leaf["unit"].as_u64().unwrap().to_string();
    let diagnostic = leaf["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["rule_id"] == "fn_params")
        .unwrap()
        .to_string();
    let (output, reviewed) = capture(
        directory.path(),
        &[
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--compiler-plans",
            "--compiler-patterns",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            &ordinal,
            &diagnostic,
            "--review-reason",
            "Independently reviewed fixture signature",
        ],
    );
    checked(&output, 0);
    assert_eq!(planning(&reviewed), planning(&original));
    (directory, original)
}

#[test]
fn patterns_reviews_and_file_selection_preserve_raw_shared_plans() {
    let (directory, original) = reviewed_fixture();
    std::fs::write(
        directory.path().join("selection.json"),
        r#"{"schema":"rstyle.changed-files/1","paths":["caller/src/lib.rs"]}"#,
    )
    .unwrap();
    let (output, selected) = capture(
        directory.path(),
        &[
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--compiler-plans",
            "--compiler-patterns",
            "--review-file",
            "reviews.json",
            "--changed-files",
            "selection.json",
            "--fail-on-findings",
        ],
    );
    checked(&output, 1);
    assert_eq!(planning(&selected), planning(&original));
    for package in ["caller", "leaf"] {
        assert_eq!(
            unit(&selected, package)["evaluation"],
            unit(&original, package)["evaluation"]
        );
        assert!(unit(&selected, package)["patterns"].is_object());
    }
}
