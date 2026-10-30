use super::*;

#[test]
fn native_guard_wrappers_preserve_source_coverage_without_effect_certification() {
    let directory = source_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn guarded(x: u32, b: bool) -> u32 { match x { 1 if b => 7, _ => 9 } }\n\
         macro_rules! truth { ($value:expr) => { $value } }\n\
         pub fn expanded(x: u32, b: bool) -> u32 { match x { 1 if truth!(b) => 7, _ => 9 } }\n",
    )
    .unwrap();
    let (output, report) = source_report(directory.path(), &["--effect-density"]);
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(assessed(&report, "guarded")["assessments"][1]["actual"], 2);
    assert_eq!(assessed(&report, "expanded")["assessments"][1]["actual"], 2);
    assert_eq!(report["mutation_effects"]["coverage"], "incomplete");
}
