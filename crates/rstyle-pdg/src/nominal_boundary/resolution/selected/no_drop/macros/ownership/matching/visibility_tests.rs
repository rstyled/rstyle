use super::*;

const CONSTANTS: &[&str] = &[
    "DEFAULT",
    "PARSING",
    "DATE",
    "TIME",
    "OFFSET",
    "DATE_TIME",
    "DATE_TIME_OFFSET",
    "TIME_OFFSET",
];

#[test]
fn original_iso8601_family_retains_every_encoded_and_associated_constant() {
    let file = syn::parse_file(include_str!("iso8601.rs.txt")).unwrap();
    let [syn::Item::Macro(definition), syn::Item::Macro(invocation)] = file.items.as_slice() else {
        panic!("complete original macro and invocation");
    };
    for edition in ["2021", "2024"] {
        let tokens = expand(definition, &invocation.mac, Context { edition }).unwrap();
        let output = syn::parse2::<syn::File>(tokens).unwrap();
        assert_eq!(output.items.len(), CONSTANTS.len() * 2);
        for (items, name) in output.items.as_chunks::<2>().0.iter().zip(CONSTANTS) {
            check_constants(items, name);
        }
    }
}

fn check_constants(items: &[syn::Item], name: &str) {
    let [syn::Item::Const(encoded), syn::Item::Impl(owner)] = items else {
        panic!("one encoding and implementation");
    };
    assert_eq!(encoded.ident, name);
    let syn::Expr::MethodCall(call) = encoded.expr.as_ref() else {
        panic!("original encoding operation");
    };
    assert!(call.method == "encode" && call.args.is_empty());
    let [syn::ImplItem::Const(associated)] = owner.items.as_slice() else {
        panic!("one associated constant");
    };
    assert_eq!(associated.ident, name);
    assert!(matches!(associated.vis, syn::Visibility::Public(_)));
    assert!(matches!(&associated.expr, syn::Expr::Path(path) if path.path.is_ident("Self")));
}

#[test]
fn independently_compiled_visibility_and_attribute_combinations_preserve_values() {
    let definition = "macro_rules! m { ($($(#[$doc:meta])* $vis:vis const $name:ident = $value:expr;)*) => {$($(#[$doc])* $vis const $name: u8 = $value;)*}; }";
    let invocation = "m! { #[doc = \"public\"] pub const A = 1; #[doc = \"inherited\"] const B = 2; #[doc = \"restricted\"] pub(crate) const C = 3; const D = 4; }";
    let output = run(definition, invocation).unwrap();
    let file = syn::parse2::<syn::File>(output).unwrap();
    assert_eq!(file.items.len(), 4);
    for ((item, name), value) in file.items.iter().zip(["A", "B", "C", "D"]).zip(1..=4) {
        let syn::Item::Const(item) = item else {
            panic!("constant");
        };
        assert_eq!(item.ident, name);
        assert_eq!(integer(&item.expr), value);
        match name {
            "A" => assert!(matches!(item.vis, syn::Visibility::Public(_))),
            "C" => assert!(matches!(item.vis, syn::Visibility::Restricted(_))),
            _ => assert!(matches!(item.vis, syn::Visibility::Inherited)),
        }
    }
}

fn integer(expression: &syn::Expr) -> u8 {
    match expression {
        syn::Expr::Group(group) => integer(&group.expr),
        syn::Expr::Lit(literal) => match &literal.lit {
            syn::Lit::Int(value) => value.base10_parse().unwrap(),
            _ => panic!("integer literal"),
        },
        _ => panic!("independently expected constant value"),
    }
}

#[test]
fn zero_width_visibility_repetitions_and_real_ambiguity_remain_closed() {
    for (definition, invocation, expected) in [
        (
            "macro_rules! m { ($($vis:vis)*) => {}; }",
            "m!();",
            "matches empty input",
        ),
        (
            "macro_rules! m { ($($name:ident)* $last:ident) => {}; }",
            "m!(word);",
            "Ambiguous",
        ),
    ] {
        let error = run(definition, invocation).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn inherited_visibility_preserves_cleanup_inside_captured_expression() {
    let definition = "macro_rules! m { ($(#[$doc:meta])* $vis:vis const $name:ident = $body:expr;) => { $vis fn $name() { $body; } }; }";
    let invocation = "m! { #[doc = \"inherited\"] const value = { impl Drop for Error { fn drop(&mut self) {} } 0 }; }";
    let output = run(definition, invocation).unwrap();
    let file = syn::parse2::<syn::File>(output).unwrap();
    let mut drops = Drops::default();
    drops.visit_file(&file);
    assert_eq!(drops.0, 1);
}
