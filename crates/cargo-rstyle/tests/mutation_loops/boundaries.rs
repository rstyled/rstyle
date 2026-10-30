use super::*;

#[test]
fn unsupported_loop_iterators_labels_value_breaks_and_mutable_binders_are_not_clean() {
    for source in [
        "pub fn repeated(target:&mut u32) { for value in [1u32,2u32].into_iter() { *target=value; } }",
        "pub fn repeated(target:&mut u32) { 'again: loop { *target=1; break 'again; } }",
        "pub fn repeated(target:&mut u32) { let _ = loop { *target=1; break 7u32; }; }",
        "pub fn repeated(target:&mut u32) { while let Some(value)=Some(0u32) { *target=value; break; } }",
        "pub fn repeated(target:&mut u32,count:u32) { for mut index in 0u32..count { index+=1; *target=index; } }",
        "pub fn repeated(target:&mut u32,count:u32) { for _ in 0u32..=count { *target=1; } }",
        "pub fn repeated(target:&mut u32,count:i32) { for _ in 0i32..count { *target=1; } }",
    ] {
        let directory = workspace(source);
        let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            report["mutation_analysis"]
        );
        let function = function(&report, "repeated");
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
    }
}

#[test]
fn unknown_loop_bodies_keep_preceding_checked_writes_as_a_lower_bound_breach() {
    let source = "pub fn repeated(flag:bool,callback:fn()) { let mut value=0u32; value=1; value=2; value=3; while flag { callback(); } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2));
    let function = function(&report, "repeated");
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
fn loop_aliases_and_reference_redirection_do_not_restore_stale_borrowed_roots() {
    for source in [
        "pub fn repeated(target:&mut u32,count:u32) { for _ in 0u32..count { let alias=&mut *target; *alias=1; } }",
        "pub fn repeated<'a>(mut target:&'a mut u32,other:&'a mut u32,flag:bool) { while flag { target=other; break; } *target=1; }",
    ] {
        let directory = workspace(source);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2));
        let function = function(&report, "repeated");
        assert_eq!(function["coverage"], "incomplete");
        assert!(
            function["evidence"]["sites"]
                .as_array()
                .unwrap()
                .iter()
                .all(|site| site["root"]["provenance"] != "formal_borrowed")
        );
    }
}

#[test]
fn checked_loop_effects_leave_formal_use_coverage_incomplete() {
    let source = "pub fn many(first:u32,second:u32)->u32 { let mut value=first; for _ in 0u32..second { value=1; } value }\n";
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
            sites: 1,
            roots: 1,
            escaped: 0,
        },
    );
    assert_eq!(report["mutation_analysis"]["coverage"], "complete");
    assert_eq!(report["mutation_effects"]["coverage"], "complete");
    assert_eq!(
        report["mutation_effects"]["functions"][0]["evidence"]["caller_visible_mutation"],
        "none_proven"
    );
    assert_eq!(report["parameter_planning"]["coverage"], "incomplete");
    assert_eq!(
        report["parameter_planning"]["caller_census"][0]["parameter_uses"]["coverage"],
        "incomplete"
    );
}

#[test]
fn unassessed_loop_owners_are_budget_exhausted_and_never_clean() {
    let directory = workspace(FOR_LOOP);
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    let analysis = &report["mutation_analysis"];
    assert_eq!(analysis["execution"], "budget_exhausted");
    assert_eq!(analysis["expected_functions"], 1);
    assert_eq!(analysis["assessed_functions"], 0);
    let function = function(&report, "repeated");
    assert!(function["evidence"].is_null());
    assert_eq!(
        rule(function, "mutation_per_target")["outcome"],
        "not_evaluated"
    );
}

#[test]
fn original_loop_source_syntax_exhaustion_is_not_an_empty_complete_inventory() {
    let source = format!(
        "pub fn repeated(target:&mut u32,count:u32) {{ // {}\nfor _ in 0u32..count {{ *target=1; }} }}\n",
        "word ".repeat(140),
    );
    let directory = workspace(&source);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    let function = function(&report, "repeated");
    assert_eq!(function["execution"], "budget_exhausted");
    assert_eq!(function["coverage"], "incomplete");
    assert_eq!(function["evidence"]["write_sites"]["kind"], "lower_bound");
    assert_ne!(
        rule(function, "mutation_per_target")["outcome"],
        "evaluated_no_finding"
    );
}

#[test]
fn macro_origin_for_and_while_bodies_do_not_become_direct_source_inventory() {
    for body in [
        "for _ in 0u32..count { *target=1; }",
        "while count>0 { *target=1; break; }",
    ] {
        let source = format!(
            "pub fn repeated(target:&mut u32,count:u32) {{ macro_rules! repeated {{()=>{{{body}}}}} repeated!(); }}"
        );
        let directory = workspace(&source);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2));
        let function = function(&report, "repeated");
        assert_eq!(function["coverage"], "incomplete");
        assert_ne!(
            rule(function, "mutation_per_target")["outcome"],
            "evaluated_no_finding"
        );
        assert!(function["evidence"]["sites"].as_array().unwrap().is_empty());
    }
}
