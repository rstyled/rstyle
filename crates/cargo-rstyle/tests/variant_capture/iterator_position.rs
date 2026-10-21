use super::*;

#[test]
fn byte_iterator_connects_complete_source_first_family_and_boundaries() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            continue;
        }
        for no_std in [false, true] {
            check_context(target, no_std);
        }
    }
}

fn check_context(target: &str, no_std: bool) {
    let directory = clone_workspace();
    let source = include_str!("../../../../eval/coverage-development/iterator-byte-position.rs");
    let source = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, source);
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--target", target],
    );
    assert!(
        output.status.success(),
        "{}; {}",
        String::from_utf8_lossy(&output.stderr),
        report["matching"]["diagnostics"]
    );
    check_family(&report);
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 17);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(
                name,
                "Accel::len"
                    | "Accel::needles"
                    | "Accel::contains"
                    | "slice_contains"
                    | "caller"
                    | "saved"
                    | "selected"
                    | "window"
                    | "first_position"
                    | "saved_position"
                    | "position_caller"
            ) {
                "admitted"
            } else {
                "excluded"
            },
            "{name}: {}",
            function["reasons"]
        );
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    check_strict_documents(report);
}

fn check_strict_documents(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    let analysis = unit.analysis();
    for function in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| function["outcome"] == "admitted")
    {
        let pair = shared_evidence::pair(&unit, function);
        let facts = analysis.function(shared_evidence::index(function)).unwrap();
        rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
    }
}

#[test]
fn byte_iterator_composes_saved_views_and_rejects_unproved_array_places_and_predicates() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), BOUNDARY_SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let expected = if matches!(
            name,
            "explicit" | "snapshot" | "repeated" | "changed" | "check"
        ) {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(
            function["outcome"], expected,
            "{name}: {}",
            function["reasons"]
        );
    }
    check_strict_documents(&report);
}

const BOUNDARY_SOURCE: &str = r#"
        pub struct Bytes { value: [u8; 4] }
        pub fn explicit(a: &Bytes, byte: u8) -> Option<usize> {
            (&a.value[0..3]).iter().position(|&b| byte == b)
        }
        pub fn implicit(a: &Bytes, byte: u8) -> Option<usize> {
            a.value[0..3].iter().position(|&b| b == byte)
        }
        pub fn snapshot<'a>(a: &'a [u8], b: &'a [u8], byte: u8) -> Option<usize> {
            let mut selected = a;
            selected[{ selected = b; 0 }..1].iter().position(|&b| b == byte)
        }
        pub fn repeated(a: &[u8], byte: u8) -> (Option<usize>, Option<usize>) {
            (a.iter().position(|&b| b == byte), a.iter().position(|&b| b == byte))
        }
        pub fn changed(a: &[u8], mut byte: u8) -> Option<usize> {
            a[{ byte = 7; 0 }..1].iter().position(|&b| b == byte)
        }
        pub fn unequal(a: &[u8], byte: u8) -> Option<usize> {
            a.iter().position(|&b| b != byte)
        }
        pub fn hidden(a: &[u8], byte: u8) -> Option<usize> {
            a.iter().position(|&b| check(b, byte))
        }
        fn check(a: u8, b: u8) -> bool { a == b }
        pub fn words(a: &[u16], byte: u16) -> Option<usize> {
            a.iter().position(|&b| b == byte)
        }
    "#;
