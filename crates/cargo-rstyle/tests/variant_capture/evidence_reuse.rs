use super::*;

#[test]
fn immutable_receiver_proofs_keep_owner_policy_source_mode_and_graph_checks_separate() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn length(a: &[u8]) -> usize { a.len() }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let function = &report["functions"][0];
    assert_eq!(function["outcome"], "admitted");
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let full = rstyle_pdg::data_receivers::verify(&facts).unwrap();
    assert!(matches!(full, std::borrow::Cow::Borrowed(_)));
    let borrowed = &facts;
    let repeated = rstyle_pdg::data_receivers::verify(&borrowed).unwrap();
    assert!(std::ptr::eq(full.as_ref(), repeated.as_ref()));
    let capture = rstyle_pdg::data_receivers::verify_capture(&facts).unwrap();
    assert!(!std::ptr::eq(full.as_ref(), capture.as_ref()));
    rstyle_pdg::verification_policy::generated(|| {
        let generated = rstyle_pdg::data_receivers::verify(&facts).unwrap();
        assert_eq!(full, generated);
        assert!(!std::ptr::eq(full.as_ref(), generated.as_ref()));
    });
    let independent = unit.analysis();
    let other = independent.function(index).unwrap();
    let separate = rstyle_pdg::data_receivers::verify(&other).unwrap();
    assert!(!std::ptr::eq(full.as_ref(), separate.as_ref()));

    rstyle_pdg::data_frontend::verify(&facts).unwrap();
    let pair = unit.pair(index).unwrap();
    rstyle_pdg::producer::verify(pair, &facts).unwrap();
    reject_changed_graph(pair, &facts);
    let mut standalone = rstyle_pdg::unit_store::EvidenceFacts::facts(&facts).clone();
    standalone
        .build
        .frontend_options
        .get_mut(rstyle_pdg::data_receivers::KEY)
        .unwrap()["methods"][0]["signature_result"] =
        serde_json::json!({"status":"Captured", "ty":{"kind":"Scalar", "evidence":"bool"}});
    assert!(rstyle_pdg::data_receivers::verify(&standalone).is_err());
    assert!(rstyle_pdg::data_receivers::verify(&facts).is_ok());
}

fn reject_changed_graph(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    changed
        .pdg
        .nodes
        .retain(|node| node.category != rstyle_pdg::NodeCategory::Call);
    assert!(rstyle_pdg::producer::verify(&changed, facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&changed, facts).is_err());
}
