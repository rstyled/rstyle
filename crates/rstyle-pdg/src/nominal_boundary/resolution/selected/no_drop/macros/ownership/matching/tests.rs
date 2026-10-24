use super::*;
use syn::visit::Visit;
#[path = "visibility_tests.rs"]
mod visibility;

fn run(definition: &str, invocation: &str) -> Result<TokenStream, String> {
    let definition = syn::parse_str::<syn::ItemMacro>(definition).unwrap();
    let invocation = syn::parse_str::<syn::ItemMacro>(invocation).unwrap();
    expand(&definition, &invocation.mac, Context { edition: "2021" })
}

const OWNERS: &[&str] = &[
    "Day",
    "MonthRepr",
    "Month",
    "Ordinal",
    "WeekdayRepr",
    "Weekday",
    "WeekNumberRepr",
    "WeekNumber",
    "YearRepr",
    "YearRange",
    "Year",
    "Hour",
    "Minute",
    "Period",
    "Second",
    "SubsecondDigits",
    "Subsecond",
    "OffsetHour",
    "OffsetMinute",
    "OffsetSecond",
    "Padding",
    "UnixTimestampPrecision",
    "UnixTimestamp",
    "TrailingInput",
    "End",
];

#[test]
fn complete_original_modifier_family_retains_both_impls_for_every_owner() {
    let file = syn::parse_file(include_str!("modifier.rs.txt")).unwrap();
    let [syn::Item::Macro(definition), syn::Item::Macro(invocation)] = file.items.as_slice() else {
        panic!("complete original macro and invocation");
    };
    for edition in ["2021", "2024"] {
        let tokens = expand(definition, &invocation.mac, Context { edition }).unwrap();
        let output = syn::parse2::<syn::File>(tokens).unwrap();
        assert_eq!(output.items.len(), OWNERS.len() * 2);
        for (items, owner) in output.items.as_chunks::<2>().0.iter().zip(OWNERS) {
            check_pair(items, owner);
        }
    }
}

fn check_pair(items: &[syn::Item], owner: &str) {
    let [syn::Item::Impl(inherent), syn::Item::Impl(default)] = items else {
        panic!("both implementations");
    };
    assert_eq!(type_name(&inherent.self_ty), owner);
    assert_eq!(type_name(&default.self_ty), owner);
    assert!(inherent.trait_.is_none());
    assert!(default.trait_.as_ref().unwrap().1.is_ident("Default"));
    assert!(
        matches!(inherent.items.as_slice(), [syn::ImplItem::Macro(item)] if item.mac.path.is_ident("if_pub"))
    );
    assert!(
        matches!(default.items.as_slice(), [syn::ImplItem::Fn(item)] if item.sig.ident == "default" && !item.block.stmts.is_empty())
    );
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Group(group) => type_name(&group.elem),
        syn::Type::Path(path) => path.path.get_ident().unwrap().to_string(),
        _ => panic!("original concrete modifier owner"),
    }
}

#[test]
fn impossible_type_starts_discharge_repetition_lookahead() {
    let definition = "macro_rules! m { ($(#[$doc:meta])* $(@$tag:ident)? $ty:ty => $body:expr;) => { fn value() { $body; } }; }";
    for invocation in [
        "m!(u8 => 0;);",
        "m!(#[doc = \"marker\"] u8 => 0;);",
        "m!(@pub u8 => 0;);",
        "m!(#[doc = \"marker\"] @pub u8 => 0;);",
    ] {
        let output = run(definition, invocation).unwrap();
        let file = syn::parse2::<syn::File>(output).unwrap();
        assert!(matches!(file.items.as_slice(), [syn::Item::Fn(_)]));
    }
}

#[test]
fn actual_malformed_types_and_local_ambiguity_remain_fatal() {
    let definition = "macro_rules! m { ($(#[$doc:meta])* $ty:ty) => {}; ($($rest:tt)*) => { impl Drop for Error { fn drop(&mut self) {} } }; }";
    for invocation in [
        "m!(Vec<);",
        "m!(#[doc = \"x\"] Vec<);",
        "m!(&);",
        "m!([u8;]);",
    ] {
        let error = run(definition, invocation).unwrap_err();
        assert!(error.contains("unsupported or malformed syntax"), "{error}");
    }
    let ambiguous = "macro_rules! m { ($($name:ident)* $ty:ty) => {}; }";
    let error = run(ambiguous, "m!(u8);").unwrap_err();
    assert!(error.contains("Ambiguous ownership macro repetition boundary"));
}

#[test]
fn ordered_type_arm_yields_to_independently_compiled_tag_and_attribute_arms() {
    let definition = "macro_rules! m { ($ty:ty) => { const VALUE: u8 = 0; }; (@pub $ty:ty) => { const VALUE: u8 = 1; }; (#[$meta:meta] $ty:ty) => { const VALUE: u8 = 2; }; }";
    for (invocation, expected) in [
        ("m!(u8);", "0"),
        ("m!(@pub u8);", "1"),
        ("m!(#[doc = \"marker\"] u8);", "2"),
    ] {
        let output = run(definition, invocation).unwrap();
        let file = syn::parse2::<syn::File>(output).unwrap();
        let [syn::Item::Const(item)] = file.items.as_slice() else {
            panic!("one selected arm");
        };
        let syn::Expr::Lit(literal) = item.expr.as_ref() else {
            panic!("independent arm value");
        };
        assert!(matches!(&literal.lit, syn::Lit::Int(value) if value.base10_digits() == expected));
    }
}

#[derive(Default)]
struct Drops(usize);
impl<'ast> Visit<'ast> for Drops {
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if item
            .trait_
            .as_ref()
            .is_some_and(|(_, path, _)| path.is_ident("Drop"))
        {
            self.0 += 1;
        }
        syn::visit::visit_item_impl(self, item);
    }
}

#[test]
fn tagged_attribute_capture_keeps_all_nested_cleanup_items() {
    let definition = "macro_rules! m { ($(#[$doc:meta])* $(@$tag:ident)? $ty:ty => $body:expr;) => { fn value() { $body; } impl Drop for Error { fn drop(&mut self) {} } }; }";
    let invocation =
        "m!(#[doc = \"x\"] @pub u8 => { impl Drop for Other { fn drop(&mut self) {} } 0 };);";
    let output = run(definition, invocation).unwrap();
    let file = syn::parse2::<syn::File>(output).unwrap();
    let mut drops = Drops::default();
    drops.visit_file(&file);
    assert_eq!(drops.0, 2);
}
