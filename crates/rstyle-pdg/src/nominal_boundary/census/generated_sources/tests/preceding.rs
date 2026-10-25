//! Expectations are about enclosing textual scope, never cleanup or body effects.
use super::*;

fn ordered(source: &str, definition: &str, cfgs: &[String]) -> Result<CheckedExpansion, String> {
    let census = census(source);
    verify(Input {
        census: &census,
        cfgs,
        namespace: &[],
        definition: &anchor(&census, definition),
        invocation: &anchor(&census, STATE),
        preceding_expansions: true,
    })
}

#[test]
fn complete_preceding_expansion_preserves_the_original_temporal_definition() {
    let source = format!("{DEFINITION}\n{PATTERN}\n{STATE}");
    let expansion = ordered(&source, DEFINITION, &[]).unwrap();
    assert_eq!(owner(&expansion), "StateID");
    let census = census(&source);
    assert!(check(&census, DEFINITION, STATE, &[]).is_err());
    let later = DEFINITION.replace("pub struct $iter;", "pub struct $iter(u8);");
    let source = format!("{DEFINITION}\n{PATTERN}\n{later}\n{STATE}");
    assert!(ordered(&source, DEFINITION, &[]).is_err());
    assert!(ordered(&source, &later, &[]).is_ok());
}

#[test]
fn generated_macro_import_module_or_unknown_attribute_never_proves_inertness() {
    for emitted in [
        "macro_rules! index_type_impls { ($x:ident) => { struct $x; }; }",
        "use unknown::name;",
        "use unknown::*;",
        "mod hidden {}",
        "#[macro_use] mod hidden {}",
        "extern crate unknown;",
        "#[unknown] struct Hidden;",
        "#[cfg_attr(feature=\"active\", unknown)] struct Hidden;",
    ] {
        let prior = format!("macro_rules! prior {{ ($x:ident) => {{ {emitted} }}; }}");
        let source = format!("{DEFINITION}\n{prior}\nprior!(Hidden);\n{STATE}");
        assert!(
            ordered(&source, DEFINITION, &["feature=\"active\"".into()]).is_err(),
            "{emitted}"
        );
    }
}

#[test]
fn earlier_unknown_is_never_healed_and_inactive_unknown_is_not_active_evidence() {
    for prior in [
        "unknown!();",
        "use unknown::*;",
        "#[unknown] struct Hidden;",
    ] {
        let source = format!("{DEFINITION}\n{prior}\n{PATTERN}\n{STATE}");
        assert!(ordered(&source, DEFINITION, &[]).is_err());
    }
    let source = format!("{DEFINITION}\n#[cfg(missing)] unknown!();\n{PATTERN}\n{STATE}");
    assert!(ordered(&source, DEFINITION, &[]).is_ok());
}

#[test]
fn body_local_and_exported_macros_and_drop_grant_no_outer_namespace_or_cleanup_proof() {
    for body in [
        "fn local() { macro_rules! index_type_impls { () => {}; } }",
        "fn local() { #[macro_export] macro_rules! exported { () => {}; } }",
    ] {
        let prior = format!("macro_rules! prior {{ ($x:ident) => {{ impl $x {{ {body} }} }}; }}");
        let source = format!("{DEFINITION}\n{prior}\nprior!(Hidden);\n{STATE}");
        assert!(ordered(&source, DEFINITION, &[]).is_ok());
    }
    let prior =
        "macro_rules! prior { ($x:ident) => { impl Drop for $x { fn drop(&mut self) {} } }; }";
    let source = format!("{DEFINITION}\n{prior}\nprior!(Hidden);\n{STATE}");
    assert!(ordered(&source, DEFINITION, &[]).is_ok());
}

#[test]
fn predecessor_work_is_bounded_without_expanding_a_supported_prefix() {
    let source = format!("{DEFINITION}\n{}\n{STATE}", PATTERN.repeat(65));
    let error = ordered(&source, DEFINITION, &[]).err().unwrap();
    assert!(error.contains("bounded invocation count"), "{error}");
}
