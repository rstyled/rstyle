//! Complete observations cannot grant unproved tagged reference operations.
use super::float_kernels::function;
use super::*;
use rstyle_pdg::concrete_types::{InterfaceObservations, Observation, TypeTree};

const SOURCE: &str = r#"
#![no_std]
mod component_range { pub struct ComponentRange { unrelated: usize } }
mod error {
    mod component_range {
        #[derive(Clone, Copy)]
        pub struct ComponentRange { name: &'static str, conditional: bool }
    }
    mod different_variant {
        #[derive(Clone, Copy)]
        pub struct DifferentVariant;
    }
    pub use component_range::ComponentRange;
    pub use different_variant::DifferentVariant;
}
use error::{ComponentRange, DifferentVariant};
#[derive(Clone, Copy)]
pub enum TryFromParsed { Empty, ComponentRange(error::ComponentRange) }
pub struct Outer { checked: TryFromParsed }
impl From<error::ComponentRange> for TryFromParsed {
    fn from(v: error::ComponentRange) -> Self { Self::ComponentRange(v) }
}
impl TryFrom<TryFromParsed> for error::ComponentRange {
    type Error = error::DifferentVariant;
    fn try_from(err: TryFromParsed) -> Result<Self, Self::Error> {
        match err {
            TryFromParsed::ComponentRange(err) => Ok(err),
            _ => Err(error::DifferentVariant),
        }
    }
}
pub fn wrap(value: ComponentRange) -> TryFromParsed { TryFromParsed::ComponentRange(value) }
pub fn unwrap(value: TryFromParsed) -> Result<ComponentRange, DifferentVariant> {
    match value { TryFromParsed::ComponentRange(payload) => Ok(payload), _ => Err(DifferentVariant) }
}
pub fn borrowed(value: &Outer) -> bool { matches!(value.checked, TryFromParsed::Empty) }
"#;

#[test]
fn tagged_string_payloads_are_captured_without_silent_reference_omission_or_admission() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["wrap", "unwrap", "borrowed"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
        let (store, index) = shared_evidence::load(entry);
        let facts = &store.functions()[index];
        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()
            .unwrap();
        interface.validate().unwrap();
        let Observation::Captured { ty: parameter } = &interface.parameters[0] else {
            panic!("Missing complete {name} input observation");
        };
        let Observation::Captured { ty: result } = &interface.result else {
            panic!("Missing complete {name} result observation");
        };
        verify_types(name, parameter, result);
    }
    verify_source_probes(&report);
}

fn verify_types(name: &str, parameter: &TypeTree, result: &TypeTree) {
    use rstyle_pdg::data_types::TypeToken;
    let tagged = if name == "wrap" { result } else { parameter };
    assert!(rstyle_pdg::data_origins::shared(&TypeToken::from_tree(
        tagged
    )));
    if name == "unwrap" {
        let TypeTree::Result { ok, err } = result else {
            panic!("Missing exact Result observation");
        };
        assert!(matches!(ok.as_ref(), TypeTree::Struct { fields, .. } if fields.len() == 2));
        let TypeTree::Struct {
            nominal,
            source,
            fields,
        } = err.as_ref()
        else {
            panic!("Error must retain its actual nominal unit struct");
        };
        assert_eq!(
            nominal.definition_path.rsplit("::").next(),
            Some("DifferentVariant")
        );
        assert!(fields.is_empty());
        let syntax: syn::ItemStruct = syn::parse_str(source).unwrap();
        assert!(matches!(syntax.fields, syn::Fields::Unit));
    }
}

fn verify_source_probes(report: &Value) {
    let entries: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.ends_with("::from") || name.ends_with("::try_from"))
        })
        .collect();
    assert_eq!(entries.len(), 2);
    for entry in entries {
        assert_eq!(entry["outcome"], "excluded");
        let (store, index) = shared_evidence::load(entry);
        let facts = &store.functions()[index];
        rstyle_pdg::nominal_boundary::resolution::probe_copy_conversion_source(facts)
            .unwrap_or_else(|error| panic!("{}: {error}", facts.name));
        let mut changed = facts.clone();
        changed.signature.is_unsafe = true;
        assert!(
            rstyle_pdg::nominal_boundary::resolution::probe_copy_conversion_source(&changed)
                .is_err()
        );
    }
}

#[test]
fn direct_local_nominals_do_not_traverse_a_parent_module_value_namesake() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        mod views {
            mod parse {
                #[derive(Clone, Copy)]
                pub struct Location { byte: u32 }
                impl Location {
                    pub fn same(self) -> Self { self }
                    pub fn duplicate(self) -> Self { self }
                }
                pub fn parse() -> u32 { 0 }
            }
            pub use self::parse::parse;
        }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "views::parse::Location::same",
        "views::parse::Location::duplicate",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        shared_evidence::load(entry);
    }
}
