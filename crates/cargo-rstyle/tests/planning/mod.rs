use super::*;

fn workspace(case: &str) -> TempDir {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/planning-cases.json"
    ))
    .unwrap();
    let source = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == case)
        .unwrap()["source"]
        .as_str()
        .unwrap();
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn requested(directory: &Path) -> (Output, Value) {
    clones(
        directory,
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
            "--edit-plans",
        ],
    )
}

#[test]
fn shared_helper_members_retain_exact_original_body_overlap_witnesses() {
    let directory = workspace("shared-helper-endpoint-is-required-edit-overlap");
    let (output, report) = requested(directory.path());
    assert!(output.status.success(), "{}", stdout(&output));
    let planning = &report["edit_planning"];
    assert_eq!(planning["coverage"], "complete");
    assert_eq!(planning["plans"].as_array().unwrap().len(), 3);
    assert_eq!(planning["regions"].as_array().unwrap().len(), 3);
    assert_eq!(planning["overlap_groups"].as_array().unwrap().len(), 1);
    assert_eq!(planning["whole_plan_independence"], "unknown");
    assert_eq!(planning["stack_order"], "not_proven");
    let original = std::fs::read_to_string(directory.path().join("src/lib.rs")).unwrap();
    for relation in planning["relations"].as_array().unwrap() {
        assert_eq!(relation["relation"], "overlap_witness");
        let witnesses = relation["witnesses"].as_array().unwrap();
        assert_eq!(witnesses.len(), 1);
        let witness = &witnesses[0];
        let first_plan = &planning["plans"][relation["first"].as_u64().unwrap() as usize];
        let second_plan = &planning["plans"][relation["second"].as_u64().unwrap() as usize];
        let first_region = first_plan["required_regions"]
            [witness["first_region"].as_u64().unwrap() as usize]
            .as_u64()
            .unwrap() as usize;
        let second_region = second_plan["required_regions"]
            [witness["second_region"].as_u64().unwrap() as usize]
            .as_u64()
            .unwrap() as usize;
        assert_eq!(first_region, second_region);
        let region = &planning["regions"][first_region];
        assert_eq!(region["resource"], "src/lib.rs");
        assert_eq!(region["package_manifest"], "Cargo.toml");
        assert_eq!(region["span"], witness["intersection"]);
        let span = &region["span"];
        let body = &original
            [span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize];
        assert!(body.starts_with('{') && body.ends_with('}'));
        assert!(body.contains("value+"));
        assert!(region.get("original").is_none());
    }
}

#[test]
fn disjoint_helper_bodies_leave_insertions_and_whole_plan_independence_unknown() {
    let directory = workspace("separate-helper-regions-in-one-file");
    let (output, report) = requested(directory.path());
    assert!(output.status.success(), "{}", stdout(&output));
    let planning = &report["edit_planning"];
    assert_eq!(planning["coverage"], "complete");
    assert_eq!(planning["plans"].as_array().unwrap().len(), 2);
    assert_eq!(
        planning["relations"][0]["relation"],
        "disjoint_known_regions"
    );
    assert_eq!(planning["whole_plan_independence"], "unknown");
    assert_eq!(
        planning["unknown_edits"],
        serde_json::json!(["helper_declaration_insertion", "helper_name_and_scope"])
    );
    assert!(planning["overlap_groups"].as_array().unwrap().is_empty());
}

#[test]
fn unavailable_helpers_do_not_become_clean_independent_plans() {
    for case in [
        "unsupported-helper-input-does-not-become-an-empty-independent-plan",
        "non-u32-native-evidence-not-helper-plan-authority",
    ] {
        let directory = workspace(case);
        let (output, report) = requested(directory.path());
        assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
        assert_eq!(report["edit_planning"]["coverage"], "incomplete");
        assert!(report["edit_planning"]["expected_plans"].is_null());
        assert!(
            report["edit_planning"]["plans"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            report["edit_planning"]["whole_plan_independence"],
            "unknown"
        );
    }
}

#[test]
fn planning_replay_retains_physical_footprints_without_original_files_or_tools() {
    let directory = workspace("shared-helper-endpoint-is-required-edit-overlap");
    let (output, original) = requested(directory.path());
    assert!(output.status.success(), "{}", stdout(&output));
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    let mut command = Command::new(executable);
    command
        .current_dir(directory.path())
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--helper-proposals",
            "--edit-plans",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.path().join("no-tools"))
        .env("CARGO", directory.path().join("no-cargo"))
        .env("RSTYLE_DRIVER", directory.path().join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(replayed["verification_policy"], "strict-replay");
    assert_eq!(replayed["edit_planning"], original["edit_planning"]);
    assert_eq!(replayed["matching"], original["matching"]);
}

#[test]
fn helper_policy_rejection_creates_no_edit_intent() {
    let directory = workspace("same-source-similarity-without-helper-finding");
    let (output, report) = requested(directory.path());
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(report["helper_proposals"]["findings"], 0);
    assert_eq!(report["edit_planning"]["expected_plans"], 0);
    assert!(
        report["edit_planning"]["plans"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["edit_planning"]["coverage"], "complete");
}

#[test]
fn same_physical_body_across_build_contexts_remains_overlapping() {
    let directory = workspace("same-file-region-in-different-compilation-contexts");
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--all-targets",
            "--helper-proposals",
            "--edit-plans",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["helper_proposals"]["coverage"], "incomplete");
    assert_eq!(report["edit_planning"]["coverage"], "incomplete");
    let (left, right) = different_context_regions(&report);
    assert!(
        report["edit_planning"]["relations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|relation| relation_contains_regions(&report, relation, (left, right)))
    );
}

fn different_context_regions(report: &Value) -> (usize, usize) {
    let regions = report["edit_planning"]["regions"].as_array().unwrap();
    regions
        .iter()
        .enumerate()
        .find_map(|(left_index, left)| {
            regions.iter().enumerate().find_map(|(right_index, right)| {
                (left["resource"] == right["resource"]
                    && left["span"] == right["span"]
                    && context_of(report, left) != context_of(report, right))
                .then_some((left_index, right_index))
            })
        })
        .expect("same source body in two checked contexts")
}

fn context_of<'a>(report: &'a Value, region: &Value) -> &'a Value {
    &report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["key"] == region["function"])
        .unwrap()["unit_evidence"]["report_unit"]
}

fn relation_contains_regions(report: &Value, relation: &Value, regions: (usize, usize)) -> bool {
    if relation["relation"] != "overlap_witness" {
        return false;
    }
    let plans = &report["edit_planning"]["plans"];
    let first = &plans[relation["first"].as_u64().unwrap() as usize];
    let second = &plans[relation["second"].as_u64().unwrap() as usize];
    relation["witnesses"]
        .as_array()
        .unwrap()
        .iter()
        .any(|witness| {
            let left = first["required_regions"][witness["first_region"].as_u64().unwrap() as usize]
                .as_u64()
                .unwrap() as usize;
            let right = second["required_regions"]
                [witness["second_region"].as_u64().unwrap() as usize]
                .as_u64()
                .unwrap() as usize;
            (left == regions.0 && right == regions.1) || (left == regions.1 && right == regions.0)
        })
}
