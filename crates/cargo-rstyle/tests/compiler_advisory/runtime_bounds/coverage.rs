use super::*;
use serde_json::json;

const SOURCE: &str = r#"
pub fn empty() {}
pub fn scan(xs: &[u8]) { for _ in xs {} }
pub fn partial(flag: bool) { while flag {} }
pub fn unknown(value: &usize) { unsafe { std::ptr::read_volatile(value); } }
pub fn library() { std::thread::yield_now(); }
macro_rules! make { () => { pub fn generated() {} } }
make!();
"#;

fn capture() -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let output = rstyle(
        directory.path(),
        &["patterns", "--format", "json", "--complexity"],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (directory, report)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "patterns",
        "--format",
        "json",
        "--complexity",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let output = rstyle(directory, &args);
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}

#[test]
fn compiler_bounds_count_the_assessed_roster_before_selecting_rows() {
    let (directory, original) = capture();
    let bounds = runtime(&original);
    assert_eq!(bounds["schema"], "rstyle.compiler-runtime-bounds-report/2");
    assert_eq!(bounds["input_complete"], true);
    assert_eq!(
        bounds["coverage"]["roster"],
        json!({"authored_functions": 5})
    );
    assert_eq!(bounds["coverage"]["capture"], json!({"captured": 5}));
    assert_eq!(
        bounds["coverage"]["local"],
        json!({"proven": 3, "partial": 1, "unknown": 1})
    );
    assert_eq!(
        bounds["coverage"]["proven_local_loops"],
        json!({"without_modeled_loops": 2, "input_dependent_loop_work": 1})
    );
    let rows = bounds["rows"].as_array().unwrap();
    assert!(named_row(rows, "library")["composed"]["Proven"].is_object());
    assert_eq!(bounds["coverage"]["library_steps"]["opaque"], 1);
    assert_context(rows);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let (output, selected) = replay(directory.path(), &original, &["--top", "0"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let selected = runtime(&selected);
    assert_eq!(selected["coverage"], bounds["coverage"]);
    assert_eq!(selected["input_complete"], bounds["input_complete"]);
    assert_eq!(selected["rows"], bounds["rows"]);
    assert_eq!(selected["selected"], json!([]));
}

fn assert_context(rows: &[Value]) {
    for row in rows {
        for field in ["identity", "compilation", "function", "root"] {
            assert!(!row[field].as_str().unwrap().is_empty(), "{field}: {row}");
        }
        assert!(row["line"].as_u64().unwrap() > 0, "{row}");
        assert!(row["column"].as_u64().unwrap() > 0, "{row}");
        assert!(row["test"].is_boolean(), "{row}");
    }
}

#[test]
fn compiler_bounds_name_their_actual_source_population() {
    let (directory, original) = capture();
    let bounds = runtime(&original);
    assert_eq!(
        bounds["coverage_scope"],
        json!({"roster": "assessed_functions_in_unit", "source_scope": "authored"})
    );
    let (output, captured) = replay(
        directory.path(),
        &original,
        &["--compiler-pattern-scope", "captured", "--top", "0"],
    );
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let captured = runtime(&captured);
    assert_eq!(
        captured["coverage_scope"],
        json!({"roster": "assessed_functions_in_unit", "source_scope": "captured"})
    );
    assert_eq!(
        captured["coverage"]["roster"],
        json!({"authored_functions": 5, "generated": 1})
    );
    assert_eq!(captured["coverage"]["capture"], json!({"captured": 6}));
    assert_eq!(captured["input_complete"], true);
    assert_eq!(
        captured["coverage"]["proven_local_loops"]["input_dependent_loop_work"],
        bounds["coverage"]["proven_local_loops"]["input_dependent_loop_work"]
    );
    assert_eq!(captured["rows"].as_array().unwrap().len(), 6);
    assert_eq!(captured["selected"], json!([]));
    assert_eq!(bounds["rows"].as_array().unwrap().len(), 5);
}
