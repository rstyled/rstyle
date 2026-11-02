use super::*;

const SOURCE: &str = r#"
#![feature(guard_patterns)]
pub fn accumulator(n: usize) -> u8 { let mut i = 0; let mut total = 0u8; while i < n { total += 1; i += 1; } total }
pub fn practical_vec(xs: &[u8]) -> Vec<u8> { let mut out = Vec::new(); for x in xs { out.push(*x); } out }
pub fn practical_map(xs: &[u8]) { let mut out = std::collections::HashMap::new(); for x in xs { out.insert(*x, 1); } }
pub fn practical_match(n: usize, flag: bool) { let mut i = 0; while i < n { match flag { true => (), false => () } i += 1; } }
pub fn practical_iflet(n: usize, x: Option<u8>) { let mut i = 0; while i < n { if let Some(_) = x {} i += 1; } }
trait Modify { fn advance(&mut self); fn reset(&mut self); }
impl Modify for u32 { fn advance(&mut self) { *self += 1; } fn reset(&mut self) { *self = 0; } }
pub fn autoref(n: u32) { let mut i = 0u32; while i < n { i.advance(); } }
pub fn autoref_reset(n: u32) { let mut i = 0u32; while i < n { i.reset(); i += 1; } }
pub fn refmut(n: u32) { let mut i = 0u32; while i < n { let ref mut alias = i; *alias = 0; i += 1; } }
pub fn ergonomic_refmut(n: usize) { let mut i = 0; while i < n { match &mut i { alias => *alias = 0 } i += 1; } }
pub fn metadata_rebind<'a>(mut xs: &'a [u8], other: &'a [u8]) { let mut i = 0; while i < xs.len() { i += 1; let alias = &mut xs; *alias = other; } }
pub fn unsafe_body(n: usize) { let mut i = 0; while i < n { unsafe { std::ptr::read_volatile(&i); } i += 1; } }
pub fn captured_closure(n: usize) { let mut i = 0; while i < n { let _f = || i; i += 1; } }
pub fn callee_continue(n: usize, flag: bool, fp: fn()) { let mut i = 0; while i < n { ({ if flag { continue; } fp })(); i += 1; } }
pub fn callee_reset(n: usize, fp: fn()) { let mut i = 0; while i < n { ({ i = 0; fp })(); i += 1; } }
pub fn short_circuit_and(n: usize, flag: bool) { let mut i = 0; while i < n { flag && { i += 1; true }; } }
pub fn short_circuit_or(n: usize, flag: bool) { let mut i = 0; while i < n { flag || { i += 1; true }; } }
pub fn guarded_or(n: usize, flag: bool) { let mut i = 0; while i < n { match Some(0u8) { Some(_) | Some(_) if { opaque(); flag } => (), _ => () } i += 1; } }
pub fn pattern_guard_reset(n: usize) { let mut i = 0; while i < n { match Some(0u8) { Some(_ if { i = 0; true }) => (), _ => () } i += 1; } }
pub fn guard_call(n: usize) { let mut i = 0; while { opaque(); i < n } { i += 1; } }
pub fn loop_local_extent(n: usize, a: &[u8], b: &[u8]) { let mut i = 0; while i < n { let xs = if i == 0 { a } else { b }; for _ in xs {} i += 1; } }
pub struct Container { pub bytes: Vec<u8> }
pub struct View<'a> { pub bytes: &'a [u8] }
fn switch<'a>(root: &mut &'a Container, other: &'a Container) { *root = other; }
pub fn containing_rebind<'a>(mut root: &'a Container, other: &'a Container) { let mut i = 0; while i < root.bytes.len() { i += 1; switch(&mut root, other); } }
pub fn shared_vec(xs: &Vec<u8>) { for _ in xs {} }
pub fn shared_field_vec(root: &Container) { for _ in &root.bytes {} }
pub fn shared_field_slice(root: &View<'_>) { for _ in root.bytes {} }
pub fn stable_slice_binding(xs: &[u8]) { let stable = xs; for _ in stable {} }
pub fn stable_len_guard(xs: &Vec<u8>) { let mut i = 0; while i < xs.len() { i += 1; } }

macro_rules! generated { () => { pub fn generated_owner() {} } }
generated!();
pub fn stale(n:u8){let mut j=0;while j<n{j+=1;}let end=j;let mut i=0;while i<end{i+=1;}}
pub fn sequential(n: usize, m: usize) { for _ in 0..n {} for _ in 0..m {} }
pub fn repeated_slice(xs: &[u8]) { for _ in xs { for _ in xs {} } }
fn touch(i: &mut usize) { *i = 0; }
pub fn alias(n: usize) { let mut i = 0; while i < n { touch(&mut i); i += 1; } }
pub fn skipped(n: usize, flag: bool) { let mut i = 0; while i < n { if flag { i += 1; } } }
pub fn unknown_predicate(flag: bool) { while flag {} }
pub fn linear(n: usize) { let mut i = 0; while i < n { i += 1; } }
pub fn persistent(n: usize, m: usize) { let mut i = 0; let mut j = 0; while i < n { while j < m { j += 1; } i += 1; } }
pub fn reset(n: usize, m: usize) { let mut i = 0; while i < n { let mut j = 0; while j < m { j += 1; } i += 1; } }
pub fn square(n: usize) { let mut i = 0; while i < n { let mut j = 0; while j < n { j += 1; } i += 1; } }
pub fn range(n: usize) { for _i in 0..n {} }
pub fn slice(xs: &[u8]) { for _x in xs {} }
pub fn unknown_continue(n: usize, flag: bool) { let mut i = 0; while i < n { if flag { continue; } i += 1; } }
fn opaque() {}
pub fn unknown_call(n: usize) { let mut i = 0; while i < n { opaque(); i += 1; } }
pub fn limitwrites(mut n: usize) { let mut i = 0; while i < n { n += 1; i += 1; } }
pub fn overflow(n: u8) { let mut i = 0u8; while i <= n { i += 1; } }
"#;

fn runtime(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]["runtime_bounds"]
}

