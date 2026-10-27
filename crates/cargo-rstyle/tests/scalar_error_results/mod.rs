//! A fresh non-Copy scalar error remains distinct from its Copy success payload.
use super::*;

macro_rules! fixture {
    ($($source:tt)*) => {
        mod oracle { $($source)* }
        const SOURCE: &str = stringify!($($source)*);
    };
}

fixture! {
#[derive(Copy, Clone)]
pub struct Index(u32);
pub struct LimitError { attempted: u64 }
impl Index { pub fn value(&self) -> u32 { self.0 } }
impl LimitError { pub fn attempted(&self) -> u64 { self.attempted } }
pub fn checked(value: u32) -> Result<Index, LimitError> {
    if value >= 7 { return Err(LimitError { attempted: value as u64 }); }
    Ok(Index(value))
}
pub fn untyped_error(value: u64) -> LimitError { LimitError { attempted: value } }
pub fn forwarded(value: u32) -> Result<Index, LimitError> { checked(value) }
pub fn same(value: u32) -> Result<Index, LimitError> { checked(value) }
pub fn stored(value: u32) -> Result<Index, LimitError> { let saved = checked(value); if value == 0 { return saved; } saved }
pub fn consumed(value: Result<Index, LimitError>) -> Result<Index, LimitError> { value }
}

#[test]
fn scalar_error_values_follow_the_independent_boundary_expectation() {
    assert_eq!(oracle::untyped_error(13).attempted(), 13);
    for value in [0, 1, 6, 7, 8, u32::MAX] {
        for result in [
            oracle::checked(value),
            oracle::forwarded(value),
            oracle::same(value),
            oracle::stored(value),
            oracle::consumed(oracle::checked(value)),
        ] {
            match result {
                Ok(index) => {
                    assert!(value < 7);
                    assert_eq!(index.value(), value);
                }
                Err(error) => {
                    assert!(value >= 7);
                    assert_eq!(error.attempted(), u64::from(value));
                }
            }
        }
    }
}

#[test]
fn fresh_scalar_error_results_compose_without_copy_or_storage_permission() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    let mut wrappers = Vec::new();
    for name in [
        "checked",
        "forwarded",
        "same",
        "stored",
        "consumed",
        "untyped_error",
    ] {
        let function = functions
            .iter()
            .find(|f| f["facts"]["name"] == name)
            .unwrap();
        let positive = matches!(name, "checked" | "forwarded" | "same" | "untyped_error");
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{name}: {}",
            function["reasons"]
        );
        if positive {
            let mut imported = verify_strict_and_historical(function);
            imported.key = name.into();
            if matches!(name, "forwarded" | "same") {
                wrappers.push(imported);
            }
        }
        if name == "untyped_error" {
            // This direct scalar constructor has its own fresh no-drop proof.
            crate::fresh_scalar_owners::verify_boundary(function);
        }
    }
    check_aggregate_documents(&report);
    let matched =
        rstyle_ccgraph::evaluate_all(&wrappers, &rstyle_ccgraph::CcgraphConfig::default());
    assert!(!matched.incomplete);
    assert_eq!(matched.pairs.len(), 1);
    assert_eq!(
        matched.pairs[0].decision,
        rstyle_ccgraph::Decision::Accepted
    );
    assert_eq!(matched.pairs[0].displayed_score.as_deref(), Some("1"));
}

fn verify_strict_and_historical(function: &Value) -> rstyle_ccgraph::ImportedFunction {
    let (store, index) = shared_evidence::load(function);
    let pair = store.pair(index).unwrap();
    let documents = pair.serialize().unwrap();
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&documents, &facts)
    })
    .unwrap();
    let imported =
        rstyle_pdg::verification_policy::strict(|| rstyle_ccgraph::admit_produced(pair, &facts))
            .unwrap();
    let mut historical = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    historical.pdg.profile.version = "3.96.0".into();
    historical.cfg.profile.version = "3.96.0".into();
    assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    let old_documents = historical.serialize().unwrap();
    assert!(
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&old_documents, &facts)
        })
        .is_err()
    );
    imported
}
