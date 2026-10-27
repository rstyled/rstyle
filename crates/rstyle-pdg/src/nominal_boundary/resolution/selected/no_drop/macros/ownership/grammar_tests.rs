//! Expectations independently checked with the pinned rustc, edition 2021.
use super::super::*;

fn expand_source(definition: &str, invocation: &str) -> Result<TokenStream, String> {
    let definition: syn::ItemMacro = syn::parse_str(definition).unwrap();
    let invocation: syn::ItemMacro = syn::parse_str(invocation).unwrap();
    expand(&definition, &invocation.mac, Context { edition: "2021" })
}

#[test]
fn underscore_capture_is_available_to_the_selected_transcriber() {
    let expanded = expand_source("macro_rules! m { ($_:tt) => { $_ }; }", "m!(7);").unwrap();
    let syn::Expr::Lit(value) = syn::parse2::<syn::Expr>(expanded).unwrap() else {
        panic!("underscore metavariable must retain its captured literal");
    };
    let syn::Lit::Int(value) = value.lit else {
        panic!("expected the independently specified integer");
    };
    assert_eq!(value.base10_parse::<u8>().unwrap(), 7);
}

#[test]
fn repeated_underscore_capture_preserves_all_tokens_and_empty_input() {
    let definition = "macro_rules! m { ($($_:tt)*) => { $($_)* }; }";
    assert!(expand_source(definition, "m!();").unwrap().is_empty());

    let expanded = expand_source(
        definition,
        "m!(const FIRST: u8 = 1; const SECOND: u8 = 2;);",
    )
    .unwrap();
    let file = syn::parse2::<syn::File>(expanded).unwrap();
    let names: Vec<_> = file
        .items
        .iter()
        .map(|item| match item {
            syn::Item::Const(item) => item.ident.to_string(),
            _ => panic!("repeated capture changed the original item kind"),
        })
        .collect();
    assert_eq!(names, ["FIRST", "SECOND"]);
}

#[test]
fn duplicate_underscore_matcher_bindings_remain_invalid() {
    for (definition, invocation) in [
        ("macro_rules! m { ($_:tt $_:tt) => {}; }", "m!(one two);"),
        (
            "macro_rules! m { (($_:tt) ($_:tt)) => {}; }",
            "m!((one) (two));",
        ),
    ] {
        assert!(expand_source(definition, invocation).is_err());
    }
}

#[test]
fn original_if_pub_order_and_underscore_fallback_are_preserved() {
    // Original time 0.3.47 modifier.rs matcher/transcriber shapes. The source
    // documentation is irrelevant to these independently specified outputs.
    let definition = r#"
        macro_rules! if_pub {
            (pub $(#[$attr:meta])*; $($x:tt)*) => { $(#[$attr])* $($x)* };
            ($($_:tt)*) => {};
        }
    "#;
    let expanded = expand_source(
        definition,
        "if_pub!(pub #[allow(dead_code)]; const SELECTED: u8 = 1;);",
    )
    .unwrap();
    let file = syn::parse2::<syn::File>(expanded).unwrap();
    let [syn::Item::Const(item)] = file.items.as_slice() else {
        panic!("the first original arm must retain its complete item");
    };
    assert_eq!(item.ident, "SELECTED");
    assert_eq!(item.attrs.len(), 1);

    assert!(
        expand_source(definition, "if_pub!(; impl Drop for Error {});")
            .unwrap()
            .is_empty()
    );
}
