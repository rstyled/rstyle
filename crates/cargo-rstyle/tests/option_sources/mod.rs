//! Source-boundary expectations precede semantic Option activation.
use super::*;

#[test]
fn scalar_option_sources_resolve_exact_modules_imports_and_nested_owners() {
    for source in [
        "pub fn value() -> Option<usize> { None }",
        "use core::option::Option as Maybe; pub fn value() -> Maybe<usize> { None }",
        "use ::core::option::{Option as Maybe}; pub fn value() -> Maybe<usize> { None }",
        "pub fn value() -> ::core::option::Option<usize> { None }",
        "fn outer() { fn value() -> Option<usize> { None } }",
        "fn outer() { use core::option::Option as Maybe; fn value() -> Maybe<usize> { None } }",
        "mod other { pub type Option = (); } use other::*; mod child { pub fn value() -> Option<usize> { None } }",
        "fn neighbor() { println!(\"body macro\"); } #[inline(always)] pub fn value() -> Option<usize> { None }",
        "#[macro_use] mod macros { macro_rules! unused { () => { type Option = (); } } } mod child { pub fn value() -> Option<usize> { None } }",
        "#[macro_use] mod macros { macro_rules! unused { () => {} } } #[derive(Clone, Copy)] pub struct Owner { n: usize } pub fn owner(n: usize) -> Owner { Owner { n } } pub fn value() -> Option<usize> { None }",
        "fn outer() { #[cfg(any())] unknown!(); fn value() -> Option<usize> { None } }",
        "fn outer() { #[cfg(any())] fn value() -> Option<usize> { None } #[cfg(all())] fn value() -> Option<usize> { None } }",
    ] {
        check_source(source, true);
    }
    check_source("#![no_std]\npub fn value() -> Option<usize> { None }", true);
}

#[test]
fn scalar_option_sources_reject_shadowing_aliases_and_lexical_macros() {
    for source in [
        "type Alias = Option<usize>; pub fn value() -> Alias { None }",
        "type Option<T> = core::option::Option<T>; pub fn value() -> Option<usize> { None }",
        "fn outer() { type Option<T> = core::option::Option<T>; fn value() -> Option<usize> { None } }",
        "type usize = u64; pub fn value() -> Option<usize> { None }",
        "mod types { pub type Option<T> = core::option::Option<T>; } use types::*; pub fn value() -> Option<usize> { None }",
        "mod core { pub mod option { pub use ::core::option::Option; } } pub fn value() -> core::option::Option<usize> { None }",
        "mod core { pub mod option { pub use ::core::option::Option; } } use crate::core::option::Option as Maybe; pub fn value() -> Maybe<usize> { None }",
        "macro_rules! types { () => { type Extra = (); } } types!(); pub fn value() -> Option<usize> { None }",
        "fn outer() { macro_rules! types { () => { type Extra = (); } } types!(); fn value() -> Option<usize> { None } }",
    ] {
        check_source(source, false);
    }
}

fn check_source(source: &str, expected: bool) {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{source}: {report}");
    for function in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["facts"]["name"] == "owner")
    {
        assert_eq!(
            function["outcome"], "admitted",
            "closed local macro imports now have a scoped nominal proof: {function}"
        );
    }
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["facts"]["name"]
                .as_str()
                .unwrap()
                .split("::")
                .last()
                == Some("value")
        })
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let parsed: syn::ItemFn = syn::parse_str(&facts.source.bytes).unwrap();
    let syn::ReturnType::Type(_, ty) = parsed.sig.output else {
        panic!("missing result");
    };
    let syn::Type::Path(path) = *ty else {
        panic!("missing path");
    };
    let result = rstyle_pdg::unit_store::EvidenceFacts::verify_option_type(&facts, &path, "usize");
    assert_eq!(result.is_ok(), expected, "{source}: {result:?}");
    if expected {
        reject_owner_edits(&facts, &path, source);
    }
}

fn reject_owner_edits(facts: &rstyle_pdg::HirFnFacts, path: &syn::TypePath, source: &str) {
    let mut changed = facts.clone();
    changed.fn_span.end -= 1;
    assert!(
        rstyle_pdg::unit_store::EvidenceFacts::verify_option_type(&changed, path, "usize").is_err()
    );
    let mut changed = facts.clone();
    changed.source.bytes = "pub fn value() -> Option<usize> { Some(99) }".into();
    assert!(
        rstyle_pdg::unit_store::EvidenceFacts::verify_option_type(&changed, path, "usize").is_err()
    );
    let start = source.find(facts.source.bytes.as_str()).unwrap() as u32;
    if start != facts.fn_span.start {
        let mut inactive = facts.clone();
        inactive.fn_span = rstyle_pdg::ByteSpan {
            start,
            end: start + facts.source.bytes.len() as u32,
        };
        assert!(
            rstyle_pdg::unit_store::EvidenceFacts::verify_option_type(&inactive, path, "usize")
                .is_err()
        );
    }
    reject_external_roots(facts, path);
}

fn reject_external_roots(facts: &rstyle_pdg::HirFnFacts, path: &syn::TypePath) {
    for extra in [
        vec!["--extern", "core=/dependency/libcore.rlib"],
        vec!["--extern=std=/dependency/libstd.rlib"],
        vec!["--extern", "priv:core=/dependency/libcore.rlib"],
        vec!["@hidden-arguments"],
    ] {
        let mut changed = facts.clone();
        let mut invocation = changed
            .build
            .frontend_options
            .get("invocation")
            .unwrap()
            .clone();
        invocation
            .as_array_mut()
            .unwrap()
            .extend(extra.into_iter().map(Value::from));
        changed
            .build
            .frontend_options
            .insert("invocation".into(), invocation);
        assert!(
            rstyle_pdg::unit_store::EvidenceFacts::verify_option_type(&changed, path, "usize")
                .is_err()
        );
    }
}
