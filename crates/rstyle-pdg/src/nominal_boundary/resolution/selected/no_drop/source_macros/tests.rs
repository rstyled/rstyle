use super::*;

fn sources(text: &str, cfgs: &[String]) -> Result<SourceMacros, String> {
    let root = syn::parse_file(text).unwrap();
    let mut modules = Vec::new();
    collect_modules(&root.items, &[], &mut modules);
    let configuration = crate::conditional_source::Configuration::new(cfgs);
    let mut result = SourceMacros::from_modules(&modules, &configuration)?;
    result.index_imports();
    Ok(result)
}

fn collect_modules(items: &[syn::Item], namespace: &[String], result: &mut Vec<census::Module>) {
    result.push(census::Module {
        namespace: namespace.to_vec(),
        path: "/sample/src/lib.rs".into(),
        items: items.to_vec(),
        verified_macro_imports: Default::default(),
        verified_external_imports: Default::default(),
    });
    for item in items {
        if let syn::Item::Mod(module) = item
            && let Some((_, children)) = &module.content
        {
            collect_modules(
                children,
                &joined(namespace, &module.ident.to_string()),
                result,
            );
        }
    }
}

fn selected(
    source: &SourceMacros,
    namespace: &[&str],
    invocation: &str,
) -> Result<Option<String>, String> {
    let invocation: syn::ExprMacro = syn::parse_str(invocation).unwrap();
    source
        .resolve(
            &namespace
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>(),
            &invocation.mac,
        )
        .map(|item| item.map(|item| item.mac.tokens.to_string()))
}

#[test]
fn ordinary_grouped_imports_aliases_and_explicit_paths_keep_the_original_definition() {
    let source = sources(
        r#"
        mod definitions {
            macro_rules! original { ($v:ident) => { consume($v) }; }
            pub(crate) use original;
        }
        mod client { use crate::definitions::{original as renamed}; }
    "#,
        &[],
    )
    .unwrap();
    let renamed = selected(&source, &["client"], "renamed!(value)")
        .unwrap()
        .unwrap();
    assert!(renamed.contains("consume"));
    assert_eq!(
        Some(renamed),
        selected(&source, &["client"], "crate::definitions::original!(value)").unwrap()
    );
}

#[test]
fn ordinary_reexport_chains_and_module_aliases_preserve_the_selected_target() {
    let source = sources(
        r#"
        mod definitions {
            macro_rules! first { () => { first_body() }; }
            macro_rules! second { () => { second_body() }; }
            pub(crate) use {first, second};
        }
        mod bridge { pub(crate) use crate::definitions::second as renamed; }
        mod client { use crate::bridge as exports; use crate::definitions::first as direct; }
    "#,
        &[],
    )
    .unwrap();
    assert!(
        selected(&source, &["client"], "exports::renamed!()")
            .unwrap()
            .unwrap()
            .contains("second_body")
    );
    assert!(
        selected(&source, &["client"], "direct!()")
            .unwrap()
            .unwrap()
            .contains("first_body")
    );
}

#[test]
fn cfg_selects_one_original_definition_and_duplicate_active_bindings_reject() {
    let text = r#"
        mod definitions {
            macro_rules! first { () => { first_body() }; }
            macro_rules! second { () => { second_body() }; }
            pub(crate) use {first, second};
        }
        mod client {
            #[cfg(feature = "alternate")] use crate::definitions::second as chosen;
            #[cfg(not(feature = "alternate"))] use crate::definitions::first as chosen;
        }
    "#;
    let first = sources(text, &[]).unwrap();
    let second = sources(text, &["feature=\"alternate\"".into()]).unwrap();
    assert!(
        selected(&first, &["client"], "chosen!()")
            .unwrap()
            .unwrap()
            .contains("first_body")
    );
    assert!(
        selected(&second, &["client"], "chosen!()")
            .unwrap()
            .unwrap()
            .contains("second_body")
    );
    let ambiguous = sources(
        &text
            .replace("#[cfg(feature = \"alternate\")]", "")
            .replace("#[cfg(not(feature = \"alternate\"))]", ""),
        &[],
    )
    .unwrap();
    assert!(selected(&ambiguous, &["client"], "chosen!()").is_err());
}

#[test]
fn cycles_unknown_reexports_and_external_roots_remain_errors() {
    for text in [
        "mod a { pub(crate) use crate::b::m; } mod b { pub(crate) use crate::a::m; }",
        "mod a { pub(crate) use crate::missing::m; }",
        "mod a { pub(crate) use external::m; }",
    ] {
        let source = sources(text, &[]).unwrap();
        assert!(selected(&source, &["a"], "m!()").is_err());
    }
}

#[test]
fn missing_unclaimed_binding_and_textual_local_definition_stay_with_the_caller() {
    let source = sources("mod a { macro_rules! local { () => {} } }", &[]).unwrap();
    assert!(selected(&source, &["a"], "missing!()").unwrap().is_none());
    assert!(selected(&source, &["a"], "local!()").unwrap().is_none());
}

#[test]
fn private_reexports_globs_and_type_shadows_cannot_supply_macro_authority() {
    for text in [
        "mod defs { macro_rules! m {()=>{}} use m; } mod client { use crate::defs::m; }",
        "mod defs { pub(crate) use unknown::*; } mod client { use crate::defs::m; }",
        "mod defs { pub type m = (); } mod client { use crate::defs::m; }",
    ] {
        let source = sources(text, &[]).unwrap();
        assert!(selected(&source, &["client"], "m!()").is_err());
    }
}

#[test]
fn active_macro_attribute_substitution_is_rejected_after_cfg_selection() {
    let text = r#"
        mod defs {
            #[cfg_attr(feature = "poison", transform)]
            macro_rules! m { () => {} }
            pub(crate) use m;
        }
        mod client { use crate::defs::m; }
    "#;
    assert!(
        selected(&sources(text, &[]).unwrap(), &["client"], "m!()")
            .unwrap()
            .is_some()
    );
    let poisoned = sources(text, &["feature=\"poison\"".into()]).unwrap();
    assert!(selected(&poisoned, &["client"], "m!()").is_err());
}
