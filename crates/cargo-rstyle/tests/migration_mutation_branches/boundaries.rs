use super::*;

#[test]
fn alias_redirection_and_opaque_branch_forms_never_become_clean_inventories() {
    for source in [
        "pub fn branch(target:&mut u32,flag:bool) { if flag { let alias=&mut *target; *alias=1; } }",
        "pub fn branch<'a>(mut target:&'a mut u32,other:&'a mut u32,flag:bool) { if flag { target=other; } *target=1; }",
        "pub fn branch(target:&mut u32,flag:bool) { if flag { std::hint::black_box(0u32); } }",
        "pub fn branch(target:&mut u32,flag:bool) { if flag { let deferred=|| 1u32; let _=deferred; } }",
        "pub fn branch(target:&mut u32,flag:bool) { if flag { #[cfg(any())] let value=1u32; } }",
        "pub fn branch(target:&mut u32) { if let Some(value)=Some(1u32) { *target=value; } }",
    ] {
        let directory = workspace(source);
        let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            report["mutation_analysis"]
        );
        let function = function(&report, "branch");
        assert_eq!(function["coverage"], "incomplete");
        assert_eq!(function["evidence"]["write_sites"]["kind"], "lower_bound");
        assert!(
            !function["evidence"]["boundaries"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_ne!(
            rule(function, "mutation_per_target")["outcome"],
            "evaluated_no_finding"
        );
        assert_eq!(function["density"]["outcome"], "not_evaluated");
    }
}

#[test]
fn checked_writes_before_an_unknown_branch_preserve_a_lower_bound_breach() {
    let source = "pub fn branch(flag:bool,callback:fn()) { let mut value=0u32; value=1; value=2; value=3; if flag { callback(); } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2));
    let function = function(&report, "branch");
    assert_eq!(function["coverage"], "incomplete");
    assert_eq!(function["evidence"]["write_sites"]["kind"], "lower_bound");
    assert_eq!(function["evidence"]["write_sites"]["value"], 3);
    assert_eq!(rule(function, "mutation_per_target")["outcome"], "finding");
    assert_eq!(
        rule(function, "mutated_targets")["outcome"],
        "not_evaluated"
    );
}

#[test]
fn completed_direct_branches_do_not_certify_conditional_effects_or_formal_uses() {
    let source = "pub fn many(first:u32,second:u32)->u32 { let mut value=first; if second>0 { value=1; } else { value=2; } value }\n";
    let directory = workspace(source);
    std::fs::write(
        directory.path().join("analysis.toml"),
        "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=1\n",
    )
    .unwrap();
    let (output, report) = requested(
        directory.path(),
        &[
            "--mutation-effects",
            "--source-rules",
            "--analysis-config",
            "analysis.toml",
            "--parameter-plans",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert_exact(
        function(&report, "many"),
        Counts {
            sites: 2,
            roots: 1,
            escaped: 0,
        },
    );
    assert_eq!(report["mutation_analysis"]["coverage"], "complete");
    assert_eq!(report["mutation_effects"]["coverage"], "incomplete");
    assert_eq!(report["parameter_planning"]["coverage"], "incomplete");
    let uses = &report["parameter_planning"]["caller_census"][0]["parameter_uses"];
    assert_eq!(uses["coverage"], "incomplete");
    assert!(
        uses["functions"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|function| function["boundaries"].as_array().unwrap())
            .any(|boundary| boundary["reason"]
                == "Conditional control flow leaves formal-use coverage incomplete")
    );
}

#[test]
fn unassessed_branch_owners_remain_budget_exhausted_instead_of_empty_and_clean() {
    let directory = workspace(BRANCH);
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    let analysis = &report["mutation_analysis"];
    assert_eq!(analysis["execution"], "budget_exhausted");
    assert_eq!(analysis["coverage"], "incomplete");
    assert_eq!(analysis["expected_functions"], 1);
    assert_eq!(analysis["assessed_functions"], 0);
    let function = function(&report, "branch");
    assert!(function["evidence"].is_null());
    assert_eq!(
        rule(function, "mutation_per_target")["outcome"],
        "not_evaluated"
    );
}

#[test]
fn compound_assignments_inside_a_condition_never_become_a_pure_zero_write_inventory() {
    let source = "pub fn branch(mut first:u32,mut second:u32) { first=1; first=2; first=3; if (first += 1) == (second += 1) { } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["mutation_analysis"]
    );
    let function = function(&report, "branch");
    assert_eq!(function["coverage"], "incomplete");
    assert_eq!(function["evidence"]["write_sites"]["kind"], "lower_bound");
    assert!(
        function["evidence"]["write_sites"]["value"]
            .as_u64()
            .unwrap()
            >= 3
    );
    assert!(
        !function["evidence"]["boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(rule(function, "mutation_per_target")["outcome"], "finding");
    let source = slice(source, &function["fn_span"]);
    let sites = function["evidence"]["sites"].as_array().unwrap();
    for (site, expected) in sites.iter().take(3).zip(["first=1", "first=2", "first=3"]) {
        assert_eq!(slice(source, &site["span"]), expected);
        assert_eq!(site["root"]["formal"], 0);
        assert_eq!(site["root"]["provenance"], "formal_owned");
    }
}
