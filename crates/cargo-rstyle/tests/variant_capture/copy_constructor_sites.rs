//! Constructor-site authority cannot certify neighboring functions or impl scopes.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Copy)]
pub struct Named { name: &'static str, conditional: bool }
impl Named {
    pub fn new(name: &'static str) -> Self { Self { name, conditional: false } }
    pub fn same(name: &'static str) -> Self { Self { name, conditional: false } }
    pub fn conditional(name: &'static str) -> Self { Self { name, conditional: true } }
}
pub fn caller(name: &'static str) -> Named { Named::new(name) }
macro_rules! unknown { () => { const HIDDEN: u8 = 1; }; }
unknown!();
pub struct Owned;
impl Drop for Owned { fn drop(&mut self) { core::hint::black_box(1); } }
pub fn sibling() -> u8 { let _owned = Owned; core::hint::black_box(7) }
"#;

#[test]
fn exact_copy_constructors_preserve_unknown_neighbors_and_document_policy() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["Named::new", "Named::same", "Named::conditional"] {
        assert_eq!(function(&report, name)["outcome"], "admitted", "{name}");
    }
    for name in ["caller", "sibling"] {
        assert_eq!(function(&report, name)["outcome"], "excluded", "{name}");
    }
    check_equivalent_matching(&report, ["Named::new", "Named::same"]);
    verify_policy_and_origins(&report);
    verify_constructor_evidence(&report);
}

fn verify_constructor_evidence(report: &Value) {
    let entry = function(report, "Named::new");
    let (store, index) = shared_evidence::load(entry);
    for attack in ["copy", "trait", "owner", "flag", "local", "extra"] {
        let mut changed = store.functions()[index].clone();
        if ["copy", "trait", "owner"].contains(&attack) {
            let mut origins = serde_json::to_value(
                changed
                    .build
                    .frontend_options
                    .data::<rstyle_pdg::declaration_origins::Origins>()
                    .unwrap()
                    .as_ref(),
            )
            .unwrap();
            match attack {
                "copy" => origins["declarations"][0]["builtin_copy"] = Value::Null,
                "trait" => {
                    origins["declarations"][0]["builtin_copy"]["trait_definition"]["path"] =
                        "local::Copy".into()
                }
                _ => origins["declarations"][0]["nominal"]["definition_path"] = "Other".into(),
            }
            changed
                .build
                .frontend_options
                .insert(rstyle_pdg::declaration_origins::KEY.into(), origins);
        } else {
            let literal = &mut changed.body.children[0];
            match attack {
                "flag" => literal.children[1].lit = Some("true".into()),
                "local" => {
                    literal.children[0]
                        .res
                        .as_mut()
                        .unwrap()
                        .target
                        .as_mut()
                        .unwrap()
                        .local += 1
                }
                _ => literal.children.push(literal.children[1].clone()),
            }
        }
        shared_evidence::reject_facts(entry, &changed);
    }
}

#[test]
fn out_of_module_copy_constructor_cannot_supply_global_namespace_permission() {
    let directory = clone_workspace();
    let source = format!(
        "{SOURCE}\npub mod factory {{ impl crate::Named {{ pub fn moved(name: &'static str) -> Self {{ Self {{ name, conditional: false }} }} }} }}"
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let entry = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.ends_with("::moved"))
        })
        .unwrap();
    assert_eq!(entry["outcome"], "excluded", "{}", entry["reasons"]);
}

fn verify_policy_and_origins(report: &Value) {
    let store = super::word_classifiers::verify_admissions(report);
    let analysis = store.analysis();
    let entry = function(report, "Named::new");
    let index = shared_evidence::index(entry);
    let facts = analysis.function(index).unwrap();
    let pair = store.pair(index).unwrap();
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
    })
    .unwrap();
    let mut historical = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    historical.pdg.profile.version = "3.101.0".into();
    historical.cfg.profile.version = "3.101.0".into();
    assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&historical, &facts).is_err());
    let returned = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
        .unwrap();
    let origins: Vec<rstyle_pdg::data_origins::Origin> =
        serde_json::from_value(returned.annotations[rstyle_pdg::data_origins::KEY].clone())
            .unwrap();
    assert_eq!(
        origins,
        vec![rstyle_pdg::data_origins::Origin {
            parameter: 0,
            path: vec![rstyle_pdg::field_locations::Projection::SharedReferent],
        }]
    );
}
