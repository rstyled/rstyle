//! Complete witness tests; source resealing never supplies Copy or cleanup authority.
use super::super::product_fields::test_support::{facts, origins, resolution, sources};
use super::tests::{INPUT, fixture};
use super::*;
use crate::nominal_boundary::resolution::environment;
use std::{collections::BTreeMap, sync::Arc};

fn check(extra: &str, change: impl FnOnce(&mut Origins)) -> Result<CheckedNoDrop, String> {
    let mut tree = fixture();
    let mut declarations = BTreeMap::new();
    sources(&tree, &mut declarations);
    let source = format!(
        "{}\n{extra}",
        declarations.into_values().collect::<Vec<_>>().join("\n")
    );
    let resolution = resolution(&source, &mut tree);
    let facts = facts(&tree);
    let mut origins = origins(&resolution, &facts);
    change(&mut origins);
    verify(&facts, &tree, &resolution, &origins)
}

#[test]
fn full_source_copy_and_cleanup_proofs_construct_the_noncopy_product_witness() {
    let checked = check("", |_| {}).unwrap();
    assert_eq!(checked.nominal().definition_path, "Input");
    assert!(INPUT.contains("derive(Clone)"));
    assert!(!INPUT.contains("Copy"));
}

#[test]
fn resealed_original_source_still_rejects_drop_and_generated_cleanup() {
    for extra in [
        "impl Drop for Input<'_> { fn drop(&mut self) {} }",
        "impl Drop for PatternID { fn drop(&mut self) {} }",
        "fn hidden() { impl Drop for SmallIndex { fn drop(&mut self) {} } }",
        "macro_rules! hidden { ($owner:ident) => { impl Drop for $owner { fn drop(&mut self) {} } }; } hidden!(Span);",
        "use external::Cleanup; impl Cleanup for Input<'_> {}",
    ] {
        let error = check(extra, |_| {}).unwrap_err();
        assert!(error.contains("possible destructor"), "{extra}: {error}");
    }
}

#[test]
fn whole_witness_rejects_missing_and_forged_copy_authority_for_every_dependency() {
    for name in ["Span", "Anchored", "PatternID", "SmallIndex"] {
        let missing = check("", |origins| {
            origins
                .declarations
                .iter_mut()
                .find(|origin| origin.nominal.definition_path == name)
                .unwrap()
                .builtin_copy = None;
        })
        .unwrap_err();
        assert!(missing.contains("Copy"), "{name}: {missing}");
        let forged = check("", |origins| {
            origins
                .declarations
                .iter_mut()
                .find(|origin| origin.nominal.definition_path == name)
                .unwrap()
                .builtin_copy
                .as_mut()
                .unwrap()
                .trait_definition
                .crate_hash = "forged".into();
        })
        .unwrap_err();
        assert!(
            forged.contains("Copy observation disagrees with pinned trait/derive authority"),
            "{name}: {forged}"
        );
    }
}

mod reader;
