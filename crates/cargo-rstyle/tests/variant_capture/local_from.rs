//! Local From observations advance the Range chain without authorizing trait calls.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Copy)] pub struct Span { pub start: usize, pub end: usize }
impl From<Span> for core::ops::Range<usize> {
    fn from(span: Span) -> Self { Self { start: span.start, end: span.end } }
}
pub fn range(span: Span) -> core::ops::Range<usize> { core::ops::Range::from(span) }
pub fn explicit(span: Span) -> core::ops::Range<usize> {
    <core::ops::Range<usize> as core::convert::From<Span>>::from(span)
}
pub fn primitive(value: u8) -> usize { usize::from(value) }
pub struct Generic<T>(T);
impl<T> From<Generic<T>> for core::ops::Range<usize> {
    fn from(_: Generic<T>) -> Self { 0..1 }
}
pub fn generic(value: Generic<u8>) -> core::ops::Range<usize> {
    core::ops::Range::from(value)
}
pub struct Dangerous;
impl From<u8> for Dangerous {
    fn from(_: u8) -> Self { panic!("hidden conversion effect") }
}
pub fn hidden(value: u8) -> Dangerous { Dangerous::from(value) }
mod fake {
    pub trait From<T> { fn from(value: T) -> Self; }
    impl From<u8> for usize { fn from(value: u8) -> Self { value as usize } }
    pub fn call(value: u8) -> usize { <usize as From<u8>>::from(value) }
}
"#;

#[test]
fn concrete_local_from_observes_selected_identity_without_trait_admission() {
    for no_std in [false, true] {
        let directory = clone_workspace();
        let prefix = if no_std { "#![no_std]\n" } else { "" };
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{prefix}{SOURCE}"),
        )
        .unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{}", report["capture_failures"]);
        check_selected(&report);
        for name in ["generic", "fake::call"] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
            assert!(
                resolutions(entry)
                    .iter()
                    .all(|callee| callee.implementation.is_none())
            );
        }
        let generic = resolutions(function(&report, "generic"));
        assert_eq!(generic.len(), 1);
        let hidden = function(&report, "hidden");
        assert_eq!(hidden["outcome"], "excluded", "{}", hidden["reasons"]);
        assert_eq!(
            resolutions(hidden)[0]
                .implementation
                .as_ref()
                .unwrap()
                .crate_name,
            "clone_fixture"
        );
        let primitive = function(&report, "primitive");
        assert_eq!(primitive["outcome"], "admitted", "{}", primitive["reasons"]);
        assert_eq!(
            resolutions(primitive)[0]
                .implementation
                .as_ref()
                .unwrap()
                .crate_name,
            "core"
        );
    }
}

fn check_selected(report: &Value) {
    for name in ["range", "explicit"] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "excluded", "{}", entry["reasons"]);
        let callees = resolutions(entry);
        let callee = &callees[0];
        assert_eq!(callee.crate_name, "core");
        assert!(callee.path.ends_with("::convert::From::from"));
        let selected = callee
            .implementation
            .as_ref()
            .expect("selected local From impl");
        assert_eq!(selected.crate_name, "clone_fixture");
        assert_eq!(selected.crate_hash, "local-unencoded");
        assert!(selected.implementation.is_none());
        let definitions = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|candidate| {
                candidate["facts"]["name"] == selected.path
                    && candidate["facts"]["build"]["frontend_options"]["definition_identity"]
                        == selected.definition_hash
            })
            .count();
        assert_eq!(
            definitions, 1,
            "selected implementation must bind one captured function"
        );
    }
}

fn resolutions(entry: &Value) -> Vec<rstyle_pdg::data_library::Callee> {
    let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(entry["facts"].clone()).unwrap();
    let mut nodes = Vec::new();
    descendants(&facts.body, &mut nodes);
    nodes
        .into_iter()
        .filter(|node| node.kind == "expr_call")
        .filter_map(|node| {
            let resolution = node.res.as_ref()?;
            (resolution.kind == "resolved_callee")
                .then(|| serde_json::from_str(&resolution.description).unwrap())
        })
        .collect()
}
