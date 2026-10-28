use super::*;

#[test]
fn both_branch_arms_count_as_two_static_sites_at_the_strict_rule_boundary() {
    let directory = workspace(BRANCH);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    let assessed = function(&report, "branch");
    assert_exact(
        assessed,
        Counts {
            sites: 2,
            roots: 1,
            escaped: 2,
        },
    );
    assert_eq!(
        rule(assessed, "mutation_per_target")["outcome"],
        "evaluated_no_finding"
    );
    assert_eq!(
        rule(assessed, "mutated_targets")["outcome"],
        "evaluated_no_finding"
    );
    let source = slice(BRANCH, &assessed["fn_span"]);
    let sites = assessed["evidence"]["sites"].as_array().unwrap();
    assert_eq!(slice(source, &sites[0]["span"]), "*target = 1");
    assert_eq!(slice(source, &sites[1]["span"]), "*target = 2");
    assert_eq!(sites[0]["root"], sites[1]["root"]);
    for site in sites {
        assert_eq!(site["root"]["formal"], 0);
        assert_eq!(site["root"]["provenance"], "formal_borrowed");
        assert_eq!(slice(source, &site["root"]["declaration_span"]), "target");
        assert!(!slice(source, &site["root"]["scope_span"]).is_empty());
        assert!(!site["root"]["native"].is_null());
    }
    policy(directory.path(), 1);
    let output = replay(directory.path(), &report, "compact");
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        rule(function(&report, "branch"), "mutation_per_target")["outcome"],
        "finding"
    );
}

#[test]
fn no_else_nested_and_else_if_inventories_count_each_original_site_once() {
    let source = "pub fn one(target:&mut u32,flag:bool) { if flag { *target=1; } }\npub fn nested(target:&mut u32,flag:bool,other:bool) { if flag { if other { *target=1; } else { *target=2; } } else { *target=3; } }\npub fn chained(target:&mut u32,flag:bool,other:bool) { if flag { *target=1; } else if other { *target=2; } else { *target=3; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    for (name, sites) in [("one", 1), ("nested", 3), ("chained", 3)] {
        let function = function(&report, name);
        assert_exact(
            function,
            Counts {
                sites,
                roots: 1,
                escaped: sites,
            },
        );
        assert_eq!(
            rule(function, "mutation_per_target")["outcome"],
            if sites > 2 {
                "finding"
            } else {
                "evaluated_no_finding"
            }
        );
    }
}

#[test]
fn branch_formals_remain_distinct_roots_and_scalar_conditions_add_no_writes() {
    let source = "pub fn distinct(first:&mut u32,second:&mut u32,value:u32) { if value + 1 > 2 { *first=1; } else { *second=2; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    let function = function(&report, "distinct");
    assert_exact(
        function,
        Counts {
            sites: 2,
            roots: 2,
            escaped: 2,
        },
    );
    let sites = function["evidence"]["sites"].as_array().unwrap();
    assert_ne!(sites[0]["root"]["native"], sites[1]["root"]["native"]);
    assert_eq!(sites[0]["root"]["formal"], 0);
    assert_eq!(sites[1]["root"]["formal"], 1);
    assert_eq!(rule(function, "mutated_targets")["outcome"], "finding");
}

#[test]
fn original_shadowed_local_baseline_keeps_same_spelling_roots_separate() {
    let directory = workspace(SHADOWED);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    let function = function(&report, "shadow");
    assert_exact(
        function,
        Counts {
            sites: 2,
            roots: 2,
            escaped: 0,
        },
    );
    let sites = function["evidence"]["sites"].as_array().unwrap();
    assert_ne!(sites[0]["root"]["native"], sites[1]["root"]["native"]);
    assert_ne!(
        sites[0]["root"]["declaration_span"],
        sites[1]["root"]["declaration_span"]
    );
    assert_ne!(
        sites[0]["root"]["scope_span"],
        sites[1]["root"]["scope_span"]
    );
    for site in sites {
        assert_eq!(slice(SHADOWED, &site["root"]["declaration_span"]), "value");
        assert_eq!(site["root"]["provenance"], "local_owned");
    }
}

#[test]
fn arm_local_shadowing_restores_the_outer_native_binding_after_the_join() {
    let source = "pub fn shadow(mut value:u32,flag:bool)->u32 { value=1; if flag { let mut value=2u32; value=3; } else { let mut value=4u32; value=5; } value=6; value }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    let function = function(&report, "shadow");
    assert_exact(
        function,
        Counts {
            sites: 4,
            roots: 3,
            escaped: 0,
        },
    );
    let sites = function["evidence"]["sites"].as_array().unwrap();
    assert_eq!(sites[0]["root"], sites[3]["root"]);
    assert_ne!(sites[1]["root"]["native"], sites[2]["root"]["native"]);
    assert_ne!(sites[0]["root"]["native"], sites[1]["root"]["native"]);
    assert_eq!(rule(function, "mutated_targets")["outcome"], "finding");
}

#[test]
fn parenthesized_scalar_conditions_preserve_native_branch_correspondence() {
    let source = "pub fn flag(target:&mut u32,flag:bool) { if (flag) { *target=1; } else { *target=2; } }\npub fn compare(target:&mut u32,value:u32) { if (value > 0) { *target=3; } }\npub fn nested(target:&mut u32,value:u32) { if ((value + 1) > 2) { *target=4; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    for (name, sites) in [("flag", 2), ("compare", 1), ("nested", 1)] {
        assert_exact(
            function(&report, name),
            Counts {
                sites,
                roots: 1,
                escaped: sites,
            },
        );
    }
}
