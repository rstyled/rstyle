//! Independent u16 blanket-selection receipts compose only through closed never errors.
use super::*;
use rstyle_pdg::{producer::Pair, unit_store::UnitStore};

const U16: &str = "try_read_u16_as_usize";
const TWIN: &str = "same_u16_widening";
const RECEIPT: &str = "usize_reader_conversion";
const ATTACKS: &[&str] = &[
    "root-selection",
    "root-impl",
    "into-impl",
    "from-selection",
    "from-impl",
    "substitution",
    "signature",
    "missing-obligation",
    "never-carrier",
    "never-input",
    "callback-order",
    "failure-roster",
    "capture-root",
];

fn fixture_source() -> String {
    let selected_sources = selection();
    let original = selected(&selected_sources, U16);
    std::iter::once(source())
        .chain([
            original.replace(U16, TWIN),
            original.replace(U16, "extra_failure_u16").replace(
                "|_| DeserializeError::invalid_usize(what)",
                "|_| { let _hidden = what.len(); DeserializeError::invalid_usize(what) }",
            ),
        ])
        .chain(
            ATTACKS
                .iter()
                .map(|change| original.replace(U16, &attack_name(change))),
        )
        .collect::<Vec<_>>()
        .join("\n")
}

fn attack_name(change: &str) -> String {
    format!("u16_attack_{}", change.replace('-', "_"))
}

fn diagnostic_capture() -> UnitStore {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), fixture_source()).unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
            .env("RSTYLE_DIAGNOSTIC_SHARED_SOURCES", "1")
            .env("RSTYLE_DIAGNOSTIC_FACTS_DIR", snapshots.path())
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success(), "{report}");
    assert_eq!(report["capture_complete"], false);
    assert_eq!(report["pair_evaluation_complete"], false);
    assert!(report["functions"].as_array().unwrap().is_empty());
    let files: Vec<_> = std::fs::read_dir(snapshots.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.to_string_lossy()
                .ends_with("-pre-helper-certification.json")
        })
        .collect();
    assert_eq!(
        files.len(),
        1,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let facts =
        rstyle_pdg::unit_store::read_diagnostic_snapshot(&std::fs::read(&files[0]).unwrap())
            .unwrap();
    UnitStore::capture(facts).unwrap()
}

#[test]
fn original_u16_widening_closes_never_error_without_granting_generic_conversion() {
    // Existing exhaustive original-source oracle fixes expectations independently:
    // lengths0/1 return the exact BufferTooSmall message before any native read;
    // all65536 native u16 values widen unchanged, preserving consumed2. No value
    // reaches InvalidUsize or invokes its callback. Complete callback source and
    // immutable captures remain mandatory even on that proved impossible path.
    let raw = diagnostic_capture();
    let mut bytes = Vec::new();
    raw.write_bundle(&mut bytes).unwrap();
    let mut bundle: Value = serde_json::from_slice(&bytes).unwrap();
    let positives = [U16, TWIN, NAME].map(|name| wire_index(&bundle, name));
    let mutants = install_attacks(&raw, &mut bundle);
    let raw = import_resealed(bundle).unwrap();
    let store = raw.with_slice_length_instances().unwrap();
    let extra = store
        .functions()
        .iter()
        .position(|facts| facts.name == "extra_failure_u16")
        .unwrap();
    assert!(
        store.pair(extra).is_none(),
        "unreachable failure source cannot be omitted"
    );
    check_controls(&store, positives);
    check_mutants(&store, mutants);
    check_graph_attacks(&store, positives[0]);
    check_previous_u32(&store, positives[2]);
}

