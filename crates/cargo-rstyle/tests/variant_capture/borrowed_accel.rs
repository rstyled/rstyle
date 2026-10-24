use super::*;
mod corruption;

#[test]
fn borrowed_accel_getters_compose_through_strict_matching() {
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
        let (_directory, report) = captured(Some(target));
        check_family(&report);
        check_partial_flow(&report);
    }
}

fn captured(target: Option<&str>) -> (tempfile::TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/borrowed-accel-views.rs"),
    )
    .unwrap();
    let mut flags = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        flags.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &flags);
    assert!(output.status.success(), "{report}");
    (directory, report)
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 18);
    for function in functions {
        let expected = if matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Accel::len"
                | "Accel::is_empty"
                | "Accel::needles"
                | "getter_len"
                | "getter_empty"
                | "getter_view"
                | "first"
                | "saved"
                | "selected"
                | "slice_window"
                | "suffix"
        ) {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(function["outcome"], expected, "{function}");
    }

    check_aggregate_documents(report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
}

fn check_partial_flow(report: &Value) {
    let (unit, _) = data_unit(&report["functions"][0]);
    for function in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| function["outcome"] == "admitted")
    {
        let partial = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "Accel::needles" | "getter_view" | "first" | "saved" | "selected"
        );
        let pair = shared_evidence::pair(&unit, function);
        assert_eq!(
            pair.cfg.extraction_evidence.status,
            if partial {
                rstyle_pdg::ExtractionStatus::Partial
            } else {
                rstyle_pdg::ExtractionStatus::Complete
            },
            "{}",
            function["facts"]["name"]
        );
    }
}

#[test]
fn named_array_lengths_keep_lexical_cfg_and_literal_boundaries() {
    let directory = named_array_workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for function in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            !function["facts"]["name"]
                .as_str()
                .unwrap()
                .contains("clone")
        })
    {
        let negative = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "imported::read" | "nonliteral::read"
        );
        assert_eq!(
            function["outcome"],
            if negative { "excluded" } else { "admitted" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
}

fn named_array_workspace() -> tempfile::TempDir {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub mod one {
            pub const CAP: usize = 8;
            #[derive(Clone)] pub struct Data { bytes: [u8; CAP] }
            pub fn read(value: &Data) -> u8 { value.bytes[0] }
        }
        pub mod two {
            const CAP: usize = 9;
            #[derive(Clone)] pub struct Data { bytes: [u8; CAP] }
            pub fn read(value: &Data) -> u8 { value.bytes[0] }
        }
        pub mod configured {
            #[cfg(any())] const CAP: usize = 8;
            #[cfg(not(any()))] const CAP: usize = 16;
            #[derive(Clone)] pub struct Data { bytes: [u8; CAP] }
            pub fn read(value: &Data) -> u8 { value.bytes[0] }
        }
        pub mod zero {
            const CAP: usize = 0;
            #[derive(Clone)] pub struct Data { bytes: [u8; CAP] }
            pub fn view(value: &Data) -> &[u8] { &value.bytes[0..0] }
            pub fn caller(value: &Data) -> &[u8] { view(value) }
        }
        pub mod imported {
            use crate::one::CAP;
            #[derive(Clone)] pub struct Data { bytes: [u8; CAP] }
            pub fn read(value: &Data) -> u8 { value.bytes[0] }
        }
        pub mod nonliteral {
            const CAP: usize = { 8 };
            #[derive(Clone)] pub struct Data { bytes: [u8; CAP] }
            pub fn read(value: &Data) -> u8 { value.bytes[0] }
        }
    "#,
    )
    .unwrap();
    directory
}
