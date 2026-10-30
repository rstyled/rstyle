use super::*;

#[test]
fn unsupported_call_results_and_siblings_preserve_available_source_rules() {
    let directory = source_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "fn keep(a: &[u8]) -> Vec<u8> { a.to_vec() }\n\
         pub fn vector_result(a: &[u8]) -> Vec<u8> { keep(a) }\n\
         fn take(a: &[u8], b: Vec<u8>) -> u32 { a.len() as u32 + b.len() as u32 }\n\
         pub fn mixed_arguments(a: &[u8], b: Vec<u8>) -> u32 { take(a, b) }\n",
    )
    .unwrap();

    let (output, report) = source_report(directory.path(), &["--effect-density"]);
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["mutation_effects"]["coverage"], "incomplete");
    assert_eq!(
        assessed(&report, "vector_result")["assessments"][1]["actual"],
        1
    );
    assert_eq!(
        assessed(&report, "mixed_arguments")["assessments"][1]["actual"],
        2
    );
    for name in ["vector_result", "mixed_arguments"] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded");
    }
}
