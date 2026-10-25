//! Earlier namespace evidence cannot certify cleanup or erase unknown work.
use super::*;

fn source() -> String {
    bytes::source()
        + "\n#[derive(Copy, Clone)] pub struct Second(Base);\n\
         identifiers!(Second, SecondError, SecondIter, WithSecondIter);\n"
}

#[test]
fn ordered_generated_constructors_compose_after_a_complete_prior_expansion() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "Wrapper::new",
        "Wrapper::same",
        "Second::new",
        "Second::same",
    ] {
        assert_eq!(function(&report, name)["outcome"], "admitted", "{name}");
    }
    rstyle_pdg::verification_policy::strict(|| {
        super::super::word_classifiers::verify_admissions(&report)
    });
    check_equivalent_matching(&report, ["Wrapper::new", "Second::new"]);
    check_equivalent_matching(&report, ["Second::new", "Second::same"]);
    reject_selected_constructor(function(&report, "Second::new"));
    let entry = function(&report, "Second::new");
    let (unit, _) = shared_evidence::load(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let pair = shared_evidence::pair(&unit, entry);
    let mut forged = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    forged.pdg.profile.version = "3.114.0".into();
    forged.cfg.profile.version = "3.114.0".into();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&forged.serialize().unwrap(), &facts)
        })
        .is_err()
    );
}

#[test]
fn preceding_textual_receipt_grants_no_drop_or_unknown_macro_permission() {
    for changed in [
        source().replace(
            "pub struct $err(BaseError);",
            "pub struct $err(BaseError); impl Drop for $err { fn drop(&mut self) {} }",
        ),
        source().replace(
            "identifiers!(Wrapper",
            "macro_rules! unknown { ($x:expr) => {}; } unknown!(0);\nidentifiers!(Wrapper",
        ),
    ] {
        let directory = clone_workspace();
        std::fs::write(directory.path().join("src/lib.rs"), changed).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for name in ["Second::new", "Second::same"] {
            assert_eq!(function(&report, name)["outcome"], "excluded", "{name}");
        }
    }
}
