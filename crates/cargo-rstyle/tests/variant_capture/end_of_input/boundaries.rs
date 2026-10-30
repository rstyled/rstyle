use super::*;

#[test]
fn end_of_input_keeps_diagnostics_checked_panics_and_drop_boundaries_distinct() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub fn narrow(n: usize) -> u16 { u16::try_from(n).unwrap() }
        pub fn after_condition(mut n: usize) -> usize {
            assert!({ n += 1; n < 5 }, "bad {n}"); n
        }
        pub fn saved_option(n: usize) -> usize { let value = n.checked_sub(1); value.unwrap() }
        pub fn flags(n: usize) { assert!(n < 5, "bad {n:x}"); }
        pub fn effect(n: usize, hits: &mut usize) { assert!(n < 5, "{}", { *hits += 1; *hits }); }
        #[derive(Clone, Copy)] pub struct Custom(u32);
        impl std::fmt::Display for Custom {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("custom") }
        }
        pub fn custom(n: usize, value: Custom) { assert!(n < 5, "{value}"); }
        struct Guard;
        impl Drop for Guard { fn drop(&mut self) {} }
        fn dropping(n: usize) -> u16 { let _guard = Guard; u16::try_from(n).unwrap() }
        pub fn dropping_caller(n: usize) -> u16 { dropping(n) }
    "#).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(name, "narrow" | "after_condition") {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
        if name == "narrow" {
            assert_eq!(function["extraction_evidence"]["cfg"]["status"], "partial");
        }
    }
    check_aggregate_documents(&report);
    check_diagnostic_snapshot(&report);
}

fn check_diagnostic_snapshot(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["facts"]["name"] == "after_condition")
        .unwrap();
    let (unit, index) = data_unit(function);
    let pair = unit.pair(index).unwrap();
    let assertion = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("core::assert.scalar-format"))
        .unwrap();
    let diagnostic = pair
        .pdg
        .edges
        .iter()
        .find(|edge| {
            edge.target == assertion.id
                && edge.subkind == "value_operand"
                && edge.operand_position == Some(1)
        })
        .unwrap();
    let definitions: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == diagnostic.source && edge.subkind == "reaching_definition")
        .collect();
    assert_eq!(definitions.len(), 1);
    let assignment = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.id == definitions[0].source)
        .unwrap();
    assert_eq!(
        assignment.original_kind.known_value(),
        Some("expr_assign_op")
    );
}

#[test]
fn checked_integer_runtime_contract_matches_independent_boundary_laws() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../../tests/fixtures/function-families/end-of-input.rs"),
        r#"
        fn narrow(n: usize) -> u16 { u16::try_from(n).unwrap() }
        #[test]
        fn independent_laws() {
            for n in 0..=65535usize { assert_eq!(usize::from(narrow(n)), n); }
            for n in [65536, usize::MAX] {
                assert!(std::panic::catch_unwind(|| narrow(n)).is_err());
            }
            for n in [0, 1, 255, 256, 65535, 65536, usize::MAX] {
                for amount in [0, 1, 255, 256, 65535, 65536, usize::MAX] {
                    let actual = std::panic::catch_unwind(|| checked(n, amount));
                    if n >= amount {
                        assert_eq!(actual.unwrap() as u128 + amount as u128, n as u128);
                    } else { assert!(actual.is_err()); }
                }
            }
            for n in 0..=256 { assert_eq!(direct(n), n); assert_eq!(saved(n), n); }
            for n in [257, 65535, 65536, usize::MAX] {
                assert!(std::panic::catch_unwind(|| direct(n)).is_err());
            }
        }
    "#
    );
    let path = directory.path().join("src/lib.rs");
    std::fs::write(&path, source).unwrap();
    let binary = directory.path().join("oracle");
    let output = Command::new("rustc")
        .args(["--test", "--edition", "2021"])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Command::new(binary).status().unwrap().success());
}
