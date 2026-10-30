use super::*;

#[test]
fn zero_and_nonzero_u32_ranges_count_the_original_body_site_once() {
    for endpoint in ["0u32", "1u32", "100u32", "count"] {
        let source = format!(
            "pub fn repeated(target:&mut u32,count:u32) {{ for _ in 0u32..{endpoint} {{ *target=1; }} }}\n"
        );
        let directory = workspace(&source);
        let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
        assert!(
            output.status.success(),
            "{endpoint}: {}",
            report["mutation_analysis"]
        );
        let function = function(&report, "repeated");
        assert_exact(
            function,
            Counts {
                sites: 1,
                roots: 1,
                escaped: 1,
            },
        );
        let site = &function["evidence"]["sites"][0];
        let source = slice(&source, &function["fn_span"]);
        assert_eq!(slice(source, &site["span"]), "*target=1");
        assert_eq!(slice(source, &site["root"]["declaration_span"]), "target");
        assert_eq!(site["root"]["formal"], 0);
        assert_eq!(site["root"]["provenance"], "formal_borrowed");
        assert_eq!(site["caller_visible"], true);
    }
}

#[test]
fn immutable_scalar_range_binders_are_typed_reads_and_do_not_become_write_roots() {
    let source = "pub fn repeated(target:&mut u32,start:u32,end:u32) { for index in start..end { *target=index; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    let function = function(&report, "repeated");
    assert_exact(
        function,
        Counts {
            sites: 1,
            roots: 1,
            escaped: 1,
        },
    );
    let root = &function["evidence"]["sites"][0]["root"];
    assert_eq!(root["formal"], 0);
    assert_eq!(slice(source, &root["declaration_span"]), "target");
}

#[test]
fn while_and_loop_count_sites_after_plain_break_and_continue_without_executing_them() {
    let source = "pub fn stopped(target:&mut u32) { loop { *target=1; break; *target=2; } }\npub fn continued(target:&mut u32,flag:bool) { while flag { *target=1; continue; *target=2; } }\npub fn compared(target:&mut u32,value:u32) { while value>0 { *target=3; break; } }\npub fn never(target:&mut u32) { while false { *target=4; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    for (name, sites) in [
        ("stopped", 2),
        ("continued", 2),
        ("compared", 1),
        ("never", 1),
    ] {
        assert_exact(
            function(&report, name),
            Counts {
                sites,
                roots: 1,
                escaped: sites,
            },
        );
    }
    let stopped = function(&report, "stopped");
    let source = slice(source, &stopped["fn_span"]);
    assert_eq!(
        slice(source, &stopped["evidence"]["sites"][1]["span"]),
        "*target=2"
    );
}

#[test]
fn nested_branches_and_loops_preserve_each_authored_assignment_once() {
    let source = "pub fn repeated(target:&mut u32,count:u32,flag:bool) { for _ in 0u32..count { if flag { *target=1; } else { for _ in 0u32..count { *target=2; } } } if flag { loop { *target=3; break; } } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &["--fail-on-findings"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        report["mutation_analysis"]
    );
    let function = function(&report, "repeated");
    assert_exact(
        function,
        Counts {
            sites: 3,
            roots: 1,
            escaped: 3,
        },
    );
    assert_eq!(rule(function, "mutation_per_target")["outcome"], "finding");
    let sites = function["evidence"]["sites"].as_array().unwrap();
    assert_eq!(sites[0]["root"], sites[1]["root"]);
    assert_eq!(sites[1]["root"], sites[2]["root"]);
}

#[test]
fn loop_scopes_preserve_shadowed_local_roots_and_restore_the_outer_binding() {
    let source = "pub fn repeated(count:u32)->u32 { let mut value=0u32; value=1; for index in 0u32..count { let mut value=index; value=2; } value=3; value }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    let function = function(&report, "repeated");
    assert_exact(
        function,
        Counts {
            sites: 3,
            roots: 2,
            escaped: 0,
        },
    );
    let sites = function["evidence"]["sites"].as_array().unwrap();
    assert_eq!(sites[0]["root"], sites[2]["root"]);
    assert_ne!(sites[0]["root"]["native"], sites[1]["root"]["native"]);
    assert_ne!(
        sites[0]["root"]["scope_span"],
        sites[1]["root"]["scope_span"]
    );
    assert_eq!(rule(function, "mutated_targets")["outcome"], "finding");
}

#[test]
fn conditional_breaks_and_continues_keep_following_original_loop_sites() {
    let source = "pub fn skipped(target:&mut u32,flag:bool) { while flag { if flag { continue; } *target=1; } }\npub fn stopped(target:&mut u32,count:u32) { for index in 0u32..count { if index>3 { break; } *target=index; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    for name in ["skipped", "stopped"] {
        assert_exact(
            function(&report, name),
            Counts {
                sites: 1,
                roots: 1,
                escaped: 1,
            },
        );
    }
}

#[test]
fn transparent_range_endpoints_keep_exact_original_correspondence() {
    let source =
        "pub fn repeated(target:&mut u32,count:u32) { for _ in (0u32)..(count) { *target=1; } }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_analysis"]);
    assert_exact(
        function(&report, "repeated"),
        Counts {
            sites: 1,
            roots: 1,
            escaped: 1,
        },
    );
}
