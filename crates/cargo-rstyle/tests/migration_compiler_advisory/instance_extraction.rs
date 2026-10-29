use super::propagation::checked;
use super::*;

const INSTANCE: &str = "rstyle.rust.call-instances";
const OPTIONS: &[&str] = &["--compiler-patterns", "--compiler-plans"];
const SOURCE: &str = r#"
pub fn identity<T>(value:T)->T { value }
pub fn generic_call()->u32 { identity(7u32) }
pub struct Reader;
pub trait Action { fn act(&self); }
impl Action for Reader {
    fn act(&self) { std::fs::write("/unused",b"reader").ok(); }
}
pub fn trait_call() { Reader.act(); }
pub fn callback(value:Option<u32>)->Option<u32> {
    value.and_then(|n| Some(identity(n)))
}
pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128) {
    std::fs::write("/unused",b"leaf").ok();
}
pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128) { leaf(a,b,c,d,e); }
pub fn dynamic(value:&dyn Action) { value.act(); }
macro_rules! make {()=>{pub fn generated(){}}} make!();
pub fn missing() { generated(); }
"#;

fn inventory<'a>(report: &'a Value, name: &str) -> &'a Value {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["facts"]["name"].as_str().unwrap().ends_with(name))
        .unwrap();
    &function["facts"]["build"]["frontend_options"][INSTANCE]
}

fn strict_observations(report: &Value) {
    for name in ["generic_call", "trait_call"] {
        assert!(
            !inventory(report, name)["calls"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{name} must exercise selected instance observations"
        );
    }
    let callbacks = inventory(report, "callback")["callbacks"]
        .as_array()
        .unwrap();
    assert_eq!(callbacks.len(), 1);
    assert!(!callbacks[0]["calls"].as_array().unwrap().is_empty());
    assert_eq!(callbacks[0]["capture_count"], 0);
}

fn independent_advice(report: &Value) {
    let advisory = &report["compiler_advisory"];
    let unit = &advisory["units"][0];
    assert_eq!(unit["evaluation"]["summary"]["effect_counts"]["io"], 5);
    assert!(
        unit["unresolved_callees"]
            .as_array()
            .unwrap()
            .iter()
            .all(|name| !name.as_str().unwrap().ends_with("generated"))
    );
    assert_eq!(advisory["complete"], false);
    assert_eq!(advisory["planning"]["input_complete"], false);
    assert!(
        !advisory["planning"]["groups"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(unit.get("patterns").is_some());
}

fn source_free_replay(directory: &Path, original: &Value) {
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-advisory-only",
            "--compiler-patterns",
            "--compiler-plans",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.join("no-tools"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 2);
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(replayed["compiler_advisory"], original["compiler_advisory"]);
    assert_eq!(replayed["matching_execution"], "not_requested");
}

#[test]
fn advisory_skips_generic_trait_and_callback_instances_without_changing_advice() {
    let (directory, output, combined) = capture(SOURCE, OPTIONS);
    checked(&output, 2);
    strict_observations(&combined);
    let mut options = OPTIONS.to_vec();
    options.push("--compiler-advisory-only");
    let (output, raw) = super::propagation::capture(directory.path(), &options);
    checked(&output, 2);
    assert_eq!(raw["compiler_advisory"], combined["compiler_advisory"]);
    independent_advice(&raw);
    for function in raw["functions"].as_array().unwrap() {
        assert!(
            function["facts"]["build"]["frontend_options"]
                .get(INSTANCE)
                .is_none()
        );
    }
    source_free_replay(directory.path(), &raw);
}
