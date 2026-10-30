use super::*;

const PERMUTED: &str = "pub fn many(a: u32, b: u32, c: u32) -> u32 { a + b + c }\n\
pub fn caller(a: u32, b: u32, c: u32) -> u32 {\n\
    let first = many(a, b, c);\n\
    let second = many(c, a, b);\n\
    first + second\n}\n";

fn bundle_for<'a>(plan: &'a Value, positions: &[u32]) -> &'a Value {
    plan["forwarding_bundles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|bundle| bundle["target_parameters"] == serde_json::json!(positions))
        .unwrap_or_else(|| panic!("Missing positions {positions:?}: {plan}"))
}

fn caller_index(plan: &Value, source: &str, call: &str) -> usize {
    let calls = plan["callers"].as_array().unwrap();
    calls
        .iter()
        .position(|caller| original_text(source, &caller["region"]["span"]) == call)
        .unwrap()
}

fn assert_witness(bundle: &Value, index: usize, roots: &[u32]) {
    let witnesses = bundle["witnesses"].as_array().unwrap();
    assert!(
        witnesses.iter().any(|witness| {
            witness["caller_index"] == serde_json::json!(index)
                && witness["caller_formals"] == serde_json::json!(roots)
        }),
        "Missing exact call witness at index {index}: {bundle}"
    );
}

#[test]
fn repeated_scalar_forwarding_preserves_permuted_caller_roots_and_target_positions() {
    let directory = workspace(PERMUTED);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    let plan = plan(&report, "::many");
    assert_eq!(plan["bundle_support_coverage"], "complete");
    let bundle = bundle_for(plan, &[0, 1, 2]);
    assert_eq!(bundle["witnesses"].as_array().unwrap().len(), 2);
    assert_witness(
        bundle,
        caller_index(plan, PERMUTED, "many(a, b, c)"),
        &[0, 1, 2],
    );
    assert_witness(
        bundle,
        caller_index(plan, PERMUTED, "many(c, a, b)"),
        &[2, 0, 1],
    );
    assert_eq!(
        report["parameter_planning"]["whole_plan_independence"],
        "unknown"
    );
}

#[test]
fn repeated_position_intersection_retains_only_unchanged_root_support() {
    let source = "pub fn many(a: u32, b: u32, c: u32) -> u32 { a + b + c }\n\
        pub fn caller(a: u32, b: u32, c: u32) -> u32 {\n\
            let first = many(a, b, c);\n\
            let second = many(b, a, 17);\n\
            first + second\n\
        }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    let plan = plan(&report, "::many");
    let bundle = bundle_for(plan, &[0, 1]);
    assert_eq!(bundle["witnesses"].as_array().unwrap().len(), 2);
    assert_witness(bundle, caller_index(plan, source, "many(a, b, c)"), &[0, 1]);
    assert_witness(
        bundle,
        caller_index(plan, source, "many(b, a, 17)"),
        &[1, 0],
    );
    assert!(
        !plan["forwarding_bundles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|bundle| bundle["target_parameters"] == serde_json::json!([0, 1, 2]))
    );
}

#[test]
fn duplicate_roots_literals_aliases_and_changed_values_do_not_form_forwarding_bundles() {
    let source = "pub fn many(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn duplicate(value: u32) -> u32 {\n\
            let first = many(value, value); let second = many(value, value); first + second\n\
        }\n\
        pub fn literals() -> u32 { let first = many(1, 2); let second = many(3, 4); first + second }\n\
        pub fn aliases(a: u32, b: u32) -> u32 {\n\
            let left = a; let right = b;\n\
            let first = many(left, right); let second = many(right, left); first + second\n\
        }\n\
        pub fn transformed(a: u32, b: u32) -> u32 {\n\
            let first = many(a + 1, b); let second = many(a + 2, b); first + second\n\
        }\n\
        pub fn mutable(mut a: u32, mut b: u32) -> u32 {\n\
            a += 1; b += 1;\n\
            let first = many(a, b); let second = many(b, a); first + second\n\
        }\n\
        pub fn mutable_unwritten(mut a: u32, mut b: u32) -> u32 {\n\
            let first = many(a, b); let second = many(b, a); first + second\n\
        }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(matches!(output.status.code(), Some(0 | 2)));
    assert!(
        plan(&report, "::many")["forwarding_bundles"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn borrowed_formals_do_not_become_unchanged_scalar_bundle_roots() {
    let directory = workspace(
        "pub fn many(a: &u32, b: &u32) -> u32 { *a + *b }\n\
         pub fn caller(a: &u32, b: &u32) -> u32 {\n\
             let first = many(a, b); let second = many(b, a); first + second\n\
         }\n",
    );
    let (output, report) = requested(directory.path(), &[]);
    assert!(matches!(output.status.code(), Some(0 | 2)));
    let plan = plan(&report, "::many");
    assert!(plan["forwarding_bundles"].as_array().unwrap().is_empty());
    assert_eq!(plan["bundle_support_coverage"], "incomplete");
}

#[test]
fn separate_exact_targets_cannot_pool_single_call_support_for_a_bundle() {
    let directory = workspace(
        "pub mod first { pub fn target(a: u32, b: u32) -> u32 { a + b } }\n\
         pub mod second { pub fn target(a: u32, b: u32) -> u32 { a + b } }\n\
         pub fn caller(a: u32, b: u32) -> u32 {\n\
             let first = first::target(a, b);\n\
             let second = second::target(a, b); first + second\n\
         }\n",
    );
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    for name in ["first::target", "second::target"] {
        let plan = plan(&report, name);
        assert_eq!(plan["callers"].as_array().unwrap().len(), 1);
        assert!(plan["forwarding_bundles"].as_array().unwrap().is_empty());
    }
}

#[test]
fn useful_repeated_bundle_witnesses_survive_an_incomplete_caller_census() {
    let source = "pub fn many(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn caller(a: u32, b: u32) -> u32 {\n\
            let first = many(a, b); let second = many(b, a);\n\
            { let selected = many; let _ = selected(a, b); }\n\
            first + second\n\
        }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_scope(&report["parameter_planning"], "incomplete");
    let plan = plan(&report, "::many");
    let bundle = bundle_for(plan, &[0, 1]);
    assert_witness(bundle, caller_index(plan, source, "many(a, b)"), &[0, 1]);
    assert_witness(bundle, caller_index(plan, source, "many(b, a)"), &[1, 0]);
    assert_eq!(plan["bundle_support_coverage"], "incomplete");
}

#[test]
fn source_free_replay_preserves_bundle_evidence_in_full_and_compact_reports() {
    let directory = workspace(PERMUTED);
    let (output, original) = requested(directory.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        original["parameter_planning"]
    );
    bundle_for(plan(&original, "::many"), &[0, 1, 2]);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, &["--report-detail", detail]);
        assert!(output.status.success(), "{}", report["parameter_planning"]);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["parameter_planning"], original["parameter_planning"]);
    }
}
