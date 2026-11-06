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
    let options = OPTIONS.to_vec();
    super::propagation::capture(directory, &options)
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut options = OPTIONS.to_vec();
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
}
