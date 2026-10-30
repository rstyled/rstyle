use super::propagation::checked;
use super::*;

const OPTIONS: &[&str] = &["--compiler-patterns", "--compiler-plans"];
const SHARED: &str =
    "pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128){std::fs::write(\"/unused\",b\"x\").ok();}";
const SOURCE: &str = r#"
#[path="../shared.rs"] mod shared;
pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){shared::leaf(a,b,c,d,e);}
pub fn cfg_control(flag:bool) {
    #[cfg(any())] { let mut hidden=0; hidden+=1; std::fs::write("/unused",b"hidden").ok(); }
    #[cfg(all())] { if flag {} }
}
#[cfg(any())] pub fn inactive(a:u32,b:u32,c:u32,d:u32,e:u32) {}
pub fn identity<T>(value:T)->T { value }
pub struct Error;
pub fn slice_check<T>(values:&[T], minimum:usize, what:&'static str)->Result<(),Error> {
    if values.len()<minimum { return Err(Error); } Ok(())
}
pub fn concrete(values:&[u8])->usize {
    let saved=identity(1u32); slice_check(values,1,"buffer").ok(); values.len()+saved as usize
}
"#;

fn fixture() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("shared.rs"), SHARED).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    directory
}

fn raw_capture(directory: &Path) -> (Output, Value) {
    let mut options = OPTIONS.to_vec();
    options.push("--compiler-advisory-only");
    super::propagation::capture(directory, &options)
}

fn named<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"].as_str().unwrap().ends_with(name))
        .unwrap()
}

fn options<'a>(report: &'a Value, name: &str) -> &'a Value {
    &named(report, name)["facts"]["build"]["frontend_options"]
}

fn strict_inputs_remain_separate(combined: &Value, raw: &Value) {
    for (name, key) in [
        ("cfg_control", "source_correspondence"),
        ("identity", "rstyle.rust.scalar-instance-projections"),
        (
            "slice_check",
            "rstyle.rust.shared-slice-instance-projections",
        ),
    ] {
        assert!(options(combined, name).get(key).is_some(), "{name}: {key}");
        assert!(options(raw, name).get(key).is_none(), "{name}: {key}");
    }
    assert_eq!(raw["matching_execution"], "not_requested");
    assert!(raw.get("matching").is_none());
    assert!(combined.get("matching").is_some());
}

fn original_semantics(report: &Value) {
    let unit = &report["compiler_advisory"]["units"][0];
    let observations = unit["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 6);
    assert!(!observations.iter().any(|row| row["name"] == "inactive"));
    let cfg = observations
        .iter()
        .find(|row| row["name"] == "cfg_control")
        .unwrap();
    assert_eq!(cfg["direct_mutations"], 0);
    assert_eq!(unit["evaluation"]["summary"]["effect_counts"]["io"], 2);
    assert!(
        unit["evaluation"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| { row["path"] == "shared.rs" && row["rule_id"] == "fn_params" })
    );
    let groups = report["compiler_advisory"]["planning"]["groups"]
        .as_array()
        .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["step_owners"].as_array().unwrap().len(), 2);
}

#[test]
fn advisory_extraction_preserves_cfg_shared_calls_and_generic_slice_semantics() {
    let directory = fixture();
    let (output, combined) = super::propagation::capture(directory.path(), OPTIONS);
    checked(&output, 0);
    let (output, raw) = raw_capture(directory.path());
    checked(&output, 0);
    assert_eq!(raw["compiler_advisory"], combined["compiler_advisory"]);
    original_semantics(&raw);
    strict_inputs_remain_separate(&combined, &raw);
}

#[test]
fn generated_and_disabled_owners_keep_the_same_missing_data_in_both_capture_modes() {
    let directory = fixture();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\nmacro_rules! make {{()=>{{pub fn generated(){{}}}}}} make!();"),
    )
    .unwrap();
    let (output, combined) = super::propagation::capture(directory.path(), OPTIONS);
    checked(&output, 2);
    let (output, raw) = raw_capture(directory.path());
    checked(&output, 2);
    assert_eq!(raw["compiler_advisory"], combined["compiler_advisory"]);
    original_semantics(&raw);
    let unit = &raw["compiler_advisory"]["units"][0];
    assert_eq!(unit["expected_functions"], 7);
    assert_eq!(unit["unavailable"].as_array().unwrap().len(), 1);
    assert_eq!(raw["compiler_advisory"]["complete"], false);
    for report in [&combined, &raw] {
        assert!(
            named(report, "generated")["facts"]["source_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["construct"] == "macro_generated_function")
        );
    }
    assert_eq!(
        raw["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut options = OPTIONS.to_vec();
    options.push("--compiler-advisory-only");
    options.extend_from_slice(extra);
    super::reviews::replay(directory, original, &options)
}

fn baseline_capture() -> (TempDir, Value) {
    let directory = fixture();
    std::fs::rename(
        directory.path().join("shared.rs"),
        directory.path().join("src/shared.rs"),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "mod shared; pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128){shared::leaf(a,b,c,d,e);}",
    )
    .unwrap();
    let (output, original) = raw_capture(directory.path());
    checked(&output, 0);
    let (output, exported) = replay(
        directory.path(),
        &original,
        &["--write-compiler-baseline", "baseline.json"],
    );
    checked(&output, 0);
    assert_eq!(
        exported["compiler_advisory"]["units"],
        original["compiler_advisory"]["units"]
    );
    let baseline: Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("baseline.json")).unwrap())
            .unwrap();
    assert!(
        baseline["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["owner"]["locator"]["source"] == "src/shared.rs" })
    );
    (directory, original)
}

#[test]
fn reduced_extraction_replays_source_free_baselines_reviews_and_shared_plans() {
    let (directory, original) = baseline_capture();
    for source in ["src/lib.rs", "src/shared.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(source)).unwrap();
    }
    let (output, report) = replay(
        directory.path(),
        &original,
        &[
            "--compiler-baseline",
            "baseline.json",
            "--fail-on-new-compiler",
            "--review-file",
            "reviews.json",
            "--dismiss-compiler-diagnostic",
            "0",
            "0",
            "--review-reason",
            "Reviewed original fixture parameter boundary",
        ],
    );
    checked(&output, 0);
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["diagnostic_baseline"]["new_findings"], 0);
    assert_eq!(advisory["diagnostic_baseline"]["coverage"], "complete");
    assert_eq!(
        advisory["diagnostic_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    for key in [
        "evaluation",
        "observations",
        "unavailable",
        "unresolved_callees",
    ] {
        assert_eq!(
            advisory["units"][0][key],
            original["compiler_advisory"]["units"][0][key]
        );
    }
    assert_eq!(
        advisory["units"][0]["visible_diagnostics"],
        serde_json::json!([1])
    );
    assert_eq!(
        advisory["planning"],
        original["compiler_advisory"]["planning"]
    );
    assert_eq!(report["matching_execution"], "not_requested");
}
