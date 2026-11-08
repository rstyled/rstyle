use super::{fixture, rstyle, stdout};
use serde_json::Value;

const SOURCE: &str = r#"#![allow(dead_code, unused_variables, unused_parens)]
pub fn straight() -> i32 { 1 }
pub fn nested(a: bool, b: bool) { if a { if b {} } }
pub fn flat(a: bool, b: bool) { if a { return; } if b { return; } }
pub fn ladder(a: bool, b: bool) { if a {} else if b {} else {} }
pub fn selection(x: u8) -> u8 { match x { 0 => 1, 1 => 2, _ => 3 } }
pub fn guarded(x: u8) -> u8 { match x { n if n > 5 => 1, 1 => 2, _ => 3 } }
pub fn boolean(a: bool, b: bool, c: bool, d: bool) -> bool { a && b && c || d && a }
pub fn for_loop(values: &[i32]) { for value in values { let _ = value; } }
pub fn while_loop(mut x: usize) { while x > 0 { x -= 1; } }
pub fn endless_loop() { loop { break; } }
pub fn propagate(value: Option<i32>) -> Option<i32> { Some(value?) }
pub fn let_else(value: Option<i32>) -> i32 { let Some(value) = value else { return 0; }; value }
pub async fn asynchronous() { async {}.await; }
pub fn closure(a: bool) { let f = || { if a {} }; f(); }
pub fn outer() { fn inner(a: bool) { if a {} } inner(true); }
pub fn jumps() { 'outer: loop { loop { break 'outer; } } }
macro_rules! branch { () => { if true {} }; }
pub fn macro_body() { branch!(); }
pub fn recursion(x: u32) -> u32 { if x == 0 { return 0; } recursion(x - 1) }
macro_rules! bind { () => { let Some(_) = Some(1) else { return; }; }; }
pub fn macro_binding() { bind!(); }
pub fn if_let(value: Option<u8>) { if let Some(_) = value {} }
pub fn bitwise(a: u8, b: u8) -> u8 { a & b | a }
macro_rules! logic { () => { true && false }; }
pub fn macro_operand(a: bool) -> bool { a && logic!() }
pub fn macro_else(a: bool) { if a {} else { branch!(); } }
#[cfg(any())] pub fn disabled(a: bool) { if a {} }
"#;

