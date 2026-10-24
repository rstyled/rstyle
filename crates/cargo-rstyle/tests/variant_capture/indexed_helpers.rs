use super::*;

const SOURCE: &str = r#"
    pub fn read(a: &[u8], i: usize) -> u8 { a[i] }
    pub fn pair(a: &[u8], i: usize, j: usize) -> bool { read(a, i) == read(a, j) }
    pub fn chain(a: &[u8], i: usize, j: usize) -> bool { pair(a, i, j) }
    pub fn snapshot<'a>(mut a: &'a [u8], b: &'a [u8]) -> u8 {
        read(a, { a = b; 0 })
    }
"#;

#[test]
fn indexed_helpers_keep_distinct_selectors_nested_calls_and_saved_argument_origins() {
    let directory = clone_workspace();
    runtime_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    assert_eq!(report["functions"].as_array().unwrap().len(), 4);
    for function in report["functions"].as_array().unwrap() {
        assert_eq!(
            function["outcome"], "admitted",
            "{}: {}",
            function["facts"]["name"], function["reasons"]
        );
        check_function(function);
        if function["facts"]["name"] == "pair" {
            reject_false_totality(function);
        }
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
}

fn check_function(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    assert_eq!(
        pair.cfg.extraction_evidence.status,
        rstyle_pdg::ExtractionStatus::Partial
    );
    let locations: Vec<_> = pair
        .pdg
        .memory_locations
        .iter()
        .filter(|location| location.abstraction == "rstyle.formal-helper-index/1")
        .collect();
    match facts.name.as_str() {
        "read" => assert!(locations.is_empty()),
        "pair" | "chain" => assert_eq!(locations.len(), 2),
        "snapshot" => {
            assert_eq!(locations.len(), 1);
            assert_eq!(
                locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"],
                0
            );
        }
        _ => unreachable!(),
    }
    if !locations.is_empty() {
        reject_scopes(pair, &facts);
        reject_old_policy(pair, &facts);
    }
}

fn reject_scopes(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    for mutation in ["call_span", "operation", "access", "path"] {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        let location = changed
            .pdg
            .memory_locations
            .iter_mut()
            .find(|location| location.abstraction == "rstyle.formal-helper-index/1")
            .unwrap();
        let scope = location
            .extensions
            .get_mut(rstyle_pdg::data_documents::LOCATION_KEY)
            .unwrap()["path"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        scope[mutation] = match mutation {
            "call_span" => serde_json::json!([0, 0]),
            "operation" => "invented-read".into(),
            "access" => 999.into(),
            "path" => serde_json::json!([]),
            _ => unreachable!(),
        };
        assert!(
            rstyle_pdg::data_documents::verify(&changed, facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&changed, facts).is_err(),
            "{mutation}"
        );
    }
}

fn reject_old_policy(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    changed.pdg.profile = rstyle_pdg::extraction_profile::ExtractionProfile::DataV2.policy();
    changed.cfg.profile.name = changed.pdg.profile.id.clone();
    changed.cfg.profile.version = changed.pdg.profile.version.clone();
    let error = rstyle_pdg::data_documents::verify(&changed, facts).unwrap_err();
    assert!(error.contains("Indexed helper reads require"), "{error}");
    assert!(rstyle_ccgraph::admit_produced(&changed, facts).is_err());
}

fn reject_false_totality(function: &Value) {
    shared_evidence::reject_edit(function, |bundle, _| {
        let helper = bundle["helpers"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|helper| {
                helper["summary"]["memory_reads"]
                    .as_array()
                    .is_some_and(|reads| {
                        reads.iter().any(|read| {
                            read["path"].as_array().is_some_and(|path| {
                                path.iter().any(|step| step["kind"] == "IndexedElement")
                            })
                        })
                    })
            })
            .unwrap();
        helper["summary"]["may_panic"] = false.into();
        helper["summary"]["always_returns"] = true.into();
    });
}

fn runtime_oracle(directory: &Path) {
    let oracle = r#"
        #[test] fn independent_reads_and_argument_order() {
            assert_eq!(read(&[7, 8], 0), 7);
            assert!(!pair(&[7, 8], 0, 1));
            assert!(!chain(&[7, 8], 0, 1));
            assert!(pair(&[7], 0, 0));
            assert_eq!(snapshot(&[7], &[]), 7);
            assert!(std::panic::catch_unwind(|| read(&[], 0)).is_err());
            assert!(std::panic::catch_unwind(|| chain(&[7], 0, 1)).is_err());
            assert!(std::panic::catch_unwind(|| snapshot(&[], &[7])).is_err());
        }
    "#;
    let source = directory.join("oracle.rs");
    let binary = directory.join("oracle");
    std::fs::write(&source, format!("{SOURCE}{oracle}")).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021"])
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executed = Command::new(binary).output().unwrap();
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stdout)
    );
}
