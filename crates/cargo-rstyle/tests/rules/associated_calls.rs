use super::*;

#[test]
fn associated_calls_with_byte_slices_keep_source_rules_and_effect_gaps() {
    let directory = source_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn external(a: &[u8]) -> std::io::Cursor<&[u8]> { std::io::Cursor::new(a) }\n\
         struct Holder;\n\
         impl Holder { fn take(a: &[u8]) -> Vec<u8> { a.to_vec() } }\n\
         pub fn local(a: &[u8]) -> Vec<u8> { Holder::take(a) }\n",
    )
    .unwrap();
    let (output, report) = source_report(directory.path(), &["--effect-density"]);
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert_eq!(report["capture_complete"], true);
    assert_eq!(report["assessment_complete"], true);
    assert_eq!(report["analysis"]["scope"]["coverage"], "complete");
    assert_eq!(report["mutation_effects"]["coverage"], "incomplete");
    for name in ["external", "local"] {
        assert_eq!(assessed(&report, name)["assessments"][1]["actual"], 1);
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded");
    }
}
