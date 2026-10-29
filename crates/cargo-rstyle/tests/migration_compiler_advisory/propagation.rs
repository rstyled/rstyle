use super::*;

mod associated;
mod concrete_traits;
mod shared_sources;

const IO: &str = "pub fn leaf(){std::fs::write(\"/unused\",b\"x\").ok();}";
const CALLER: &str = "pub fn caller(){renamed::leaf();}";

pub(super) fn workspace(leaf: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers=['caller','leaf']\nresolver='2'\n",
    )
    .unwrap();
    for (name, source, dependencies) in [
        (
            "caller",
            CALLER,
            "[dependencies]\nrenamed={package='leaf',path='../leaf'}\n",
        ),
        ("leaf", leaf, ""),
    ] {
        let root = directory.path().join(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname='{name}'\nversion='0.1.0'\nedition='2021'\n{dependencies}"),
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), source).unwrap();
    }
    directory
}

pub(super) fn capture(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec!["--extraction-profile", "hir-data-v3", "--compiler-advisory"];
    args.extend_from_slice(extra);
    clones(directory, &args)
}

pub(super) fn unit<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| unit["package_root"].as_str().unwrap().ends_with(name))
        .unwrap()
}

fn io(unit: &Value) -> u64 {
    unit["evaluation"]["summary"]["effect_counts"]["io"]
        .as_u64()
        .unwrap_or(0)
}

pub(super) fn checked(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}\n{}",
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn renamed_direct_workspace_calls_reuse_core_propagation_without_strict_admission() {
    let directory = workspace(IO);
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(io(unit(&report, "caller")), 1);
    assert_eq!(io(unit(&report, "leaf")), 1);
    assert_eq!(
        unit(&report, "caller")["resolved_dependency_callees"]["leaf::leaf"],
        unit(&report, "leaf")["unit"]
    );
    assert_eq!(
        unit(&report, "caller")["unresolved_callees"],
        serde_json::json!([])
    );
    assert!(
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| function["outcome"] != "admitted")
    );
}

#[test]
fn cross_unit_hops_and_suppression_boundaries_use_the_existing_policy() {
    let directory = workspace(IO);
    let (output, _) = capture(directory.path(), &[]);
    checked(&output, 0);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[side_effects]\ntransitive_depth=0\n",
    )
    .unwrap();
    let (output, zero) = capture(
        directory.path(),
        &["--compiler-advisory-config", "policy.toml"],
    );
    checked(&output, 0);
    assert_eq!(io(unit(&zero, "caller")), 0);
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        format!("// rstyle-allow(side_effect_density): boundary\n{IO}"),
    )
    .unwrap();
    let (output, boundary) = capture(directory.path(), &[]);
    checked(&output, 0);
    assert_eq!(io(unit(&boundary, "caller")), 0);
    assert_eq!(io(unit(&boundary, "leaf")), 1);
}