fn install_attacks(raw: &UnitStore, bundle: &mut Value) -> Vec<(&'static str, usize)> {
    let selected_sources = selection();
    let original = selected(&selected_sources, U16);
    ATTACKS
        .iter()
        .map(|&change| {
            let name = attack_name(change);
            let index = wire_index(bundle, &name);
            assert_eq!(
                raw.functions()[index].source.bytes,
                original.replace(U16, &name)
            );
            assert!(
                bundle["roots"][index].is_null(),
                "raw capture grants no wrapper root"
            );
            mutate_attack(&mut bundle["functions"][index], change);
            (change, index)
        })
        .collect()
}

fn conversion(facts: &mut Value) -> &mut Value {
    let calls = facts["build"]["frontend_options"][INSTANCE]["callbacks"][0]["calls"]
        .as_array_mut()
        .unwrap();
    let indices: Vec<_> = calls
        .iter()
        .enumerate()
        .filter_map(|(index, call)| {
            call.get(RECEIPT)
                .filter(|value| !value.is_null())
                .map(|_| index)
        })
        .collect();
    assert_eq!(indices.len(), 1, "one original scoped conversion receipt");
    &mut calls[indices[0]][RECEIPT]
}

fn mutate_attack(facts: &mut Value, change: &str) {
    match change {
        "never-carrier" => mutate_carrier(&mut facts["body"]),
        "never-input" => mutate_never_input(facts),
        "callback-order" => {
            let closure = facts.pointer_mut(SUCCESS).unwrap();
            assert_eq!(closure["kind"], "expr_closure");
            let tuple = &mut closure["children"][1];
            assert_eq!(tuple["kind"], "expr_tup");
            tuple["children"].as_array_mut().unwrap().swap(0, 1);
        }
        "failure-roster" => {
            let failure_hir = facts.pointer(FAILURE).unwrap()["hir"].clone();
            let callbacks =
                facts["build"]["frontend_options"][INSTANCE]["callbacks"][0]["callbacks"]
                    .as_array_mut()
                    .unwrap();
            let index = callbacks
                .iter()
                .position(|callback| callback["hir"] == failure_hir)
                .unwrap();
            assert_eq!(callbacks.len(), 2);
            callbacks.remove(index);
        }
        "capture-root" => mutate_environment(facts, "capture-root"),
        _ => mutate_receipt(conversion(facts), change),
    }
}

fn mutate_receipt(receipt: &mut Value, change: &str) {
    assert_eq!(
        receipt["identity"],
        "rstyle.rust.usize-reader-conversion-closure/1"
    );
    match change {
        "root-selection" => {
            receipt["try_from"]["selected"]["definition_identity"] = "forged".into()
        }
        "root-impl" => {
            receipt["try_from"]["selected_impl"]["definition_identity"] = "forged".into()
        }
        "into-impl" => receipt["into"]["selected_impl"]["definition_identity"] = "forged".into(),
        "from-selection" => receipt["from"]["selected"]["definition_identity"] = "forged".into(),
        "from-impl" => receipt["from"]["selected_impl"]["definition_identity"] = "forged".into(),
        "substitution" => {
            receipt["into"]["requested_arguments"][0]["identity"]["name"] = "u32".into();
            receipt["into"]["selected_arguments"][0]["identity"]["name"] = "u32".into();
        }
        "signature" => {
            receipt["from"]["structural_signature"]["output"][0]["identity"]["name"] = "u32".into()
        }
        "missing-obligation" => {
            receipt.as_object_mut().unwrap().remove("from");
        }
        _ => unreachable!(),
    }
}

