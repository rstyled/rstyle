//! A refutable declaration evaluates its initializer once, then runs one continuation.
use super::*;

const SOURCE: &str = r#"
fn spin(n: usize) { for _ in 0..n {} }
fn inspect(n: usize, value: Option<usize>) -> Option<usize> { spin(n); value }
pub fn scan(xs: &[Option<usize>], n: usize) {
    for x in xs { let Some(_) = inspect(n, *x) else { continue; }; }
}
pub fn either(value: Option<usize>, n: usize, m: usize) {
    let Some(_) = inspect(n, value) else { spin(m); return; };
    spin(3);
}
pub fn early_return(n: usize, value: Option<usize>) {
    let mut i = 0; while i < n { let Some(_) = value else { return; }; i += 1; }
}
pub fn bypasses_progress(n: usize, value: Option<usize>) {
    let mut i = 0; while i < n { let Some(_) = value else { continue; }; i += 1; }
}
pub fn resets_progress(n: usize, value: Option<usize>) {
    let mut i = 0; while i < n { let Some(_) = ({ i = 0; value }) else { return; }; i += 1; }
}
pub fn unresolved_initializer(n: usize, f: fn() -> Option<usize>) {
    for _ in 0..n { let Some(_) = f() else { continue; }; }
}
pub fn extracted_limit(value: Option<usize>) {
    let Some(limit) = value else { return; }; for _ in 0..limit {}
}
pub fn exclusive_pattern(n: usize, mut value: Option<usize>) {
    for _ in 0..n { let Some(ref mut inner) = value else { continue; }; *inner += 1; }
}
pub struct Guard;
impl Drop for Guard { fn drop(&mut self) { spin(2); } }
fn make_guard() -> Option<Guard> { Some(Guard) }
pub fn cleanup(n: usize) {
    for _ in 0..n { let Some(_guard) = make_guard() else { continue; }; }
}
"#;

fn report() -> Value {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let output = rstyle(
        directory.path(),
        &["patterns", "--format", "json", "--complexity"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    serde_json::from_slice(&output.stdout).unwrap()
}

fn loop_work(row: &Value, inputs: &[(&str, u64)]) -> u64 {
    let composition = &row["composed"]["Proven"];
    let bound: rstyle_core::runtime_bounds::Bound =
        serde_json::from_value(composition["loop_body_entries"].clone())
            .unwrap_or_else(|_| panic!("{row}"));
    bound
        .evaluate(&|name| inputs.iter().find(|(key, _)| *key == name).map(|(_, n)| *n))
        .unwrap()
}

#[test]
fn refutable_declarations_preserve_initializer_and_branch_work() {
    let report = report();
    let rows = runtime(&report)["rows"].as_array().unwrap();
    let scan = named_row(rows, "scan");
    let either = named_row(rows, "either");

    // Three initializer invocations each run spin(2), in addition to the outer scan.
    assert_eq!(loop_work(scan, &[("len(xs)#0", 3), ("n", 2)]), 9);
    assert_eq!(
        scan["result"]["Proven"]["loops"].as_array().unwrap().len(),
        1
    );
    // Initializer work plus a conservative sum of the two branch contributions.
    assert_eq!(loop_work(either, &[("n", 2), ("m", 5)]), 10);
    assert_eq!(loop_work(named_row(rows, "early_return"), &[("n", 7)]), 7);
}

#[test]
fn refutable_declarations_do_not_invent_progress_values_or_callees() {
    let report = report();
    let rows = runtime(&report)["rows"].as_array().unwrap();
    for name in ["bypasses_progress", "resets_progress"] {
        let row = named_row(rows, name);
        assert!(row["result"].get("Partial").is_some(), "{name}: {row}");
        assert!(row["composed"].get("Proven").is_none(), "{name}: {row}");
    }
    let extracted = named_row(rows, "extracted_limit");
    assert!(extracted["result"].get("Proven").is_none(), "{extracted}");
    let cleanup = named_row(rows, "cleanup");
    assert!(cleanup["result"].get("Unknown").is_some(), "{cleanup}");
    let unresolved = named_row(rows, "unresolved_initializer");
    assert!(unresolved["result"].get("Proven").is_some(), "{unresolved}");
    assert!(
        unresolved["composed"]["Partial"]["opaque_factors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|factor| factor["kind"] == "Call"),
        "{unresolved}"
    );
    let exclusive = named_row(rows, "exclusive_pattern");
    assert!(exclusive["result"].get("Unknown").is_some(), "{exclusive}");
}