#[test]
fn omitted_and_generated_dependency_owners_remain_unresolved() {
    let directory = workspace(IO);
    let (output, omitted) = clones(
        directory.path(),
        &[
            "-p",
            "caller",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
        ],
    );
    checked(&output, 2);
    assert_eq!(io(unit(&omitted, "caller")), 0);
    assert_eq!(
        unit(&omitted, "caller")["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        format!("macro_rules! make {{()=>{{{IO}}}}} make!();"),
    )
    .unwrap();
    let (output, generated) = capture(directory.path(), &[]);
    checked(&output, 2);
    assert_eq!(
        unit(&generated, "caller")["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    assert!(
        !unit(&generated, "leaf")["unavailable"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn exclusion_does_not_turn_a_cross_unit_callee_into_a_pure_completed_target() {
    let directory = workspace(IO);
    std::fs::write(
        directory.path().join("policy.toml"),
        "exclude=['leaf/**']\n",
    )
    .unwrap();
    let (output, report) = capture(
        directory.path(),
        &["--compiler-advisory-config", "policy.toml"],
    );
    checked(&output, 2);
    assert_eq!(io(unit(&report, "caller")), 0);
    assert_eq!(
        unit(&report, "caller")["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
}

#[test]
fn cross_unit_effects_replay_without_sources_and_ignore_strict_pair_budgets() {
    let directory = workspace(IO);
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    for path in ["caller", "leaf", "Cargo.toml", "Cargo.lock"] {
        let path = directory.path().join(path);
        if path.is_dir() {
            std::fs::remove_dir_all(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    let (output, replayed) = capture(
        directory.path(),
        &[
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--max-pairs",
            "0",
        ],
    );
    // Neither IO function is strictly admitted, so the strict pair budget has no work.
    checked(&output, 0);
    assert_eq!(original["compiler_advisory"], replayed["compiler_advisory"]);
    assert_eq!(original["matching"], replayed["matching"]);
}

#[test]
fn same_crate_paths_from_distinct_dependencies_remain_ambiguous() {
    let directory = workspace(IO);
    std::fs::write(
        directory.path().join("Cargo.toml"),
        "[workspace]\nmembers=['caller','leaf','other']\nresolver='2'\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("leaf/Cargo.toml"),
        "[package]\nname='leaf'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='shared'\n",
    )
    .unwrap();
    std::fs::create_dir_all(directory.path().join("other/src")).unwrap();
    std::fs::write(
        directory.path().join("other/Cargo.toml"),
        "[package]\nname='other'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='shared'\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("other/src/lib.rs"), "pub fn leaf(){}").unwrap();
    std::fs::write(directory.path().join("caller/Cargo.toml"),
        "[package]\nname='caller'\nversion='0.1.0'\nedition='2021'\n[dependencies]\nrenamed={package='leaf',path='../leaf'}\nsecond={package='other',path='../other'}\n").unwrap();
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 2);
    assert_eq!(io(unit(&report, "caller")), 0);
    assert_eq!(
        unit(&report, "caller")["unresolved_callees"],
        serde_json::json!(["shared::leaf"])
    );
}

#[test]
fn indistinguishable_check_and_test_graph_entries_never_share_effect_inputs() {
    let directory = workspace(&format!(
        "#[cfg(not(test))]\n{IO}\n#[cfg(test)] pub fn leaf(){{}}"
    ));
    let (output, report) = clones(
        directory.path(),
        &[
            "--all-targets",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
        ],
    );
    checked(&output, 2);
    let callers: Vec<_> = report["compiler_advisory"]["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|unit| unit["package_root"].as_str().unwrap().ends_with("caller"))
        .collect();
    assert_eq!(callers.len(), 2);
    for caller in callers {
        let id = caller["unit"].as_u64().unwrap() as usize;
        let observed = report["unit_contexts"][id]["observed_unit"]
            .as_u64()
            .unwrap() as usize;
        assert!(report["observed_units"][observed]["test"].is_boolean());
        // Cargo emits two indistinguishable graph descriptors for this check/test
        // selection. Neither can supply a unique dependency route.
        assert_eq!(io(caller), 0);
        assert_eq!(
            caller["unresolved_callees"],
            serde_json::json!(["leaf::leaf"])
        );
    }
}

#[test]
fn a_local_owner_and_dependency_with_the_same_compiler_path_remain_unresolved() {
    let directory = workspace(IO);
    std::fs::write(directory.path().join("caller/Cargo.toml"),
        "[package]\nname='caller'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='leaf'\n[dependencies]\nrenamed={package='leaf',path='../leaf'}\n").unwrap();
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn leaf(){} pub fn caller(){renamed::leaf();}",
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &[]);
    checked(&output, 2);
    assert_eq!(io(unit(&report, "caller")), 0);
    assert_eq!(
        unit(&report, "caller")["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
}

#[test]
fn one_shared_propagation_pass_preserves_hop_limits_across_package_boundaries() {
    let directory =
        workspace("pub fn leaf(){inner();} fn inner(){std::fs::write(\"/unused\",b\"x\").ok();}");
    let (output, original) = capture(directory.path(), &[]);
    checked(&output, 0);
    for (depth, caller_io) in [(1, 0), (2, 1)] {
        std::fs::write(
            directory.path().join("policy.toml"),
            format!("[side_effects]\ntransitive_depth={depth}\n"),
        )
        .unwrap();
        let (output, report) = capture(
            directory.path(),
            &[
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
                "--compiler-advisory-config",
                "policy.toml",
            ],
        );
        checked(&output, 0);
        assert_eq!(io(unit(&report, "caller")), caller_io);
        assert_eq!(io(unit(&report, "leaf")), 2);
    }
}

fn reviewed_density_workspace() -> TempDir {
    let directory = workspace(IO);
    std::fs::write(
        directory.path().join("policy.toml"),
        "[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\n",
    )
    .unwrap();
    let policy = ["--compiler-advisory-config", "policy.toml"];
    let (output, original) = capture(directory.path(), &policy);
    checked(&output, 0);
    let caller = unit(&original, "caller");
    let id = caller["unit"].as_u64().unwrap().to_string();
    let diagnostic = caller["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .position(|diagnostic| diagnostic["rule_id"] == "side_effect_density")
        .unwrap()
        .to_string();
    let (output, _) = capture(
        directory.path(),
        &[
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            &id,
            &diagnostic,
            "--review-reason",
            "Independently reviewed fixture density",
        ],
    );
    checked(&output, 0);
    directory
}

#[test]
fn missing_dependency_effect_inputs_cannot_retire_a_retained_density_finding() {
    let directory = reviewed_density_workspace();
    let path = directory.path().join("reviews.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(
        directory.path().join("leaf/src/lib.rs"),
        format!("macro_rules! make {{()=>{{{IO}}}}} make!();"),
    )
    .unwrap();
    let (output, report) = capture(
        directory.path(),
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-file",
            "reviews.json",
            "--review-status",
        ],
    );
    checked(&output, 2);
    assert_eq!(
        report["review_status"]["records"][0]["state"],
        "unavailable"
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
