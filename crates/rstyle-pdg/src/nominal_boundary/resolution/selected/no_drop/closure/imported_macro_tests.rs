//! Production cleanup entry point with complete original-source ownership.
//! These source witnesses establish cleanup absence, never value admission.
use super::*;
use crate::nominal_boundary::resolution::{CONTEXT_KEY, environment};
use std::sync::Arc;

const WITNESS: &str = "fn witness() {}";
const ORIGINAL: &str = include_str!("../macros/ownership/fixtures/internal_macros.rs.txt");

fn source(imports: &str, body: &str) -> String {
    format!(
        "mod internal_macros {{\n{ORIGINAL}\n}}\n\
         struct Error(());\n{imports}\n{WITNESS}\n\
         fn use_macro(value: i32) {{ {body} }}"
    )
}

fn fixture(source: &str) -> (Environment, HirFnFacts) {
    let mut census = (*environment::tests::census()).clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = crate::source_identity::checksum(source.as_bytes());
    let mut facts: HirFnFacts =
        serde_json::from_str(include_str!("../../function_cleanup_tests/facts.json")).unwrap();
    facts.build.frontend_options.insert(
        CONTEXT_KEY.into(),
        serde_json::to_value(&census.context).unwrap(),
    );
    facts.build.frontend_options.insert(
        "source_file".into(),
        serde_json::to_value(&census.files[0].path).unwrap(),
    );
    let start = source.find(WITNESS).unwrap() as u32;
    facts.fn_span = ByteSpan {
        start,
        end: start + WITNESS.len() as u32,
    };
    facts.source.bytes = WITNESS.into();
    facts.source.digest = crate::source_identity::checksum(WITNESS.as_bytes());
    let environment = Environment::with_options(Arc::new(census), true, Some(&[]), false).unwrap();
    assert!(supports(&facts, OWNERSHIP_MACROS_CAPABILITY).unwrap());
    (environment, facts)
}

fn verify(source: &str) -> Result<(), String> {
    let (environment, facts) = fixture(source);
    super::verify_reference_product(&environment, &facts, &vec!["Error".into()])
}

#[test]
fn imported_original_macro_is_checked_through_the_production_source_closure() {
    verify(&source(
        "use crate::internal_macros::div_floor;",
        "let _ = div_floor!(value, 3);",
    ))
    .unwrap();
}

#[test]
fn original_macro_reexport_aliases_preserve_definition_ownership() {
    verify(&source(
        "mod exports { pub(crate) use crate::internal_macros::div_floor as floor; }\n\
         use crate::exports::floor as rounded;",
        "let _ = rounded!(value, 3);",
    ))
    .unwrap();
}

#[test]
fn imported_original_expression_capture_cannot_hide_drop() {
    assert!(
        verify(&source(
            "use crate::internal_macros::div_floor;",
            "let _ = div_floor!({ impl Drop for Error { fn drop(&mut self) {} } value }, 3);",
        ))
        .is_err()
    );
}

#[test]
fn block_local_import_alias_does_not_reuse_the_module_macro_certificate() {
    assert!(
        verify(&source(
            "use crate::internal_macros::div_floor;",
            "use external::hidden as div_floor; let _ = div_floor!(value, 3);",
        ))
        .is_err()
    );
}

#[test]
fn block_local_glob_does_not_reuse_the_module_macro_certificate() {
    assert!(
        verify(&source(
            "use crate::internal_macros::div_floor;",
            "use external::*; let _ = div_floor!(value, 3);",
        ))
        .is_err()
    );
}

#[test]
fn unsealed_or_missing_census_source_cannot_supply_macro_ownership() {
    let text = source(
        "use crate::internal_macros::div_floor;",
        "let _ = div_floor!(value, 3);",
    );
    let (environment, _) = fixture(&text);
    let mut changed = environment.census().clone();
    changed.files[0].bytes = text.replace("this / rhs", "this + rhs").into();
    assert!(Environment::with_options(Arc::new(changed), true, Some(&[]), false).is_err());

    let mut missing = environment.census().clone();
    missing.files.clear();
    assert!(Environment::with_options(Arc::new(missing), true, Some(&[]), false).is_err());
}

#[test]
fn wrong_or_missing_original_reexport_cannot_supply_the_invoked_macro() {
    for imports in [
        "mod exports { pub(crate) use crate::internal_macros::missing as floor; }\n\
         use crate::exports::floor as rounded;",
        "mod exports {} use crate::exports::floor as rounded;",
    ] {
        assert!(verify(&source(imports, "let _ = rounded!(value, 3);")).is_err());
    }
}

#[test]
fn resealed_original_macro_definition_still_exposes_its_destructor() {
    let original = source(
        "use crate::internal_macros::div_floor;",
        "let _ = div_floor!(value, 3);",
    );
    verify(&original).unwrap();

    let changed = original.replace(
        "let d = this / rhs;",
        "impl Drop for crate::Error { fn drop(&mut self) {} } let d = this / rhs;",
    );
    assert_ne!(original, changed);
    // fixture() reseals the whole original census with XXH3. Byte consistency
    // cannot stand in for inspecting the newly selected macro output.
    assert!(verify(&changed).is_err());
}
