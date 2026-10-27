use super::*;

fn descendants<'a>(node: &'a rstyle_pdg::HirNode, output: &mut Vec<&'a rstyle_pdg::HirNode>) {
    output.push(node);
    for child in &node.children {
        descendants(child, output);
    }
}

#[test]
fn try_capture_retains_operand_and_both_continuations_without_admission() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        fn checked(value: u8) -> Result<u8, u8> { if value == 0 { Err(7) } else { Ok(value) } }
        pub fn reader(value: u8) -> Result<u8, u8> { let byte = checked(value)?; Ok(byte) }
    "#,
    )
    .unwrap();
    for profile in ["hir-data-v3", "hir-data-v2"] {
        let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(output.status.success(), "{report}");
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["facts"]["name"] == "reader")
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{function}");
        let facts: rstyle_pdg::HirFnFacts =
            serde_json::from_value(function["facts"].clone()).unwrap();
        let mut nodes = Vec::new();
        descendants(&facts.body, &mut nodes);
        if profile == "hir-data-v3" {
            let operation = nodes.iter().find(|n| n.kind == "expr_try_desugar").unwrap();
            assert_eq!(operation.children.len(), 3);
            assert_eq!(operation.children[0].kind, "expr_call");
            assert!(
                operation.children[1..]
                    .iter()
                    .all(|n| n.kind == "match_arm")
            );
            assert!(
                nodes
                    .iter()
                    .any(|n| n.kind == "expr_call" && n.snippet == "checked(value)")
            );
            assert!(nodes.iter().any(|n| n.kind == "expr_ret"));
        } else {
            let operation = nodes.iter().find(|n| n.kind == "expr_match").unwrap();
            assert!(operation.children.is_empty());
            assert!(!nodes.iter().any(|n| n.snippet == "checked(value)"));
        }
    }
}

#[test]
fn nested_option_try_capture_keeps_each_operand_without_certifying_synthetic_arms() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn nested(value: Option<Option<u8>>) -> Option<u8> { Some(value??) }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "excluded", "{function}");
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    let mut nodes = Vec::new();
    descendants(&facts.body, &mut nodes);
    let operations: Vec<_> = nodes
        .iter()
        .filter(|n| n.kind == "expr_try_desugar")
        .collect();
    assert_eq!(operations.len(), 2);
    assert!(operations.iter().all(|n| n.children.len() == 3));
    assert_eq!(nodes.iter().filter(|n| n.kind == "expr_ret").count(), 2);
    assert_eq!(
        nodes
            .iter()
            .filter(|n| n.kind == "expr_path" && n.snippet == "value")
            .count(),
        1
    );
    assert!(rstyle_pdg::producer::produce(&facts).is_err());
}
