use super::propagation::checked;
use super::*;

const POLICY: &str = "[side_effects]\nmax_density_milli=1\nmin_statements=0\nmin_lines=0\n";
const CHAIN: &str = r#"
macro_rules! make {()=>{
    pub fn generated_leaf() {let _ = std::fs::read("/unused");}
    pub fn generated_middle() {generated_leaf();}
    pub fn generated_left() {generated_right();}
    pub fn generated_right() {generated_left(); generated_leaf();}
    pub fn generated_dynamic(callback:fn()) {callback();}
    pub fn generated_unsafe(value:*const u32)->u32 {unsafe {*value}}
}}
make!();
pub fn authored_chain(a:u32,b:u32,c:u32,d:u32,e:u32) {generated_middle();}
pub fn authored_cycle() {generated_left();}
pub fn authored_dynamic(callback:fn()) {generated_dynamic(callback);}
pub fn authored_unsafe(value:*const u32)->u32 {generated_unsafe(value)}
"#;

fn unit(report: &Value) -> &Value {
    &report["compiler_advisory"]["units"][0]
}

fn effect_count(unit: &Value, kind: &str) -> u64 {
    unit["evaluation"]["summary"]["effect_counts"][kind]
        .as_u64()
        .unwrap_or(0)
}

fn partial(report: &Value, authored: usize, generated: usize) {
    assert_eq!(
        report["compiler_advisory"]["metrics"],
        "rstyle.compiler-style-metrics/7"
    );
    assert_eq!(report["compiler_advisory"]["complete"], false);
    let unit = unit(report);
    assert_eq!(unit["observations"].as_array().unwrap().len(), authored);
    assert_eq!(unit["evaluation"]["summary"]["functions"], authored);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), generated);
    let rows = unit["generated_effects"].as_array().unwrap();
    assert_eq!(rows.len(), generated);
    for row in rows {
        assert_eq!(row["metrics"], "rstyle.compiler-generated-effects/2");
        assert!(row["connections"]["id"].is_string());
        assert!(row["connections"].get("implements").is_some());
        assert!(row["connections"]["edges"].is_array());
        for field in ["source", "statements", "complexity", "evaluation"] {
            assert!(row.get(field).is_none(), "{field}: {row}");
        }
    }
}

fn fixture(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("policy.toml"), POLICY).unwrap();
    directory
}

fn current(directory: &Path, extra: &[&str]) -> (Output, Value) {
    let mut args = vec![
        "--compiler-advisory-only",
        "--compiler-advisory-config",
        "policy.toml",
        "--compiler-plans",
    ];
    args.extend_from_slice(extra);
    propagation::capture(directory, &args)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command.current_dir(directory).args([
        "clones",
        "--format",
        "json",
        "--extraction-profile",
        "hir-data-v3",
        "--compiler-advisory",
        "--compiler-advisory-only",
        "--compiler-plans",
        "--compiler-advisory-config",
        "policy.toml",
        "--replay-run",
        original["run_directory"].as_str().unwrap(),
    ]);
    command
        .args(extra)
        .env("PATH", directory.join("no-tools"))
        .env("CARGO", directory.join("no-cargo"))
        .env("RUSTC", directory.join("no-rustc"))
        .env("RSTYLE_DRIVER", directory.join("no-driver"));
    let output = diagnostic_process::output(&mut command).unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap();
    (output, report)
}

fn authored_plans(report: &Value, expected: usize) {
    let planning = &report["compiler_advisory"]["planning"];
    assert_eq!(planning["input_complete"], false);
    assert_eq!(planning["known_functions"], expected);
    let authored = unit(report)["observations"].as_array().unwrap();
    for group in planning["groups"].as_array().unwrap() {
        for field in ["members", "step_owners"] {
            for key in group[field].as_array().unwrap() {
                assert!(authored.iter().any(|row| row["key"] == *key));
            }
        }
    }
}

#[test]
fn generated_chains_and_cycles_use_existing_depth_and_local_effect_rules() {
    let directory = fixture(CHAIN);
    let (output, original) = current(directory.path(), &[]);
    checked(&output, 2);
    partial(&original, 4, 6);
    authored_plans(&original, 4);
    assert_eq!(effect_count(unit(&original), "io"), 2);
    assert_eq!(effect_count(unit(&original), "unsafe"), 0);
    assert_eq!(effect_count(unit(&original), "unknown"), 0);
    assert_eq!(unit(&original)["unresolved_callees"], serde_json::json!([]));
    for (depth, expected) in [(0, 0), (1, 0), (2, 1), (3, 2)] {
        std::fs::write(
            directory.path().join("policy.toml"),
            format!("{POLICY}transitive_depth={depth}\n"),
        )
        .unwrap();
        let (output, report) = replay(directory.path(), &original, &[]);
        checked(&output, 2);
        partial(&report, 4, 6);
        assert_eq!(effect_count(unit(&report), "io"), expected, "depth {depth}");
        assert_eq!(effect_count(unit(&report), "unsafe"), 0);
        assert_eq!(effect_count(unit(&report), "unknown"), 0);
    }
}

