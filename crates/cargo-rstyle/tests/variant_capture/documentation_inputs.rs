use super::float_kernels::{function, reject_resealed};
use super::*;

const SOURCE: &str = "#![doc = include_str!(\"../README.md\")]\npub fn checked(n: usize) -> Option<usize> { n.checked_mul(4) }\n";

fn capture(source: &str, documentation: &str) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    std::fs::write(directory.path().join("README.md"), documentation).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn literal_documentation_retains_full_bytes_and_root_ownership_under_strict_replay() {
    for bytes in ["# UTF8 π\n", "# Windows\r\n", "\u{feff}# BOM\n", ""] {
        let (_directory, report) = capture(SOURCE, bytes);
        let entry = function(&report, "checked");
        assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
        let unit = super::word_classifiers::verify_admissions(&report);
        let census: serde_json::Value =
            serde_json::to_value(&unit.functions()[0].build.frontend_options).unwrap();
        let census = &census[rstyle_pdg::nominal_boundary::census::KEY];
        assert_eq!(census["files"].as_array().unwrap().len(), 2);
        assert_eq!(census["documentation"].as_array().unwrap().len(), 1);
        let input = &census["documentation"][0];
        assert_eq!(
            input["input"]["span"]["start"],
            if bytes.starts_with('\u{feff}') { 3 } else { 0 }
        );
        assert_eq!(input["input"]["span"]["end"], bytes.len());
    }
}

#[test]
fn documentation_resealed_owner_input_authority_and_content_mutations_reject() {
    let (_directory, report) = capture(SOURCE, "# Actual original input\n");
    let entry = function(&report, "checked");
    assert_eq!(entry["outcome"], "admitted");
    for mutation in [
        "owner",
        "span",
        "physical_path",
        "builtin",
        "expanded",
        "partial",
        "missing",
        "duplicate",
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let inputs = &mut bundle["functions"][0]["build"]["frontend_options"]
            [rstyle_pdg::nominal_boundary::census::KEY]["value"]["documentation"];
        match mutation {
            "owner" => inputs[0]["invocation"]["path"] = "/outside/lib.rs".into(),
            "span" => inputs[0]["invocation"]["span"]["start"] = 0.into(),
            "physical_path" => inputs[0]["physical_path"] = "/outside/README.md".into(),
            "builtin" => inputs[0]["builtin"]["definition_hash"] = "unresolved".into(),
            "expanded" => inputs[0]["expanded_digest"] = "changed".into(),
            "partial" => inputs[0]["input"]["span"]["end"] = 1.into(),
            "missing" => *inputs = serde_json::json!([]),
            "duplicate" => {
                let copy = inputs[0].clone();
                inputs.as_array_mut().unwrap().push(copy);
            }
            _ => unreachable!(),
        }
        reject_resealed(bundle, mutation);
    }
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    changed.pdg.profile.version = "3.72.0".into();
    changed.cfg.profile.version = "3.72.0".into();
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&changed, &facts).is_err());
}

#[test]
fn documentation_does_not_hide_code_includes_or_neighboring_shadow_macros() {
    for source in [
        format!("{SOURCE}\ninclude!(\"../README.md\");"),
        format!(
            "{SOURCE}\n#[macro_export]\nmacro_rules! include_str {{ ($p:literal) => {{ \"shadow\" }}; }}\ninclude!(\"../README.md\");"
        ),
    ] {
        let (_directory, report) = capture(&source, "pub struct Hidden;\n");
        assert_eq!(function(&report, "checked")["outcome"], "excluded");
    }
}

#[cfg(unix)]
#[test]
fn documentation_symlink_redirects_outside_package_remain_excluded() {
    let directory = clone_workspace();
    let outside = tempfile::tempdir().unwrap();
    let input = outside.path().join("README.md");
    std::fs::write(&input, "# Outside package\n").unwrap();
    std::os::unix::fs::symlink(input, directory.path().join("README.md")).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    assert_eq!(function(&report, "checked")["outcome"], "excluded");
}

#[test]
fn documentation_does_not_authorize_external_macro_imports_for_length_family() {
    let original = include_str!("../../../../eval/coverage-development/base64ct-length.rs");
    let source = format!("{SOURCE}\n#[macro_use] extern crate alloc;\n{original}");
    let (_directory, report) = capture(&source, "# Documentation\n");
    let selected = function(&report, "encoded_len_inner");
    assert_eq!(selected["outcome"], "excluded");
    assert!(
        selected["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| { reason.as_str().unwrap().contains("unsupported attributes") })
    );
}
