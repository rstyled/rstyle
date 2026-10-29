use super::*;

const SOURCE: &str = r#"
pub fn automatic(values: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut a = Vec::new(); let mut b = Vec::new();
    for value in values { a.push(*value); b.push(*value); }
    (a, b)
}
pub fn explicit(values: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut a = Vec::new(); let mut b = Vec::new();
    let mut iter = values.iter();
    while let Some(value) = iter.next() { a.push(*value); b.push(*value); }
    (a, b)
}
pub fn pattern(values: &[u32]) -> Vec<u32> {
    let mut a = Vec::new();
    for mut iter in values.iter().copied() { iter += 1; a.push(iter); }
    a
}
pub fn borrowed(values: &[u32]) -> Vec<u32> {
    let mut a = Vec::new(); let mut iter = values.iter();
    for value in &mut iter { a.push(*value); }
    a
}
pub fn nested(values: &[u32]) -> Vec<u32> {
    let mut a = Vec::new();
    for left in values { for right in values { a.push(left + right); } }
    a
}
pub fn callback(values: &[u32]) -> Vec<u32> {
    let mut a = Vec::new();
    values.iter().for_each(|value| a.push(*value));
    a
}
macro_rules! increment { ($value:expr) => {{
    let mut iter = $value; iter += 1; iter
}}; }
pub fn expanded(value: u32) -> u32 { increment!(value) }
pub fn empty(values: &[u32]) { for _value in values {} }
pub struct Counter(u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 { None } else { self.0 -= 1; Some(self.0) }
    }
}
pub fn custom() -> Vec<u32> {
    let mut a = Vec::new();
    for value in Counter(3) { a.push(value); }
    a
}
pub fn borrowed_custom(counter: &mut Counter) -> Vec<u32> {
    let mut a = Vec::new();
    for value in counter { a.push(value); }
    a
}
pub fn reborrowed_custom(counter: &mut Counter) -> Vec<u32> {
    let mut a = Vec::new();
    for value in &mut *counter { a.push(value); }
    a
}
"#;

fn expected() -> [(&'static str, &'static [&'static str]); 11] {
    [
        ("automatic", &["a", "b"]),
        ("explicit", &["a", "b", "iter"]),
        ("pattern", &["a", "iter"]),
        ("borrowed", &["a", "iter"]),
        ("nested", &["a"]),
        ("callback", &["a"]),
        ("expanded", &["iter"]),
        ("next", &["self"]),
        ("custom", &["a"]),
        ("borrowed_custom", &["a"]),
        ("reborrowed_custom", &["a", "counter"]),
    ]
}

fn check_roots(advisory: &Value) {
    assert_eq!(advisory["metrics"], "rstyle.compiler-style-metrics/6");
    let evaluation = &advisory["units"][0]["evaluation"];
    let diagnostics = evaluation["diagnostics"].as_array().unwrap();
    for (name, roots) in expected() {
        let function = evaluation["fix_groups"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|group| group["functions"].as_array().unwrap())
            .find(|function| function["name"].as_str().unwrap().rsplit("::").next() == Some(name))
            .unwrap_or_else(|| panic!("Missing source mutation finding for {name}: {evaluation}"));
        let diagnostic = diagnostics
            .iter()
            .find(|row| row["rule_id"] == "mutated_targets" && row["line"] == function["line"])
            .unwrap();
        assert_eq!(diagnostic["actual"], roots.len(), "{name}");
        let mut observed: Vec<_> = diagnostic["details"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["label"].as_str().unwrap())
            .collect();
        observed.sort_unstable();
        assert_eq!(observed, roots, "{name}");
    }
    assert_eq!(
        diagnostics
            .iter()
            .filter(|row| row["rule_id"] == "mutated_targets")
            .count(),
        11
    );
}

fn check_custom_iterators(report: &Value) {
    for name in ["custom", "borrowed_custom", "reborrowed_custom"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let observed =
            &function["facts"]["build"]["frontend_options"][rstyle_core::advisory::KEY]["facts"];
        assert!(
            observed["mutated_types"]
                .as_array()
                .unwrap()
                .iter()
                .any(|name| name.as_str().unwrap().ends_with("::Counter"))
        );
        let generated: Vec<_> = observed["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["origin"] == "for_desugaring")
            .collect();
        assert_eq!(generated.len(), 1, "{name}");
        assert_eq!(generated[0]["delegated"], false, "{name}");
        assert_eq!(generated[0]["escapes"], false, "{name}");
        if name == "reborrowed_custom" {
            assert!(
                observed["mutations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|event| event["root"] == "counter"
                        && event["origin"] == "source"
                        && event["delegated"] == true)
            );
        }
    }
}

#[test]
fn lowered_for_iterators_are_not_source_targets_but_explicit_bindings_are() {
    let policy = TempDir::new().unwrap();
    let path = policy.path().join("policy.toml");
    std::fs::write(&path, "[mutation_budget]\nmax_mutated_targets=0\n").unwrap();
    let (directory, output, report) = capture(
        SOURCE,
        &[
            "--compiler-advisory-config",
            path.to_str().unwrap(),
            "--compiler-plans",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    check_roots(&report["compiler_advisory"]);
    check_custom_iterators(&report);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    let (output, replay) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
            "--compiler-advisory-config",
            path.to_str().unwrap(),
            "--replay-run",
            report["run_directory"].as_str().unwrap(),
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(replay["compiler_advisory"], report["compiler_advisory"]);
    check_roots(&replay["compiler_advisory"]);
}
