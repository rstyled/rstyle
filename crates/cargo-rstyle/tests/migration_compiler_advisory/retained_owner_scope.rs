use super::*;

const OWNERS: usize = 4098;
const OPTIONS: &[&str] = &[
    "--compiler-advisory-only",
    "--compiler-plans",
    "--review-file",
    "reviews.json",
];

fn large_source() -> String {
    let mut source =
        String::from("pub fn aa_reviewed(a:u32,b:u32,c:u32,d:u32,e:u32)->u32 {a+b+c+d+e}\n");
    for index in 0..OWNERS - 2 {
        source.push_str(&format!("pub fn filler_{index:04}()->u32 {{{index}}}\n"));
    }
    source.push_str("pub fn zz_omitted(a:u32,b:u32,c:u32,d:u32,e:u32)->u32 {a+b+c+d+e}\n");
    source
}

fn replay(directory: &Path, original: &Value, extra: &[&str]) -> (Output, Value) {
    let mut options = OPTIONS.to_vec();
    options.extend_from_slice(extra);
    super::reviews::replay(directory, original, &options)
}

fn states(report: &Value) -> Vec<&str> {
    report["review_status"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["state"].as_str().unwrap())
        .collect()
}

fn dismiss(directory: &Path, original: &Value, index: &str) -> Value {
    let (output, report) = replay(
        directory,
        original,
        &[
            "--dismiss-compiler-diagnostic",
            "0",
            index,
            "--review-reason",
            "Independently reviewed large fixture",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    report
}

fn changed_policy(directory: &Path, original: &Value) {
    std::fs::write(directory.join("policy.toml"), "[fn_params]\nmax_params=5\n").unwrap();
    let (output, changed) = replay(
        directory,
        original,
        &[
            "--compiler-advisory-config",
            "policy.toml",
            "--review-prune",
            "--review-prune-dry-run",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(states(&changed), ["stale", "stale"]);
    assert_eq!(changed["review_prune"]["eligible_entries"], 0);
    assert_eq!(changed["review_prune"]["retained_entries"], 2);
    assert_eq!(
        changed["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"],
        serde_json::json!([])
    );
    assert_eq!(
        changed["strict_assessment_execution"],
        original["strict_assessment_execution"]
    );
}

#[test]
fn complete_name_census_resolves_both_retained_owners_without_unrelated_payloads() {
    let (directory, output, original) = capture(&large_source(), OPTIONS);
    assert!(output.status.success(), "{}", stdout(&output));
    let evaluated = &original["compiler_advisory"]["units"][0]["evaluation"];
    assert_eq!(evaluated["summary"]["functions"], OWNERS);
    assert_eq!(evaluated["diagnostics"].as_array().unwrap().len(), 2);
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
    let first = dismiss(directory.path(), &original, "0");
    assert_eq!(
        first["compiler_advisory"]["units"][0]["evaluation"],
        *evaluated
    );
    assert_eq!(
        first["compiler_advisory"]["planning"],
        original["compiler_advisory"]["planning"]
    );
    let tail = dismiss(directory.path(), &original, "1");
    assert_eq!(
        tail["compiler_advisory"]["diagnostic_review"]["hidden_findings"],
        2
    );

    let (output, status) = replay(directory.path(), &original, &["--review-status"]);
    assert!(output.status.success(), "{}", stdout(&output));
    assert_eq!(states(&status), ["hidden_exact", "hidden_exact"]);
    changed_policy(directory.path(), &original);
}
