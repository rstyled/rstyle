//! Complete source-selected Result/error helper chains, not isolated enum cases.
use super::float_kernels::function;
use super::*;
use rstyle_pdg::unit_store::{BundleManifest, UnitStore, transport_checksum};
mod attacks;
mod negatives;
mod propagation;

const SELECTION: &str = include_str!(
    "../../../../../tests/fixtures/function-families/scalar-result-next-selection.json"
);
const RESOLUTION: &str = "resolved_result_variant";

fn selected_source() -> String {
    let selected: Value = serde_json::from_str(SELECTION).unwrap();
    let owner = &selected["owner"];
    let mut source = format!(
        "#![no_std]\n#[cfg(feature=\"std\")] extern crate std;\n{}\n{}\n",
        owner["declaration"]["source"].as_str().unwrap(),
        owner["complete_inherent_impl"]["source"].as_str().unwrap(),
    );
    for (index, guard) in [
        (
            2,
            "all(feature=\"unicode-word-boundary\",feature=\"syntax\",feature=\"unicode-perl\")",
        ),
        (
            3,
            "all(feature=\"unicode-word-boundary\",not(all(feature=\"syntax\",feature=\"unicode-perl\")))",
        ),
        (4, "not(feature=\"unicode-word-boundary\")"),
    ] {
        let helper = selected["functions"][index]["source"].as_str().unwrap();
        source.push_str(&format!(
            "\n#[cfg({guard})] mod is_word_char {{ {helper} }}\n"
        ));
        let alpha = helper.replace("UnicodeWordBoundaryError::new()", "alpha_new()");
        source.push_str(&format!(
            "\n#[cfg({guard})] mod alpha_word_char {{ {alpha} }}\n"
        ));
    }
    let constructor = selected["functions"][0]["source"]
        .as_str()
        .unwrap()
        .replace("fn new()", "fn alpha_new()");
    source.push_str(&format!(
        "\n#[cfg(not(feature=\"unicode-word-boundary\"))]\n{constructor}\n"
    ));
    source.push_str(PEERS);
    source.push_str(TRAIT_NEIGHBORS);
    source
}

const PEERS: &str = r#"
pub fn alpha_check() -> Result<(), UnicodeWordBoundaryError> { alpha_word_char::check() }
pub fn forward() -> Result<(), UnicodeWordBoundaryError> { UnicodeWordBoundaryError::check() }
pub fn alpha_forward() -> Result<(), UnicodeWordBoundaryError> { alpha_check() }
"#;

// Keep uncaptured formatter effects beside the owner; no formatter is invoked
// by any selected function or native oracle.
const TRAIT_NEIGHBORS: &str = r#"
#[cfg(feature = "std")]
impl std::error::Error for UnicodeWordBoundaryError {}
impl core::fmt::Display for UnicodeWordBoundaryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Unicode-aware \\b and \\B are unavailable because the \
             requisite data tables are missing, please enable the \
             unicode-word-boundary feature")
    }
}
"#;

fn capture(source: &str, options: &[&str]) -> (TempDir, Value) {
    let directory = clone_workspace();
    let manifest = "[package]\nname='clone_fixture'\nversion='0.1.0'\nedition='2021'\n[workspace]\n[features]\ndefault=['unicode-word-boundary','syntax','unicode-perl','std']\nunicode-word-boundary=[]\nsyntax=[]\nunicode-perl=[]\nstd=[]\n";
    std::fs::write(directory.path().join("Cargo.toml"), manifest).unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let args: Vec<_> = ["--extraction-profile", "hir-data-v3"]
        .into_iter()
        .chain(options.iter().copied())
        .collect();
    let (output, report) = clones(directory.path(), &args);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn original_result_checks_compose_in_all_three_selected_feature_contexts() {
    for options in [
        vec![],
        vec!["--no-default-features"],
        vec![
            "--no-default-features",
            "--features",
            "unicode-word-boundary",
        ],
    ] {
        let (_directory, report) = capture(&selected_source(), &options);
        let unavailable = options == ["--no-default-features"];
        check_admissions(&report, unavailable);
        check_cfg_inventory(&report, unavailable);
        for (first, second) in [
            ("UnicodeWordBoundaryError::check", "alpha_check"),
            ("is_word_char::check", "alpha_word_char::check"),
            ("forward", "alpha_forward"),
        ] {
            accepted_pair(&report, first, second);
        }
        let helper = function(&report, "is_word_char::check");
        attacks::reject_constructor_attacks(helper);
        attacks::reject_policy(function(&report, "forward"));
    }
}

fn check_admissions(report: &Value, unavailable: bool) {
    let mut names = vec![
        "UnicodeWordBoundaryError::check",
        "is_word_char::check",
        "alpha_word_char::check",
        "alpha_check",
        "forward",
        "alpha_forward",
    ];
    if unavailable {
        names.extend(["UnicodeWordBoundaryError::new", "alpha_new"]);
    }
    for name in names {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    super::word_classifiers::verify_admissions(report);
}

fn check_cfg_inventory(report: &Value, unavailable: bool) {
    let definitions = report["owned_source_inventory"][0]["source_functions"]
        .as_array()
        .unwrap();
    let helpers: Vec<_> = definitions
        .iter()
        .filter(|item| item["name"] == "check")
        .collect();
    assert_eq!(helpers.len(), 7); // Original public check + three helper branches per chain.
    assert_eq!(
        helpers
            .iter()
            .filter(|item| item["presence"] == "active")
            .count(),
        3
    );
    assert_eq!(
        helpers
            .iter()
            .filter(|item| item["presence"] == "inactive_by_cfg")
            .count(),
        4
    );
    let constructors: Vec<_> = definitions
        .iter()
        .filter(|item| item["name"] == "new" || item["name"] == "alpha_new")
        .collect();
    assert_eq!(constructors.len(), 2);
    for constructor in constructors {
        assert_eq!(
            constructor["presence"],
            if unavailable {
                "active"
            } else {
                "inactive_by_cfg"
            }
        );
    }
}

fn accepted_pair(report: &Value, first: &str, second: &str) {
    let first = &function(report, first)["key"];
    let second = &function(report, second)["key"];
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (&pair["first"] == first && &pair["second"] == second)
                || (&pair["first"] == second && &pair["second"] == first)
        })
        .expect("complete source-selected peer comparison");
    assert_eq!(pair["decision"], "Accepted", "{pair}");
}

fn variant_mut(node: &mut Value) -> Option<&mut Value> {
    if node["res"]["kind"] == RESOLUTION {
        return Some(node);
    }
    node["children"]
        .as_array_mut()?
        .iter_mut()
        .find_map(variant_mut)
}

fn reject_resealed(entry: &Value, edit: impl FnOnce(&mut Value, usize)) {
    let mut bundle = shared_evidence::wire(entry);
    UnitStore::from_bundle(&serde_json::to_vec(&bundle).unwrap()).unwrap();
    edit(&mut bundle, shared_evidence::index(entry));
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let manifest = BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    manifest.verify(&bytes).unwrap();
    assert!(UnitStore::from_bundle(&bytes).is_err());
    assert!(rstyle_pdg::verification_policy::generated(|| UnitStore::from_bundle(&bytes)).is_err());
}
