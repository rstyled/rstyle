//! Ordinary methods retain exhaustive neighboring expansion checks.
use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Copy)]
pub struct Index(u32);
impl Index {
    pub fn new(value: u32) -> Self { Index(value) }
    pub fn value(&self) -> u32 { self.0 }
    pub fn roundtrip(&self) -> Self { Self::new(self.value()) }
    pub fn same(&self) -> Self { Self::new(self.value()) }
}
macro_rules! neighbors {
    ($a:ident, $b:ident, $c:ident, $d:ident) => {
        pub struct $a;
        pub struct $b;
        pub struct $c;
        pub struct $d;
    };
}
neighbors!(A, B, C, D);
"#;

#[test]
fn copy_owner_source_composes_constructor_and_getter_with_neighboring_expansion() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 5);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let positive = name.starts_with("Index::");
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{name}: {}",
            function["reasons"]
        );
        if positive {
            reject_historical(function);
        }
    }
    check_aggregate_documents(&report);
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_evaluation_complete",
        "pair_enumeration_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
}

fn reject_historical(function: &Value) {
    let (store, index) = shared_evidence::load(function);
    let pair = store.pair(index).unwrap();
    let historical = shared_evidence::with_profile_version(pair, "3.94.0");
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    let documents = historical.serialize().unwrap();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&documents, &facts)
        })
        .is_err()
    );
}
