use super::*;
use crate::nominal_boundary::resolution::environment;
use std::sync::Arc;

fn environment(source: &str, cfgs: &[String]) -> Result<Environment, String> {
    let mut census = (*environment::tests::census()).clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = crate::source_identity::checksum(source.as_bytes());
    Environment::with_options(Arc::new(census), true, Some(cfgs), false)
}

fn probe(source: &str, namespace: &[&str], enabled: bool, cfgs: &[String]) -> Result<(), String> {
    let environment = environment(source, cfgs)?;
    let enums = enum_globs::Scope::new(&environment.modules, cfgs, true)?;
    let macros = SourceMacros::new(&environment, cfgs, enabled.then_some(&enums))?;
    let invocation = syn::parse_str::<syn::ExprMacro>("write!(sink, \"x\")").unwrap();
    let namespace = namespace
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    assert!(macros.resolve(&namespace, &invocation.mac)?.is_none());
    Ok(())
}

#[test]
fn exact_original_enum_glob_preserves_builtin_fallback_only_in_new_profile() {
    let source = "enum Month { January, February } use self::Month::*;";
    probe(source, &[], true, &[]).unwrap();
    assert!(probe(source, &[], false, &[]).is_err());
    probe(
        "mod month { enum Month { January, February } use self::Month::*; }",
        &["month"],
        true,
        &[],
    )
    .unwrap();
}

#[test]
fn mixed_unknown_module_alias_and_duplicate_declarations_keep_glob_barrier() {
    for extra in [
        "use unknown::*;",
        "use self::{Month::*, unknown::*};",
        "use external::Other as Month;",
        "type Month = ();",
        "struct Month;",
        "enum Month { Other }",
        "mod Month {}",
    ] {
        let source = format!("enum Month {{ January }} use self::Month::*; {extra}");
        assert!(probe(&source, &[], true, &[]).is_err(), "{source}");
    }
    for source in [
        "mod Month {} use self::Month::*;",
        "enum Month { January } use Month::*;",
        "enum Month { January } use crate::Month::*;",
        "mod other { enum Month { January } } use self::Month::*;",
    ] {
        assert!(probe(source, &[], true, &[]).is_err(), "{source}");
    }
}

#[test]
fn original_cfg_selection_and_missing_roster_remain_distinct() {
    let inactive = "#[cfg(feature = \"month\")] enum Month { January } use self::Month::*;";
    assert!(probe(inactive, &[], true, &[]).is_err());
    probe(inactive, &[], true, &["feature=\"month\"".into()]).unwrap();
    probe(
        "enum Month { January } use self::Month::*; #[cfg(any())] use unknown::*;",
        &[],
        true,
        &[],
    )
    .unwrap();
    let environment = environment("enum Month { January } use self::Month::*;", &[]).unwrap();
    let enums = enum_globs::Scope::default();
    let macros = SourceMacros::new(&environment, &[], Some(&enums)).unwrap();
    let invocation = syn::parse_str::<syn::ExprMacro>("write!(sink, \"x\")").unwrap();
    assert!(macros.resolve(&[], &invocation.mac).is_err());
}

#[test]
fn macro_module_globs_never_erase_a_hidden_destructor() {
    let source = "mod hidden { macro_rules! cleanup { () => { impl Drop for Error { fn drop(&mut self) {} } }; } pub(crate) use cleanup; } use self::hidden::*;";
    assert!(probe(source, &[], true, &[]).is_err());
    let original = "struct Error(()); enum Month { January } use self::Month::*; fn hidden() { impl Drop for Error { fn drop(&mut self) {} } }";
    assert!(super::super::closure::inspect_external_source(original, true).is_err());
}
