use super::*;

fn workspace(source: &str) -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    directory
}

fn pair<'a>(report: &'a Value, left: &str, right: &str) -> &'a Value {
    let key = |suffix: &str| {
        report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| {
                function["facts"]["name"]
                    .as_str()
                    .unwrap()
                    .ends_with(suffix)
            })
            .unwrap()["key"]
            .as_str()
            .unwrap()
    };
    let (left, right) = (key(left), key(right));
    report["pair_explanations"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (pair["first"] == left && pair["second"] == right)
                || (pair["first"] == right && pair["second"] == left)
        })
        .unwrap()
}

#[test]
fn native_differences_preserve_literals_and_ordered_formals_in_all_reports() {
    let directory = workspace(
        "pub fn add_one(value:u32)->u32 { value+1 }\npub fn add_two(input:u32)->u32 { input+2 }\npub fn original(left:u32,right:u32)->u32 { left-right }\npub fn reordered(left:u32,right:u32)->u32 { right-left }\n",
    );
    let args = ["--extraction-profile", "hir-data-v3", "--explain-pairs"];
    let (output, full) = clones(directory.path(), &args);
    assert!(output.status.success(), "{}", stdout(&output));
    let explanation = &full["pair_explanations"];
    assert_eq!(explanation["execution"], "completed");
    assert_eq!(explanation["coverage"], "complete");
    assert_eq!(explanation["expected_pairs"], explanation["assessed_pairs"]);
    assert_native_holes(&full);
    let (output, compact) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--explain-pairs",
            "--report-detail",
            "compact",
        ],
    );
    assert!(output.status.success());
    assert_eq!(full["pair_explanations"], compact["pair_explanations"]);
    assert_eq!(full["matching"], compact["matching"]);
    let (output, default) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(default["matching"], full["matching"]);
    assert_eq!(default["pair_explanations"]["execution"], "not_requested");
    let output = rstyle(
        directory.path(),
        &[
            "clones",
            "--extraction-profile",
            "hir-data-v3",
            "--explain-pairs",
        ],
    );
    assert!(output.status.success());
    assert!(stdout(&output).contains("Native differences"));
    assert!(stdout(&output).contains("do not establish equivalence"));
}

#[test]
fn unsupported_native_pairs_are_explicitly_incomplete() {
    let directory = workspace(
        "pub fn first(value:u32)->u32 { if value>0 { value } else { 0 } }\npub fn second(input:u32)->u32 { if input>0 { input } else { 0 } }\n",
    );
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--explain-pairs"],
    );
    assert_eq!(output.status.code(), Some(2));
    let explanation = &report["pair_explanations"];
    assert_eq!(explanation["execution"], "completed");
    assert_eq!(explanation["coverage"], "incomplete");
    assert_eq!(explanation["complete_pairs"], 0);
    let native = pair(&report, "first", "second");
    assert_eq!(native["coverage"], "unsupported");
    assert!(native["correspondence"].as_array().unwrap().is_empty());
    assert!(native["reason"].as_str().is_some());
    assert_eq!(report["pair_evaluation_complete"], true);
}

#[test]
fn matching_budget_does_not_become_an_empty_complete_explanation() {
    let directory = clone_workspace();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--explain-pairs",
            "--max-pairs",
            "0",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let explanation = &report["pair_explanations"];
    assert_eq!(explanation["coverage"], "incomplete");
    assert!(explanation["expected_pairs"].is_null());
    assert_eq!(explanation["assessed_pairs"], 0);
    assert!(!explanation["reasons"].as_array().unwrap().is_empty());
}

fn assert_native_holes(full: &Value) {
    let literal = pair(full, "add_one", "add_two");
    assert_eq!(literal["correspondence"].as_array().unwrap().len(), 4);
    let holes = literal["holes"].as_array().unwrap();
    assert_eq!(holes.len(), 1);
    assert_eq!(holes[0]["kind"], "literal");
    assert_eq!(holes[0]["role"], "binary_right");
    assert_ne!(holes[0]["left"]["literal"], holes[0]["right"]["literal"]);
    assert!(holes[0]["left"].get("children").is_none());
    let ordered = pair(full, "original", "reordered");
    assert_eq!(ordered["correspondence"].as_array().unwrap().len(), 4);
    let holes = ordered["holes"].as_array().unwrap();
    assert_eq!(holes.len(), 2);
    assert_eq!(holes[0]["kind"], "formal_role");
    assert_eq!(holes[0]["role"], "binary_left");
    assert_eq!(holes[1]["kind"], "formal_role");
    assert_eq!(holes[1]["role"], "binary_right");
}
