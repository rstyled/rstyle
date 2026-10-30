use super::*;

fn two_files() -> TempDir {
    let directory =
        workspace("pub mod opaque;\npub fn many() -> u32 { let mut value=0u32; value=1; value }\n");
    std::fs::write(directory.path().join("src/opaque.rs"), OPAQUE).unwrap();
    directory
}

fn opaque_owner(report: &Value) -> &Value {
    report["mutation_analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["source_file"]
                .as_str()
                .unwrap()
                .ends_with("src/opaque.rs")
        })
        .unwrap()
}

#[test]
fn excluding_an_opaque_owner_completes_policy_scope_without_changing_raw_evidence() {
    let directory = two_files();
    let (_, baseline) = requested(directory.path(), &[]);
    policy(directory.path(), &["src/opaque.rs"]);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    let excluded = opaque_owner(&report);
    assert_raw_same(excluded, opaque_owner(&baseline));
    assert_eq!(excluded["coverage"], "incomplete");
    assert_eq!(excluded["evidence"]["write_sites"]["kind"], "lower_bound");
    assert!(
        !excluded["evidence"]["boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(excluded["policy"]["coverage"], "complete");
    assert_eq!(excluded["policy"]["source_exclusion"]["status"], "excluded");
    for id in ["mutation_per_target", "mutated_targets"] {
        assert_eq!(decision(excluded, id)["disposition"], "excluded");
        assert!(decision(excluded, id)["actual"].is_null());
        assert_eq!(decision(excluded, id)["outcome"], "not_evaluated");
    }
    let analysis = &report["mutation_analysis"];
    assert_eq!(analysis["expected_functions"], 2);
    assert_eq!(analysis["evaluated_functions"], 1);
    assert_eq!(analysis["unavailable_functions"], 1);
    assert_eq!(analysis["policy_evaluated_functions"], 1);
    assert_eq!(analysis["excluded_functions"], 1);
    assert_eq!(analysis["policy_unavailable_functions"], 0);
    policy(directory.path(), &[]);
    let (output, included) = replay(directory.path(), &report, "compact");
    assert_eq!(output.status.code(), Some(2));
    assert_raw_same(opaque_owner(&included), excluded);
    assert_eq!(included["mutation_analysis"]["excluded_functions"], 0);
    assert_eq!(
        included["mutation_analysis"]["policy_unavailable_functions"],
        1
    );
}

#[test]
fn excluded_numeric_breaches_remain_observable_but_have_no_measured_policy_actual() {
    let directory = workspace(WRITES);
    let (_, baseline) = requested(directory.path(), &[]);
    policy(directory.path(), &["src/lib.rs"]);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success());
    counts(&report, 2, 0, 0);
    assert_eq!(report["mutation_analysis"]["excluded_findings"], 2);
    assert_raw_same(function(&report), function(&baseline));
    assert_eq!(function(&report)["evidence"]["write_sites"]["value"], 6);
    for id in ["mutation_per_target", "mutated_targets"] {
        assert_eq!(decision(function(&report), id)["disposition"], "excluded");
        assert!(decision(function(&report), id)["actual"].is_null());
    }
}

#[test]
fn lib_and_test_owners_share_path_selection_without_merging_owner_counts() {
    let directory = two_files();
    policy(directory.path(), &["src/opaque.rs"]);
    let (output, report) = requested(directory.path(), &["--all-targets"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["mutation_analysis"]
    );
    let analysis = &report["mutation_analysis"];
    // Two authored functions appear in both units; rustc also emits a test harness main.
    assert_eq!(analysis["expected_functions"], 5);
    assert_eq!(analysis["excluded_functions"], 2);
    assert_eq!(analysis["policy_evaluated_functions"], 2);
    assert_eq!(analysis["policy_unavailable_functions"], 1);
    let harness = analysis["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == "main")
        .unwrap();
    assert_eq!(harness["availability"], "unsupported");
    assert_eq!(harness["policy"]["coverage"], "incomplete");
    assert_eq!(
        harness["policy"]["source_exclusion"]["status"],
        "unavailable"
    );
    let excluded = analysis["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| function["policy"]["source_exclusion"]["status"] == "excluded")
        .collect::<Vec<_>>();
    assert_eq!(excluded.len(), 2);
    assert_ne!(excluded[0]["key"], excluded[1]["key"]);
}

#[test]
fn invalid_mutation_exclusion_globs_reject_before_analysis() {
    let directory = workspace(WRITES);
    policy(directory.path(), &["["]);
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--mutation-rules",
            "--mutation-config",
            "mutation.toml",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("glob") || error.contains("pattern"),
        "{error}"
    );
}

#[test]
fn mutation_exclusions_leave_source_findings_effects_and_parameter_plans_independent() {
    let source = WRITES
        .replace("many()", "many(left:u32,right:u32)")
        .replace("first + second }", "first + second + left + right }");
    let directory = workspace(&source);
    policy(directory.path(), &["src/lib.rs"]);
    std::fs::write(
        directory.path().join("analysis.toml"),
        "exclude=[]\n[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = requested(
        directory.path(),
        &[
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--mutation-effects",
            "--parameter-plans",
            "--fail-on-findings",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        report["parameter_planning"]
    );
    counts(&report, 2, 0, 0);
    assert_eq!(report["analysis"]["visible_findings"], 1);
    assert_eq!(
        report["mutation_effects"]["functions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(report["mutation_effects"]["coverage"], "complete");
    assert_eq!(report["parameter_planning"]["findings"], 1);
}
