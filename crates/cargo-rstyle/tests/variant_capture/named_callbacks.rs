use super::*;

mod corruption;

const SOURCE: &str = r#"
    pub fn scalar(byte: u8) -> bool { byte == b'_' }
    pub fn eager(mut value: Option<u8>, mut fallback: bool) -> bool {
        value.map_or({ value = None; fallback = true; fallback }, scalar)
    }
    pub fn merged(value: Option<u8>, pick: bool) -> bool {
        value.map_or(if pick { true } else { false }, scalar)
    }
    pub fn nested(value: Option<u8>, other: Option<u8>) -> bool {
        value.map_or(other.map_or(false, scalar), scalar)
    }
    pub fn receiver(mut value: Option<u8>, fallback: bool) -> bool {
        ({ value = Some(95); value }).map_or(fallback, scalar)
    }
    pub fn pointer(value: Option<u8>, callback: fn(u8) -> bool) -> bool {
        value.map_or(false, callback)
    }
    pub fn stored(value: Option<u8>) -> bool {
        let callback = scalar; value.map_or(false, callback)
    }
    pub fn coerced(value: Option<u8>) -> bool {
        value.map_or(false, scalar as fn(u8) -> bool)
    }
    pub fn panicking(byte: u8) -> bool { 100 / byte == 0 }
    pub fn through_panic(value: Option<u8>) -> bool { value.map_or(false, panicking) }
    pub fn generic<T>(_: T) -> bool { true }
    pub fn through_generic(value: Option<u8>) -> bool { value.map_or(false, generic::<u8>) }
"#;

fn captured() -> (tempfile::TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn named_callbacks_compose_with_eager_rebinding_merges_and_nested_invocations() {
    let (_directory, report) = captured();
    assert_eq!(report["functions"].as_array().unwrap().len(), 12);
    for function in report["functions"].as_array().unwrap() {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "scalar" | "eager" | "merged" | "nested" | "receiver" | "panicking"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{}: {}",
            function["facts"]["name"],
            function["reasons"]
        );
    }
    super::word_classifiers::verify_admissions(&report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
}
