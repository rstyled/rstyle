//! Independently specified summary roster and tampering of retained Cargo evidence.
use super::*;
use rstyle_pdg::unit_store::UnitStore;

const SUMMARY: &str = "rstyle.helper.guarded-digit-product/1";

#[test]
fn checked_digit_helper_retains_conditional_reads_and_rejects_summary_tampering() {
    let selection = selected_sources();
    let directory = clone_workspace();
    let source = format!(
        "pub(crate) struct ParsedItem<'a,T>(&'a [u8],T);\n\
         pub(crate) struct ExactlyNDigits<const N:u8>;\n\
         impl ExactlyNDigits<2> {{ {} }}",
        selection["functions"][1]["source"].as_str().unwrap()
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let functions: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|function| {
            function["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.ends_with("::parse"))
        })
        .collect();
    assert_eq!(functions.len(), 1);
    assert_eq!(
        functions[0]["outcome"], "admitted",
        "{}",
        functions[0]["reasons"]
    );
    let bundle = shared_evidence::wire(functions[0]);
    let index = shared_evidence::index(functions[0]);
    let helper = bundle["helpers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|helper| helper["function"] == index)
        .unwrap();
    expected(&helper["summary"], 2);
    reject_changes(&bundle);
    inventory_policy(&bundle);
}

fn inventory_policy(original: &Value) {
    let mut changed = original.clone();
    changed["helpers"] = serde_json::json!([]);
    changed["certified"]
        .as_array_mut()
        .unwrap()
        .fill(Value::Null);
    let bytes = serde_json::to_vec(&changed).unwrap();
    let error = UnitStore::from_bundle(&bytes).err().unwrap();
    assert!(
        error.contains("Incomplete unit helper certification inventory"),
        "{error}"
    );
    let historical = rstyle_pdg::ProfileId {
        id: "rstyle.rust.hir-data".into(),
        version: "3.111.0".into(),
    };
    assert!(!rstyle_pdg::extraction_profile::policy_supports(
        &historical,
        "rust.checked-digit-product-summaries"
    ));
}

fn expected(summary: &Value, digits: u64) {
    assert_eq!(summary["identity"], SUMMARY);
    assert_eq!(summary["always_returns"], true);
    assert_eq!(summary["may_panic"], false);
    assert_eq!(summary["memory_effects"], true);
    assert!(summary.get("reference_outputs").is_none());
    let suffix = &summary["component_outputs"][0];
    assert_eq!(suffix["selectors"], serde_json::json!([0, 0]));
    assert_eq!(suffix["origins"][0]["parameter"], 0);
    assert_eq!(
        suffix["origins"][0]["path"],
        serde_json::json!([
            {"kind":"SharedReferent"}, {"kind":"PrefixSuffix", "start":digits}
        ])
    );
    let reads = summary["memory_reads"].as_array().unwrap();
    assert_eq!(reads.len() as u64, 2 * digits);
    for position in 0..digits as usize {
        let selector = &reads[2 * position];
        let body = &reads[2 * position + 1];
        assert_ne!(selector["operation"], body["operation"]);
        assert_eq!(selector["access"], 0);
        assert_eq!(selector["parameter"], 0);
        assert_eq!(
            selector["path"],
            serde_json::json!([
                {"kind":"SharedReferent"}, {"kind":"PrefixElement", "position":position}
            ])
        );
        assert_eq!(selector["path"], body["path"]);
        assert_eq!(
            selector["value"],
            serde_json::json!({"kind":"scalar", "value":"u8"})
        );
    }
}

fn reject_changes(original: &Value) {
    UnitStore::from_bundle(&serde_json::to_vec(original).unwrap()).unwrap();
    let index = original["helpers"]
        .as_array()
        .unwrap()
        .iter()
        .position(|helper| {
            helper["summary"]["memory_reads"]
                .as_array()
                .is_some_and(|reads| reads.len() == 4)
        })
        .unwrap();
    for mutation in 0..7 {
        let mut changed = original.clone();
        tamper(&mut changed["helpers"][index]["summary"], mutation);
        let bytes = serde_json::to_vec(&changed).unwrap();
        let error = UnitStore::from_bundle(&bytes).err().unwrap();
        assert!(
            error.contains("Digit helper summary changes"),
            "mutation {mutation}: {error}"
        );
        assert!(
            rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err(),
            "generated mutation {mutation}"
        );
    }
}

fn tamper(summary: &mut Value, mutation: u8) {
    match mutation {
        0 => {
            summary["memory_reads"].as_array_mut().unwrap().remove(0);
        }
        1 => summary["memory_reads"][1] = summary["memory_reads"][0].clone(),
        2 => summary["component_outputs"][0]["origins"][0]["path"][1]["start"] = 1.into(),
        3 => summary["component_outputs"][0]["selectors"] = serde_json::json!([0, 1]),
        4 => summary["always_returns"] = false.into(),
        5 => summary["result_dependence"] = "verified-formal-referent-substitution/1".into(),
        _ => summary["memory_effects"] = false.into(),
    }
}