fn mutate_carrier(body: &mut Value) {
    if body["kind"] == "expr_call" && body["snippet"] == "usize::try_from(n)" {
        assert!(body["ty"]["normalized"].as_str().is_some());
        // A valid older inhabited conversion carrier cannot authorize a never
        // obligation, even when source spelling and output scalar agree.
        body["ty"]["normalized"] =
            "{\"kind\":\"int_conversion_result\",\"value\":\"usize\"}".into();
        return;
    }
    let child = body["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|child| contains_conversion(child))
        .expect("owned conversion path");
    mutate_carrier(child);
}

fn contains_conversion(node: &Value) -> bool {
    node["kind"] == "expr_call" && node["snippet"] == "usize::try_from(n)"
        || node["children"]
            .as_array()
            .is_some_and(|children| children.iter().any(contains_conversion))
}

fn mutate_never_input(facts: &mut Value) {
    let closure = facts.pointer_mut(FAILURE).unwrap();
    let mut environment: Value =
        serde_json::from_str(closure["res"]["description"].as_str().unwrap()).unwrap();
    assert_eq!(environment["signature_inputs"][0]["status"], "Captured");
    environment["signature_inputs"][0] = json!({"status":"Captured", "ty":{
        "kind":"Tuple", "evidence":[{"kind":"Scalar", "evidence":"u8"}]}});
    closure["res"]["description"] = serde_json::to_string(&environment).unwrap().into();
}

fn check_controls(store: &UnitStore, indices: [usize; 3]) {
    rstyle_pdg::verification_policy::strict(|| {
        let analysis = store.analysis();
        let admitted: Vec<_> = indices
            .map(|index| {
                let facts = analysis.function(index).unwrap();
                let pair = store.pair(index).unwrap_or_else(|| {
                    panic!(
                        "complete original control must project: {} {:?}",
                        facts.name,
                        rstyle_pdg::classify(&facts)
                    )
                });
                rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
                    .unwrap();
                rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
                rstyle_ccgraph::admit_produced(pair, &facts).unwrap()
            })
            .into_iter()
            .collect();
        let matching =
            rstyle_ccgraph::evaluate_all(&admitted[..2], &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!matching.incomplete, "{:?}", matching.diagnostics);
        assert_eq!(matching.pairs.len(), 1);
        assert_eq!(
            matching.pairs[0].decision,
            rstyle_ccgraph::Decision::Accepted
        );
        assert_eq!(matching.pairs[0].displayed_score.as_deref(), Some("1"));
    });
}

fn check_mutants(store: &UnitStore, mutants: Vec<(&str, usize)>) {
    let analysis = store.analysis();
    rstyle_pdg::verification_policy::strict(|| {
        for (change, index) in mutants {
            let facts = analysis.function(index).unwrap();
            assert_ne!(
                rstyle_pdg::classify(&facts).status,
                rstyle_pdg::EligibilityVerdict::Eligible,
                "{change}"
            );
            assert!(
                store.pair(index).is_none(),
                "mutant acquired a pair: {change}"
            );
        }
    });
}

fn previous_profile(pair: &mut Pair) {
    let previous = "3.128.0".to_owned();
    pair.pdg.profile.version = previous.clone();
    pair.cfg.profile.version = previous;
}

fn check_graph_attacks(store: &UnitStore, index: usize) {
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    for change in ["origin", "guard", "operand", "entry", "previous-profile"] {
        let mut changed = store.pair(index).unwrap().clone();
        if change == "previous-profile" {
            previous_profile(&mut changed);
        } else {
            mutate_pair(&mut changed, change);
        }
        rstyle_pdg::verification_policy::strict(|| {
            assert!(
                rstyle_pdg::producer::verify_serialized(&changed.serialize().unwrap(), &facts)
                    .is_err(),
                "{change}"
            );
            assert!(
                rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
                "{change}"
            );
        });
    }
}

fn check_previous_u32(store: &UnitStore, index: usize) {
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let mut historical = store.pair(index).unwrap().clone();
    previous_profile(&mut historical);
    rstyle_pdg::verification_policy::strict(|| {
        // Superseded Data3 policies are unregistered in this pipeline. Current
        // u32 positive replay is checked above; historical IDs remain closed.
        assert!(!rstyle_pdg::extraction_profile::registered(
            &historical.pdg.profile
        ));
        assert!(
            rstyle_pdg::producer::verify_serialized(&historical.serialize().unwrap(), &facts)
                .is_err()
        );
        assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    });
}
