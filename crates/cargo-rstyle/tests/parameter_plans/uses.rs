use super::*;

const FORWARDING: &str = "pub fn target(a: u32, b: u32) -> u32 { a + b }\n\
pub fn wrapper(a: u32, b: u32) -> u32 { target(a, b) }\n\
pub fn repeated(a: u32, b: u32) -> u32 {\n\
    let first = target(a, b); let second = target(b, a); first + second\n\
}\n";

fn function_uses<'a>(report: &'a Value, name: &str) -> &'a Value {
    let function = report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap();
    report["parameter_planning"]["caller_census"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| unit["parameter_uses"]["functions"].as_array().unwrap())
        .find(|uses| uses["function"] == function["key"])
        .unwrap_or_else(|| panic!("Missing formal inventory for {name}"))
}

fn formal(uses: &Value, index: u32) -> &Value {
    uses["formals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|formal| formal["formal"] == index)
        .unwrap()
}

fn assert_outcomes(report: &Value, name: &str, expected: &[&str]) {
    let uses = function_uses(report, name);
    assert_eq!(uses["expected_formals"], serde_json::json!(expected.len()));
    assert_eq!(uses["formals"].as_array().unwrap().len(), expected.len());
    for (index, expected) in expected.iter().enumerate() {
        assert_eq!(
            formal(uses, index as u32)["outcome"],
            *expected,
            "{name}: {uses}"
        );
    }
}

#[test]
fn pass_through_candidates_require_every_original_formal_use_to_be_a_selected_call_argument() {
    let directory = workspace(FORWARDING);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    for (name, count) in [("wrapper", 1), ("repeated", 2)] {
        let uses = function_uses(&report, name);
        assert_eq!(uses["coverage"], "complete");
        assert_eq!(uses["execution"], "completed");
        assert_eq!(uses["assessed_formals"], 2);
        assert_outcomes(
            &report,
            name,
            &["pass_through_candidate", "pass_through_candidate"],
        );
        for index in [0, 1] {
            let occurrences = formal(uses, index)["occurrences"].as_array().unwrap();
            assert_eq!(occurrences.len(), count);
            for occurrence in occurrences {
                assert_eq!(occurrence["role"]["kind"], "call_argument");
                assert_eq!(
                    original_text(FORWARDING, &occurrence["span"]),
                    if index == 0 { "a" } else { "b" }
                );
            }
        }
    }
    assert_outcomes(&report, "target", &["has_other_uses", "has_other_uses"]);
}

#[test]
fn ordinary_reads_shadow_initializers_and_unselected_callees_block_only_affected_formals() {
    let source = "pub fn target(a: u32, b: u32) -> u32 { a + b }\n\
        pub fn identity(value: u32) -> u32 { value }\n\
        pub fn arithmetic(a: u32, b: u32) -> u32 { target(a + 1, b) }\n\
        pub fn returned(a: u32, b: u32) -> u32 { let _ = target(a, b); a }\n\
        pub fn shadowed(a: u32, b: u32) -> u32 { let a = a; target(a, b) }\n\
        pub fn unselected(a: u32, b: u32) -> u32 {\n\
            let first = target(a, b); let next = identity(a); first + next\n\
        }\n\
        pub fn unused(a: u32, b: u32) -> u32 { 0 }\n";
    let directory = workspace(source);
    let (_, report) = requested(directory.path(), &[]);
    for name in ["arithmetic", "returned", "shadowed", "unselected"] {
        assert_eq!(function_uses(&report, name)["coverage"], "complete");
        assert_outcomes(&report, name, &["has_other_uses", "pass_through_candidate"]);
    }
    assert_outcomes(&report, "unused", &["unused", "unused"]);
    for row in function_uses(&report, "unused")["formals"]
        .as_array()
        .unwrap()
    {
        assert!(row["occurrences"].as_array().unwrap().is_empty());
    }
}

#[test]
fn mutable_owned_uses_are_inventoried_and_borrowed_formals_remain_unavailable() {
    let directory = workspace(
        "pub fn target(a: u32, b: u32) -> u32 { a + b }\n\
         pub fn mutable(mut a: u32, b: u32) -> u32 { target(a, b) }\n\
         pub fn written(mut a: u32, b: u32) -> u32 { a += 1; target(a, b) }\n\
         pub fn borrowed(a: &u32, b: &u32) -> u32 { target(*a, *b) }\n",
    );
    let (_, report) = requested(directory.path(), &[]);
    for name in ["mutable", "written"] {
        assert_eq!(function_uses(&report, name)["coverage"], "complete");
        assert_outcomes(&report, name, &["has_other_uses", "pass_through_candidate"]);
    }
    assert_outcomes(&report, "borrowed", &["unavailable", "unavailable"]);
    assert_eq!(function_uses(&report, "borrowed")["coverage"], "incomplete");
}

#[test]
fn opaque_owner_inventories_do_not_erase_independent_complete_wrapper_evidence() {
    let source = "pub fn target(a: u32, b: u32) -> u32 { a + b }\n\
        macro_rules! hidden { ($value:expr) => { $value }; }\n\
        pub fn supported(a: u32, b: u32) -> u32 { target(a, b) }\n\
        pub fn expanded(a: u32, b: u32) -> u32 {\n\
            let known = target(a, b); let hidden = hidden!(a); known + hidden\n\
        }\n\
        pub fn closure(a: u32, b: u32) -> u32 {\n\
            let known = target(a, b); let hidden = || a; known + hidden()\n\
        }\n\
        pub fn control(a: u32, b: u32) -> u32 {\n\
            let known = target(a, b); if a > 0 { known } else { b }\n\
        }\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        report["parameter_planning"]
    );
    assert_outcomes(
        &report,
        "supported",
        &["pass_through_candidate", "pass_through_candidate"],
    );
    assert_eq!(function_uses(&report, "supported")["coverage"], "complete");
    for name in ["expanded", "closure", "control"] {
        let uses = function_uses(&report, name);
        assert_eq!(uses["coverage"], "incomplete", "{name}: {uses}");
        assert_outcomes(&report, name, &["unavailable", "unavailable"]);
        assert!(
            formal(uses, 0)["occurrences"]
                .as_array()
                .unwrap()
                .iter()
                .any(|occurrence| original_text(source, &occurrence["span"]) == "a")
        );
    }
}

