use super::float_kernels::function;
use super::*;

const ORIGINAL: &str = include_str!("../../../../eval/coverage-development/base64ct-length.rs");
const EXTRA: &str = r#"
pub fn caller(n: usize, padded: bool) -> Option<usize> { encoded_len_inner(n, padded) }
pub fn same(n: usize, padded: bool) -> Option<usize> { encoded_len_inner(n, padded) }
pub fn cast(value: bool) -> usize { value as usize }
pub fn snapshot(mut value: bool) -> (usize, bool) {
    let converted = { let saved = value; value = !value; saved } as usize;
    (converted, value)
}
pub fn byte(value: bool) -> u8 { value as u8 }
pub fn character(value: char) -> usize { value as usize }
pub fn unsafe_cast(value: bool) -> usize { unsafe { value as usize } }
"#;

fn capture(target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("README.md"),
        "# Full retained documentation π\n",
    )
    .unwrap();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}#![doc = include_str!(\"../README.md\")]\n{ORIGINAL}\n{EXTRA}"),
    )
    .unwrap();
    let mut arguments = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        arguments.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &arguments);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn bool_length_family_composes_checked_values_matches_saved_casts_and_doc_inputs() {
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
            let (_directory, report) = capture(Some(target), no_std);
            assert_eq!(report["functions"].as_array().unwrap().len(), 8);
            for entry in report["functions"].as_array().unwrap() {
                let name = entry["facts"]["name"].as_str().unwrap();
                assert_eq!(
                    entry["outcome"],
                    if ["byte", "character", "unsafe_cast"].contains(&name) {
                        "excluded"
                    } else {
                        "admitted"
                    },
                    "{name}: {}",
                    entry["reasons"]
                );
            }
            super::word_classifiers::verify_admissions(&report);
            for flag in [
                "capture_complete",
                "assessment_complete",
                "pair_enumeration_complete",
                "pair_evaluation_complete",
            ] {
                assert_eq!(report[flag], true);
            }
            assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
        }
    }
}

#[test]
fn bool_cast_documents_reject_changed_semantics_effects_operands_and_old_policy() {
    let (_directory, report) = capture(None, false);
    let entry = function(&report, "cast");
    assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    for mutation in [
        "mapping",
        "operator",
        "type",
        "construct",
        "category",
        "effect",
        "operand",
        "policy",
    ] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        mutate_cast(&mut changed, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&changed, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&changed, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn mutate_cast(changed: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let node = changed
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.bool-to-usize"))
        .unwrap();
    match mutation {
        "mapping" => {
            node.annotations
                .get_mut("rstyle.data:bool-to-usize")
                .unwrap()["true"] = "0".into()
        }
        "operator" => node.attributes.operator = Some(rstyle_pdg::StringFact::known("!")),
        "type" => {
            node.annotations.get_mut("compat.rust.hir-r0:type").unwrap()["value"] = "u8".into()
        }
        "construct" => node.attributes.construct = Some("integer.cast".into()),
        "category" => node.category = rstyle_pdg::NodeCategory::Call,
        "effect" => node.attributes.defines.push("invented".into()),
        "operand" => {
            let id = node.id.clone();
            changed
                .pdg
                .edges
                .retain(|edge| !(edge.target == id && edge.subkind == "value_operand"));
        }
        "policy" => {
            changed.pdg.profile.version = "3.72.0".into();
            changed.cfg.profile.version = "3.72.0".into();
        }
        _ => unreachable!(),
    }
}

#[test]
fn bool_length_native_values_follow_independent_ceiling_formula_and_saved_operand() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("native.rs");
    std::fs::write(&source, format!("{ORIGINAL}\n{EXTRA}\n{NATIVE_ORACLE}")).unwrap();
    let binary = directory.path().join("native");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--test"])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

const NATIVE_ORACLE: &str = r#"
#[test]
fn independent_values() {
    assert_eq!(cast(false), 0);
    assert_eq!(cast(true), 1);
    assert_eq!(snapshot(false), (0, true));
    assert_eq!(snapshot(true), (1, false));
    let boundary = usize::MAX / 4;
    let values = (0..1024).chain([boundary - 1, boundary, boundary + 1, usize::MAX]);
    for n in values {
        for padded in [false, true] {
            let n128 = n as u128;
            let expected = if n128 * 4 > usize::MAX as u128 {
                None
            } else if padded {
                Some((((n128 + 2) / 3) * 4) as usize)
            } else {
                Some(((n128 * 4 + 2) / 3) as usize)
            };
            assert_eq!(encoded_len_inner(n, padded), expected, "{n} {padded}");
        }
    }
}
"#;
