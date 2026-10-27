//! Supplied string references retain their exact aggregate component origins.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
#![no_std]
#[derive(Clone, Copy)]
pub struct Named { name: &'static str, conditional: bool }
#[derive(Clone, Copy)]
pub struct Count(u8);
impl Named {
    pub fn new(name: &'static str) -> Self { Self { name, conditional: false } }
    pub fn same(name: &'static str) -> Self { Self { name, conditional: false } }
    pub fn conditional(name: &'static str) -> Self { Self { name, conditional: true } }
}
pub fn caller(name: &'static str) -> Named { Named::new(name) }
pub fn paired(name: &'static str, count: u8) -> (Named, Count) { (Named::new(name), Count(count)) }
pub fn saved(name: &'static str, other: &'static str) -> Named {
    let mut selected = name;
    let result = Named::new(selected);
    selected = other;
    result
}
"#;

const ORACLE: &str = r#"
#[test]
fn exact_backing_length_and_flag_survive_construction() {
    static LEFT: [u8; 3] = *b"abc";
    static RIGHT: [u8; 3] = *b"abc";
    let a = core::str::from_utf8(&LEFT).unwrap();
    let b = core::str::from_utf8(&RIGHT).unwrap();
    assert_ne!(a.as_ptr(), b.as_ptr());
    assert!(!core::mem::needs_drop::<Named>());
    for name in ["", "λ", a, b] {
        for value in [Named::new(name), Named::same(name), caller(name), saved(name, b)] {
            assert_eq!(value.name.as_ptr(), name.as_ptr());
            assert_eq!(value.name.len(), name.len());
            assert!(!value.conditional);
        }
        let value = Named::conditional(name);
        assert_eq!(value.name.as_ptr(), name.as_ptr());
        assert_eq!(value.name.len(), name.len());
        assert!(value.conditional);
        let (value, count) = paired(name, 7);
        assert_eq!(count.0, 7);
        assert_eq!(value.name.as_ptr(), name.as_ptr());
    }
}
"#;

#[test]
fn static_string_copy_products_preserve_supplied_reference_components() {
    let directory = clone_workspace();
    native_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "Named::new",
        "Named::same",
        "Named::conditional",
        "caller",
        "saved",
        "paired",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    verify_documents(&report);
    check_equivalent_matching(&report, ["Named::new", "Named::same"]);
}

#[test]
fn static_string_product_activation_keeps_unknown_ownership_and_history_closed() {
    let directory = clone_workspace();
    for (source, profile, constructor) in [
        (
            SOURCE.replace("#[derive(Clone, Copy)]", ""),
            "hir-data-v3",
            false,
        ),
        (
            format!(
                "{SOURCE}\nmacro_rules! neighbor {{ () => {{ const N:u8=0; }}; ($x:ident) => {{ struct $x; }}; }} neighbor!();"
            ),
            "hir-data-v3",
            true,
        ),
        (SOURCE.to_owned(), "hir-data-v2", false),
    ] {
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for name in ["Named::new", "caller", "saved", "paired"] {
            let entry = function(&report, name);
            assert_eq!(
                entry["outcome"],
                if constructor && name == "Named::new" {
                    "admitted"
                } else {
                    "excluded"
                },
                "{profile}: {name}: {}",
                entry["reasons"]
            );
        }
    }
}

#[test]
fn borrowed_string_owner_history_does_not_acquire_static_product_requirements() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub struct View<'a> { name: &'a str }
        pub fn length(view: &View<'_>) -> usize { view.name.len() }
    "#,
    )
    .unwrap();
    for profile in ["hir-data-v1", "hir-data-v3"] {
        let (output, report) = clones(directory.path(), &["--extraction-profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(function(&report, "length")["outcome"], "admitted");
        super::word_classifiers::verify_admissions(&report);
    }
}

fn native_oracle(directory: &Path) {
    let source = directory.join("oracle.rs");
    let binary = directory.join("oracle");
    std::fs::write(&source, format!("{SOURCE}\n{ORACLE}")).unwrap();
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

fn verify_documents(report: &Value) {
    let store = super::word_classifiers::verify_admissions(report);
    let analysis = store.analysis();
    for entry in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["outcome"] == "admitted")
    {
        let index = shared_evidence::index(entry);
        let facts = analysis.function(index).unwrap();
        let pair = store.pair(index).unwrap();
        rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
        })
        .unwrap();
        verify_saved_origin(pair, &facts);
        let mut historical = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        historical.pdg.profile.version = "3.100.0".into();
        historical.cfg.profile.version = "3.100.0".into();
        assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
        assert!(rstyle_ccgraph::admit_produced(&historical, &facts).is_err());
    }
}

fn verify_saved_origin(
    pair: &rstyle_pdg::producer::Pair,
    facts: &impl rstyle_pdg::unit_store::EvidenceFacts,
) {
    let returned = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
        .unwrap();
    let roots = &returned.annotations[rstyle_pdg::data_origins::KEY];
    // Use typed paths to fix an expectation independently of wire spelling.
    let origins: Vec<rstyle_pdg::data_origins::Origin> =
        serde_json::from_value(roots.clone()).unwrap();
    assert_eq!(
        origins,
        vec![rstyle_pdg::data_origins::Origin {
            parameter: 0,
            path: vec![rstyle_pdg::field_locations::Projection::SharedReferent],
        }]
    );
    let mut forged = rstyle_pdg::producer::Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    let returned = forged
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
        .unwrap();
    let mut wrong = roots.clone();
    wrong[0]["parameter"] = 1.into();
    returned
        .annotations
        .insert(rstyle_pdg::data_origins::KEY.into(), wrong);
    assert!(rstyle_pdg::data_documents::verify(&forged, facts).is_err());
}
