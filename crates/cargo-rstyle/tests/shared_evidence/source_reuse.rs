use crate::*;
use rstyle_pdg::unit_store::{EvidenceFacts, SourceAnalyses};

#[test]
fn capture_source_proofs_keep_exact_owners_modes_and_verification_policies() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "#[derive(Clone, Copy)] pub struct Value { bits: u32 }\n\
         impl Value { pub fn bits(&self) -> u32 { self.bits }\n\
         pub fn again(&self) -> u32 { self.bits() } }",
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "Value::again")
        .unwrap();
    assert_eq!(function["outcome"], "admitted", "{}", function["reasons"]);
    let raw: rstyle_pdg::HirFnFacts = serde_json::from_value(function["facts"].clone()).unwrap();
    check_fast_source_claims(function);
    check_capture_scopes(&raw);
    let mut changed = raw.clone();
    changed.source.bytes = "pub fn again(&self) -> u32 {".into();
    assert!(
        SourceAnalyses::default()
            .function(&changed)
            .receiver_inventory(true)
            .is_err()
    );
    let inventory = rstyle_pdg::source_evidence::capture(&raw).unwrap();
    assert_eq!(
        rstyle_pdg::source_evidence::diagnostics(
            &SourceAnalyses::default().function(&raw),
            &inventory
        ),
        rstyle_pdg::source_evidence::diagnostics(&raw, &inventory)
    );
}

fn check_capture_scopes(raw: &rstyle_pdg::HirFnFacts) {
    let sources = SourceAnalyses::default();
    let source = sources.function(raw);
    let nominal = source.nominal_resolution().unwrap();
    assert!(std::ptr::eq(
        &*nominal,
        &*source.nominal_resolution().unwrap()
    ));
    let receivers = source.receiver_inventory(true).unwrap();
    assert!(std::ptr::eq(
        &*receivers,
        &*source.receiver_inventory(true).unwrap()
    ));
    assert!(!std::ptr::eq(
        &*receivers,
        &*source.receiver_inventory(false).unwrap()
    ));
    rstyle_pdg::verification_policy::generated(|| {
        assert!(!std::ptr::eq(
            &*nominal,
            &*source.nominal_resolution().unwrap()
        ));
        assert!(!std::ptr::eq(
            &*receivers,
            &*source.receiver_inventory(true).unwrap()
        ));
    });
    let independent = sources.function(raw);
    assert!(!std::ptr::eq(
        &*nominal,
        &*independent.nominal_resolution().unwrap()
    ));
}

fn check_fast_source_claims(function: &serde_json::Value) {
    let facts = &function["facts"];
    let options = &facts["build"]["frontend_options"];
    let package = &options["rstyle.nominal_selected_unit"]["package"];
    let origins = options["concrete_declaration_origins"]["declarations"]
        .as_array()
        .unwrap();
    assert!(!origins.is_empty());
    let files = options["rstyle.nominal_source_census"]["files"]
        .as_array()
        .unwrap();
    let claims = std::iter::once(&facts["source"]["digest"])
        .chain(std::iter::once(&package["manifest_digest"]))
        .chain(files.iter().map(|file| &file["digest"]))
        .chain(origins.iter().flat_map(|origin| {
            [
                &origin["file_digest"],
                &origin["nominal"]["manifest_digest"],
                &origin["nominal"]["declaration_digest"],
            ]
        }));
    for claim in claims {
        assert_eq!(
            rstyle_pdg::source_identity::Scheme::parse(claim.as_str().unwrap()),
            Some(rstyle_pdg::source_identity::Scheme::Xxh3)
        );
    }
    let (store, index) = super::load(function);
    let analysis = store.analysis();
    let view = analysis.function(index).unwrap();
    rstyle_pdg::producer::verify(super::pair(&store, function), &view).unwrap();
}