#[test]
fn generated_delegated_writes_and_authored_boundaries_preserve_existing_semantics() {
    let source = r#"
pub struct Counter(u32);
macro_rules! make {()=>{
    impl Counter {pub fn generated_write(&mut self) {self.0+=1;}}
    pub fn generated_io() {let _ = std::fs::read("/unused");}
}}
// rstyle-allow(side_effect_density): invocation is not a generated owner boundary
make!();
impl Counter {pub fn wrapper(&mut self) {self.generated_write();}}
pub fn io_caller() {generated_io();}
// rstyle-allow(side_effect_density): authored boundary absorbs inherited IO
pub fn boundary() {generated_io();}
pub fn after_boundary() {boundary();}
"#;
    let directory = fixture(source);
    let (output, report) = current(directory.path(), &[]);
    checked(&output, 2);
    partial(&report, 4, 2);
    assert_eq!(effect_count(unit(&report), "mutation"), 1);
    assert_eq!(effect_count(unit(&report), "io"), 2);
    let diagnostics = unit(&report)["evaluation"]["diagnostics"]
        .as_array()
        .unwrap();
    for name in ["Counter::wrapper", "io_caller"] {
        assert!(
            diagnostics.iter().any(|row| {
                row["rule_id"] == "side_effect_density"
                    && row["message"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("`{name}`"))
            }),
            "{name}: {diagnostics:?}"
        );
    }
    for name in ["boundary", "after_boundary"] {
        assert!(!diagnostics.iter().any(|row| {
            row["rule_id"] == "side_effect_density"
                && row["message"]
                    .as_str()
                    .unwrap()
                    .contains(&format!("`{name}`"))
        }));
    }
    authored_plans(&report, 4);
}

#[test]
fn generated_connections_replay_without_sources_tools_or_recollecting_calls() {
    let directory = fixture(CHAIN);
    let (output, original) = current(directory.path(), &[]);
    checked(&output, 2);
    std::fs::remove_dir_all(directory.path().join("src")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    for detail in ["full", "compact"] {
        let (output, report) = replay(directory.path(), &original, &["--report-detail", detail]);
        checked(&output, 2);
        assert_eq!(report["compiler_advisory"], original["compiler_advisory"]);
    }
    std::fs::write(
        directory.path().join("policy.toml"),
        format!("{POLICY}\n[side_effects.paths]\nnetwork=['std::fs::read']\n"),
    )
    .unwrap();
    let (output, changed) = replay(directory.path(), &original, &[]);
    checked(&output, 2);
    partial(&changed, 4, 6);
    assert_eq!(effect_count(unit(&changed), "io"), 0);
    assert_eq!(effect_count(unit(&changed), "network"), 2);
    for (before, after) in unit(&original)["generated_effects"]
        .as_array()
        .unwrap()
        .iter()
        .zip(unit(&changed)["generated_effects"].as_array().unwrap())
    {
        for field in [
            "key",
            "connections",
            "mutations",
            "calls",
            "intrinsic_effects",
        ] {
            assert_eq!(before[field], after[field], "{field}");
        }
    }
}

#[test]
fn generated_dependency_targets_need_exact_selected_unit_contexts() {
    let directory = propagation::workspace(
        "macro_rules! make {()=>{pub fn leaf(){let _=std::fs::read(\"/unused\");}}} make!();",
    );
    std::fs::write(directory.path().join("policy.toml"), POLICY).unwrap();
    let (output, omitted) = current(directory.path(), &["-p", "caller"]);
    checked(&output, 2);
    let caller = propagation::unit(&omitted, "caller");
    assert_eq!(
        caller["unresolved_callees"],
        serde_json::json!(["leaf::leaf"])
    );
    assert_eq!(effect_count(caller, "io"), 0);
    let (output, selected) = current(directory.path(), &[]);
    checked(&output, 2);
    let caller = propagation::unit(&selected, "caller");
    let leaf = propagation::unit(&selected, "leaf");
    assert_eq!(effect_count(caller, "io"), 1);
    assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    assert_eq!(
        caller["resolved_dependency_callees"]["leaf::leaf"],
        leaf["unit"]
    );
    assert_eq!(leaf["observations"], serde_json::json!([]));
    assert_eq!(leaf["generated_effects"].as_array().unwrap().len(), 1);
    assert_eq!(leaf["unavailable"].as_array().unwrap().len(), 1);
    assert_eq!(
        selected["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}
