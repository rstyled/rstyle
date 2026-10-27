use super::*;

fn source() -> &'static str {
    r#"
    pub fn nested(mut n: u32, b: bool) -> u32 {
        let outer = if b { 1 } else { 2 };
        let mut sum = 0;
        while n > 0 {
            let step = if n & 1 == 0 { outer } else { 3 };
            let mut j = 2;
            while j > 0 { sum += if j == 1 { step } else { outer }; j -= 1; }
            n -= 1;
        }
        sum
    }
    pub fn continues(mut n: u32, b: bool) -> u32 {
        let mut sum = 0;
        while n > 0 {
            n -= 1;
            if n & 1 == 0 { sum += if b { 1 } else { 2 }; continue; }
            sum += if b { 3 } else { 4 };
        }
        sum
    }
    pub fn exits(mut n: u32, b: bool) -> u32 {
        let saved = if b { 5 } else { 7 };
        while n > 0 {
            let step = if n == 1 { 1 } else { 2 };
            if n < step { break; }
            n -= step;
        }
        saved + n
    }
    "#
}

#[test]
fn repeated_values_compose_with_nested_loops_continue_and_break() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 3);
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    check_aggregate_documents(&report);
}

#[test]
fn historical_profile_does_not_gain_cyclic_value_support() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}
