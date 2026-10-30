use super::*;

#[test]
fn a_third_call_edge_is_incomplete_instead_of_assuming_a_transitive_fixed_point() {
    let source = "fn leaf(target: &mut u32) { *target += 1; }\nfn wrapper(target: &mut u32) { leaf(target); }\nfn outer(target: &mut u32) { wrapper(target); }\npub fn caller(target: &mut u32) { outer(target); }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(output.status.code(), Some(2));
    complete(&report, "outer", "writes_formal");
    assert_unknown(&report, "caller", "unknown");
    assert!(
        function(&report, "caller")["evidence"]["roots"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn assert_unknown(report: &Value, name: &str, visible: &str) {
    let evidence = &function(report, name)["evidence"];
    assert_eq!(evidence["coverage"], "incomplete", "{evidence}");
    assert_eq!(evidence["caller_visible_mutation"], visible, "{evidence}");
    assert!(
        !evidence["unresolved_boundaries"]
            .as_array()
            .unwrap()
            .is_empty()
            || !evidence["reasons"].as_array().unwrap().is_empty()
    );
}

#[test]
fn recursive_and_unknown_wrappers_preserve_the_callers_checked_direct_witness() {
    for helpers in [
        "fn wrapper(target: &mut u32) { wrapper(target); }",
        "fn first(target: &mut u32) { wrapper(target); } fn wrapper(target: &mut u32) { first(target); }",
        "fn wrapper(target: &mut u32) { *target += 2; std::hint::black_box(0u32); }",
    ] {
        let source = format!(
            "{helpers}\npub fn caller(target: &mut u32) {{ *target += 1; wrapper(target); }}\n"
        );
        let directory = workspace(&source);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(output.status.code(), Some(2));
        assert_unknown(
            &report,
            "caller",
            "witnessed_writes_formal_with_unknown_remainder",
        );
        let roots = function(&report, "caller")["evidence"]["roots"]
            .as_array()
            .unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0]["formal"], 0);
        assert_eq!(roots[0]["provenance"], "formal_borrowed");
    }
}

#[test]
fn aliases_reference_redirection_and_nominal_coercions_do_not_acquire_summary_authority() {
    for source in [
        "fn leaf(target: &mut u32) { *target += 1; } fn wrapper(target: &mut u32) { let alias = &mut *target; leaf(alias); } pub fn caller(target: &mut u32) { wrapper(target); }",
        "fn leaf(target: &mut u32) { *target += 1; } fn wrapper<'a>(mut target: &'a mut u32, other: &'a mut u32) { target = other; leaf(target); } pub fn caller(first: &mut u32, second: &mut u32) { wrapper(first, second); }",
        "pub struct Number(u32); impl core::ops::Deref for Number { type Target = u32; fn deref(&self) -> &u32 { &self.0 } } impl core::ops::DerefMut for Number { fn deref_mut(&mut self) -> &mut u32 { &mut self.0 } } fn leaf(target: &mut u32) { *target += 1; } fn wrapper(target: &mut Number) { leaf(target); } pub fn caller(target: &mut Number) { wrapper(target); }",
    ] {
        let directory = workspace(source);
        let (output, report) = requested(directory.path(), &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            report["mutation_effects"]
        );
        assert_unknown(&report, "caller", "unknown");
        assert!(
            function(&report, "caller")["evidence"]["roots"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn missing_owner_budget_never_publishes_a_clean_composed_effect() {
    let directory = workspace(REPEATED);
    let (output, report) = requested(directory.path(), &["--max-analysis-functions", "0"]);
    assert_eq!(output.status.code(), Some(2));
    let effects = &report["mutation_effects"];
    assert_eq!(effects["execution"], "budget_exhausted");
    assert_eq!(effects["coverage"], "incomplete");
    assert_eq!(effects["assessed_functions"], 0);
    assert_eq!(effects["expected_functions"], 3);
    assert!(effects["summaries"].as_array().unwrap().is_empty());
    for function in effects["functions"].as_array().unwrap() {
        assert_eq!(function["evidence"]["coverage"], "incomplete");
        assert_eq!(function["evidence"]["execution"], "budget_exhausted");
        assert_eq!(function["evidence"]["caller_visible_mutation"], "unknown");
    }
}

#[test]
fn same_named_leaf_definitions_keep_distinct_composed_write_owners() {
    let source = "mod first { pub fn leaf(target: &mut u32) { *target += 1; } }\nmod second { pub fn leaf(target: &mut u32) { *target += 2; } }\nfn wrapper(left: &mut u32, right: &mut u32) { first::leaf(left); second::leaf(right); }\npub fn caller(left: &mut u32, right: &mut u32) { wrapper(left, right); }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["mutation_effects"]);
    complete(&report, "caller", "writes_formal");
    let caller = function(&report, "caller");
    let summary = summary(&report, caller, &caller["evidence"]["calls"][0]);
    let sites = summary["sites"].as_array().unwrap();
    assert_eq!(sites.len(), 2);
    assert_ne!(sites[0]["function_index"], sites[1]["function_index"]);
    let mut projected = sites
        .iter()
        .map(|site| site["mapped_root"]["formal"].as_u64().unwrap())
        .collect::<Vec<_>>();
    projected.sort();
    assert_eq!(projected, [0, 1]);
    let mut owners = sites
        .iter()
        .map(|site| {
            owner(&report, &summary["unit"], &site["function_index"])["name"]
                .as_str()
                .unwrap()
        })
        .collect::<Vec<_>>();
    owners.sort();
    assert!(owners[0].ends_with("first::leaf"));
    assert!(owners[1].ends_with("second::leaf"));
}
