//! Whole non-Copy error transfer must preserve its exact payload and static root.
use super::*;
use serde_json::json;

const TRANSFER: &str = "rstyle.data:named-error-owned-transfer";
const ORIGINS: &str = "rstyle.data:named-error-value-origins";
const CONSTRUCTORS: &str = r#"
impl Error {
    pub fn first(err: FirstError, what: &'static str) -> Error {
        Error(ErrorKind::First { err, what })
    }
    pub fn same(err: FirstError, what: &'static str) -> Error {
        Error(ErrorKind::First { err, what })
    }
    pub fn second(err: SecondError, what: &'static str) -> Error {
        Error(ErrorKind::Second { err, what })
    }
    pub fn extra(err: FirstError, what: &'static str) -> Error {
        let ignored = what;
        Error(ErrorKind::First { err, what })
    }
    pub fn discarded(err: FirstError, what: &'static str) -> Error {
        Error(ErrorKind::Generic { msg: what })
    }
}
"#;

#[test]
fn owned_error_constructors_preserve_once_only_moves_and_static_message_components() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{CONSTRUCTORS}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["Error::first", "Error::same", "Error::second"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in ["Error::extra", "Error::discarded"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    check_equivalent_matching(&report, ["Error::first", "Error::same"]);
    check_callable_receipts(&report);
    strict_composition(&report);
}

fn check_callable_receipts(report: &Value) {
    let bundle = shared_evidence::wire(function(report, "Error::first"));
    for name in ["Error::first", "Error::same", "Error::second"] {
        let index = shared_evidence::index(function(report, name));
        let helper = bundle["certified"][index].as_u64().unwrap() as usize;
        let summary = &bundle["helpers"][helper]["summary"];
        assert_eq!(
            summary["identity"],
            "rstyle.helper.named-error-owned-constructor/1"
        );
        assert_eq!(summary["argument_evaluation"], "source-order-once/1");
        assert_eq!(summary["may_panic"], false);
        assert_eq!(summary["always_returns"], true);
        assert_eq!(summary["memory_effects"], false);
    }
}

fn strict_composition(report: &Value) {
    let (store, _) = shared_evidence::load(function(report, "Error::first"));
    rstyle_pdg::verification_policy::strict(|| {
        let analysis = store.analysis();
        let mut imports = Vec::new();
        for name in ["Error::first", "Error::same", "Error::second"] {
            let index = shared_evidence::index(function(report, name));
            let facts = analysis.function(index).unwrap();
            let pair = store.pair(index).unwrap();
            rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
            reject_mutations(pair, &facts);
            let mut old = pair.clone();
            old.pdg.profile.version = "3.123.0".into();
            old.cfg.profile.version = "3.123.0".into();
            assert!(rstyle_ccgraph::admit_produced(&old, &facts).is_err());
            imports.push(rstyle_ccgraph::admit_produced(pair, &facts).unwrap());
        }
        let result =
            rstyle_ccgraph::evaluate_all(&imports, &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!result.incomplete);
        assert_eq!(result.pairs.len(), 3);
    });
}

fn reject_mutations(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    for change in 0..8 {
        let mut changed = pair.clone();
        if change == 6 {
            changed.cfg.extensions.insert(TRANSFER.into(), json!({}));
        } else {
            mutate(&mut changed.pdg, change);
        }
        // Verify the typed graph directly; the unmodified wire roundtrip above
        // exercises integrity without repeatedly serializing/resealing mutations.
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "owned transfer mutation {change}"
        );
    }
}

fn mutate(pdg: &mut rstyle_pdg::PdgDocument, change: usize) {
    if change == 7 {
        pdg.extensions.insert(TRANSFER.into(), json!({}));
        return;
    }
    let node = pdg
        .nodes
        .iter_mut()
        .find(|node| node.annotations.contains_key(TRANSFER))
        .unwrap();
    let transfer = node.annotations.get_mut(TRANSFER).unwrap();
    match change {
        0 => {
            transfer["parameter"] = json!(1);
        }
        1 => {
            transfer["output"] = json!([]);
        }
        2 => {
            transfer["input"]["kind"] = json!("Struct");
        }
        3 => {
            node.annotations.remove(TRANSFER);
        }
        4 => {
            node.annotations
                .insert("rstyle.data:reference-origins".into(), json!([]));
        }
        _ => {
            node.annotations.remove(ORIGINS);
        }
    }
}

#[test]
fn owned_error_constructors_reject_hidden_drop_in_the_complete_family() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{CONSTRUCTORS}\nmod hidden {{ use crate::SecondError as Alias; impl Drop for Alias {{ fn drop(&mut self) {{}} }} }}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["Error::first", "Error::same", "Error::second"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
}