fn named_row<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["name"].as_str().unwrap().rsplit("::").next() == Some(name))
        .unwrap_or_else(|| panic!("missing runtime-bound row {name}"))
}

#[test]
fn runtime_bounds_are_independent_replayable_owner_results() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let output = rstyle(
        directory.path(),
        &["patterns", "--format", "json", "--complexity"],
    );
    let original: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    assert_known(&original);
    assert_call(runtime(&original)["rows"].as_array().unwrap());
    assert_unknown(runtime(&original)["rows"].as_array().unwrap());
    assert_replay(directory.path(), &original);
}

fn assert_known(original: &Value) {
    let report = runtime(original);
    assert_eq!(report["input_complete"], true);
    let rows = report["rows"].as_array().unwrap();
    assert!(
        !rows
            .iter()
            .any(|row| row["name"].as_str().unwrap().rsplit("::").next() == Some("generated_owner"))
    );
    for name in ["linear", "persistent", "reset", "square", "range", "slice"] {
        let row = named_row(rows, name);
        assert!(row["result"].get("Proven").is_some(), "{name}: {row}");
        assert!(row.get("observation").is_none());
    }
    assert_known_bounds(rows);
}

fn assert_known_bounds(rows: &[Value]) {
    for (name, expected) in [
        ("linear", 3),
        ("persistent", 8),
        ("reset", 18),
        ("square", 12),
        ("sequential", 8),
        ("repeated_slice", 12),
        ("accumulator", 3),
        ("practical_vec", 3),
        ("practical_map", 3),
        ("practical_match", 3),
        ("practical_iflet", 3),
        ("shared_vec", 3),
        ("shared_field_vec", 3),
        ("shared_field_slice", 3),
        ("stable_slice_binding", 3),
        ("stable_len_guard", 3),
    ] {
        let row = named_row(rows, name);
        let bound: rstyle_core::runtime_bounds::Bound =
            serde_json::from_value(row["result"]["Proven"]["loop_body_entries"].clone()).unwrap();
        assert_eq!(value(&bound), expected, "{name}: {bound:?}");
    }
}