#[test]
fn strict_source_free_replay_retains_parameter_use_outcomes_and_original_occurrences() {
    let directory = workspace(FORWARDING);
    let (output, original) = requested(directory.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        original["parameter_planning"]
    );
    assert_outcomes(
        &original,
        "wrapper",
        &["pass_through_candidate", "pass_through_candidate"],
    );
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, &["--report-detail", detail]);
        assert!(output.status.success(), "{}", report["parameter_planning"]);
        assert_eq!(report["verification_policy"], "strict-replay");
        assert_eq!(report["parameter_planning"], original["parameter_planning"]);
    }
}

#[test]
fn formal_declarations_and_occurrences_keep_original_utf8_crlf_offsets() {
    let source = "// λ shifts original file offsets\r\npub fn target(left: u32, right: u32) -> u32 { left + right }\r\npub fn wrapper(left: u32, right: u32) -> u32 {\r\n    /* λ */ target(left, right)\r\n}\r\n";
    let directory = workspace(source);
    let (output, report) = requested(directory.path(), &[]);
    assert!(output.status.success(), "{}", report["parameter_planning"]);
    let uses = function_uses(&report, "wrapper");

    for (index, name) in [(0, "left"), (1, "right")] {
        let formal = formal(uses, index);
        assert_eq!(formal["outcome"], "pass_through_candidate");
        assert_eq!(original_text(source, &formal["declaration"]), name);
        let occurrences = formal["occurrences"].as_array().unwrap();
        assert_eq!(occurrences.len(), 1);
        assert_eq!(original_text(source, &occurrences[0]["span"]), name);
        assert_eq!(
            original_text(source, &occurrences[0]["selected_call"]["span"]),
            "target(left, right)"
        );
    }
}