fn report() -> (tempfile::TempDir, Value) {
    let dir = fixture("clean");
    std::fs::write(dir.path().join("src/lib.rs"), SOURCE).unwrap();
    std::fs::write(
        dir.path().join("rstyle.toml"),
        "[complexity]\nmax_cyclomatic = 0\nmax_cognitive = 0\n",
    )
    .unwrap();
    let output = rstyle(dir.path(), &["--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value = serde_json::from_slice(&output.stdout).unwrap();
    (dir, value)
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"].as_str().unwrap().rsplit("::").next() == Some(name))
        .unwrap_or_else(|| panic!("missing function {name}"))
}

#[test]
fn source_constructs_have_exact_shared_scores() {
    let (_, report) = report();
    let cases = [
        ("straight", 1, 0),
        ("nested", 3, 3),
        ("flat", 3, 2),
        ("ladder", 3, 3),
        ("selection", 3, 1),
        ("guarded", 4, 3),
        ("boolean", 5, 3),
        ("for_loop", 2, 1),
        ("while_loop", 2, 1),
        ("endless_loop", 2, 1),
        ("propagate", 2, 0),
        ("let_else", 2, 1),
        ("asynchronous", 1, 0),
        ("closure", 2, 2),
        ("outer", 1, 0),
        ("inner", 2, 1),
        ("jumps", 3, 4),
        ("macro_body", 1, 0),
        ("recursion", 2, 2),
        ("macro_binding", 1, 0),
        ("if_let", 2, 1),
        ("bitwise", 1, 0),
        ("macro_operand", 2, 1),
        ("macro_else", 2, 2),
    ];
    for (name, cc, cog) in cases {
        let score = &function(&report, name)["complexity"];
        assert_eq!(
            (score["cyclomatic"].as_u64(), score["cognitive"].as_u64()),
            (Some(cc), Some(cog)),
            "{name}"
        );
    }
    assert_eq!(report["summary"]["complexity_unavailable"], 0);
    assert_eq!(report["summary"]["complexity_measured"], cases.len());
    assert_eq!(report["functions"].as_array().unwrap().len(), cases.len());
}

#[test]
fn scores_retain_evidence_and_ranking_is_deterministic() {
    let (dir, report) = report();
    let score = &function(&report, "nested")["complexity"];
    let evidence = score["contributions"].as_array().unwrap();
    assert_eq!(evidence[0]["line"], 3);
    assert_eq!(evidence[0]["nesting"], 0);
    assert_eq!(evidence[1]["nesting"], 1);
    assert_eq!(evidence[1]["cognitive"], 2);
    assert!(evidence[1]["column"].as_u64().unwrap() > evidence[0]["column"].as_u64().unwrap());
    let propagation = &function(&report, "propagate")["complexity"]["contributions"][0];
    let source = SOURCE
        .lines()
        .find(|line| line.starts_with("pub fn propagate"))
        .unwrap();
    assert_eq!(propagation["column"], source.find('?').unwrap() + 1);
    let output = rstyle(dir.path(), &["--format", "json"]);
    let repeated: Value = serde_json::from_slice(&output.stdout).unwrap();
    for field in ["functions", "diagnostics", "fix_groups", "summary"] {
        assert_eq!(repeated[field], report[field], "{field}");
    }
    assert_eq!(
        report["functions"][0]["name"]
            .as_str()
            .unwrap()
            .rsplit("::")
            .next(),
        Some("jumps")
    );
    let output = rstyle(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("CC  COG  LOCATION  FUNCTION"));
    for function in report["functions"].as_array().unwrap() {
        assert!(text.contains(function["name"].as_str().unwrap()));
    }
}

#[test]
fn configured_limits_use_existing_diagnostics_suppressions_and_groups() {
    let dir = fixture("clean");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn flat(a: bool, b: bool) { if a {} if b {} }\n// rstyle-allow(cyclomatic_complexity, cognitive_complexity): contract fixture\npub fn allowed(a: bool, b: bool) { if a {} if b {} }\n").unwrap();
    std::fs::write(
        dir.path().join("rstyle.toml"),
        "[complexity]\nmax_cyclomatic = 2\nmax_cognitive = 1\n",
    )
    .unwrap();
    let output = rstyle(dir.path(), &["--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["diagnostics"].as_array().unwrap().len(), 2);
    assert_eq!(report["fix_groups"].as_array().unwrap().len(), 1);
    assert_eq!(report["functions"].as_array().unwrap().len(), 1);
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["line"] == 1)
    );
    let output = rstyle(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(text.contains("cyclomatic_complexity"));
    assert!(text.contains("cognitive_complexity"));
    assert!(text.contains("CC  COG  LOCATION  FUNCTION"));
    assert!(text.contains("2 measured, 0 unavailable"));
}

#[test]
fn default_twelve_limits_report_only_functions_strictly_over_either_limit() {
    let dir = fixture("clean");
    let nested = "if x { if x { if x { if x {} } } }";
    let source = format!(
        "pub fn cc_at(x: bool) {{\n\n {} }}\npub fn cc_over(x: bool) {{\n\n {} }}\npub fn cog_at(x: bool) {{\n\n {nested} {} }}\npub fn cog_over(x: bool) {{\n\n {nested} {} }}\n",
        "if x {} ".repeat(11),
        "if x {} ".repeat(12),
        "if x {} ".repeat(2),
        "if x {} ".repeat(3)
    );
    std::fs::write(dir.path().join("src/lib.rs"), source).unwrap();
    let output = rstyle(dir.path(), &["--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["summary"]["functions"], 4);
    assert_eq!(report["summary"]["complexity_measured"], 4);
    assert_eq!(report["functions"].as_array().unwrap().len(), 2);
    assert_eq!(report["diagnostics"].as_array().unwrap().len(), 2);
    assert_eq!(report["diagnostics"][0]["rule_id"], "cyclomatic_complexity");
    assert_eq!(report["diagnostics"][1]["rule_id"], "cognitive_complexity");
    for finding in report["diagnostics"].as_array().unwrap() {
        assert_eq!(finding["actual"], 13);
        assert_eq!(finding["limit"], 12);
    }
    let output = rstyle(dir.path(), &[]);
    let text = stdout(&output);
    assert!(text.contains("cc_over"));
    assert!(text.contains("cog_over"));
    assert!(!text.contains("cc_at"));
    assert!(!text.contains("cog_at"));
}