fn assert_call(rows: &[Value]) {
    let call = named_row(rows, "unknown_call");
    let assessment = &call["result"]["Proven"];
    let bound: rstyle_core::runtime_bounds::Bound =
        serde_json::from_value(assessment["loop_body_entries"].clone()).unwrap();
    assert_eq!(value(&bound), 3);
    let calls = assessment["calls"].as_array().unwrap();
    let opaque = calls
        .iter()
        .find(|call| call["callee"].as_str().unwrap().ends_with("opaque"))
        .unwrap();
    let count: rstyle_core::runtime_bounds::Bound =
        serde_json::from_value(opaque["count"].clone()).unwrap();
    assert_eq!(value(&count), 3);
    assert!(
        assessment["whole_runtime"]
            .as_str()
            .unwrap()
            .starts_with("unknown")
    );
}

fn assert_unknown(rows: &[Value]) {
    for name in [
        "unknown_continue",
        "limitwrites",
        "overflow",
        "stale",
        "alias",
        "skipped",
        "unknown_predicate",
        "autoref",
        "autoref_reset",
        "refmut",
        "ergonomic_refmut",
        "metadata_rebind",
        "unsafe_body",
        "captured_closure",
        "callee_continue",
        "callee_reset",
        "short_circuit_and",
        "short_circuit_or",
        "guarded_or",
        "pattern_guard_reset",
        "guard_call",
        "loop_local_extent",
        "containing_rebind",
    ] {
        let row = named_row(rows, name);
        assert!(row["result"].get("Unknown").is_some(), "{name}: {row}");
    }
}

fn assert_replay(directory: &Path, original: &Value) {
    let report = runtime(original);
    let output = rstyle(
        directory,
        &[
            "patterns",
            "--format",
            "json",
            "--complexity",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(runtime(&replayed), report);
    assert_source_free_replay(directory, original);
}

fn assert_source_free_replay(directory: &Path, original: &Value) {
    let report = runtime(original);
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let output = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap())
        .current_dir(directory)
        .args([
            "patterns",
            "--format",
            "json",
            "--complexity",
            "--top",
            "1",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", "/runtime-replay-no-tools")
        .env("CARGO", "/runtime-replay-no-cargo")
        .env("RUSTC", "/runtime-replay-no-rustc")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    let listed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(runtime(&listed)["selected"].as_array().unwrap().len(), 1);
    assert_eq!(runtime(&listed)["rows"], report["rows"]);
    assert_default_off(directory, original);
}

fn assert_default_off(directory: &Path, original: &Value) {
    let output = rstyle(
        directory,
        &[
            "patterns",
            "--format",
            "json",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    let off: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(
        off["compiler_advisory"]["units"][0]
            .get("runtime_bounds")
            .is_none()
    );
}

#[test]
fn old_capture_cannot_silently_supply_runtime_bounds() {
    let (directory, output, original) = capture("pub fn empty() {}", &["--compiler-advisory-only"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert!(
        original["compiler_advisory"]["units"][0]
            .get("runtime_bounds")
            .is_none()
    );
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-runtime-bounds",
            "--compiler-advisory-only",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("missing compiler runtime-bound capture capability")
    );
}

fn value(bound: &rstyle_core::runtime_bounds::Bound) -> u64 {
    use rstyle_core::runtime_bounds::Bound;
    match bound {
        Bound::Const(value) => *value,
        Bound::Input(name) => {
            if name.ends_with('m') {
                5
            } else {
                3
            }
        }
        Bound::Add(values) => values.iter().map(value).sum(),
        Bound::Mul(values) => values.iter().map(value).product(),
        Bound::Max(values) => values.iter().map(value).max().unwrap_or(0),
    }
}
