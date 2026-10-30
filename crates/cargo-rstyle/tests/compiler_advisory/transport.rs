use super::*;
use serde_json::json;

#[test]
fn deep_derived_bodies_preserve_original_advisory_coverage_and_source_free_replay() {
    let fields = (0..96)
        .map(|index| format!("field{index}:u32"))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "#[derive(Debug,PartialEq)] pub struct Wide{{{fields}}}\npub fn original()->u32{{1}}"
    );
    let (directory, output, report) = capture(&source, &["--report-detail", "compact"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["handoff_failures"], json!([]));
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["complete"], false);
    let unit = &advisory["units"][0];
    assert_eq!(unit["observations"].as_array().unwrap().len(), 1);
    assert!(!unit["unavailable"].as_array().unwrap().is_empty());
    let path = report["functions"][0]["unit_evidence"]["bundle"]
        .as_str()
        .unwrap();
    let wire: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(
        wire["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| function["body"].is_array())
    );
    std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
    let (_, replay) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--replay-run",
            report["run_directory"].as_str().unwrap(),
            "--compiler-advisory",
            "--report-detail",
            "compact",
        ],
    );
    assert_eq!(replay["handoff_failures"], json!([]));
    assert_eq!(&replay["compiler_advisory"], advisory);
    assert_eq!(replay["matching"], report["matching"]);
}
