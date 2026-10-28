use super::*;

fn selected(source: &str, cfgs: &[String]) -> Result<Vec<Declaration>, String> {
    declarations(syn::parse_file(source).unwrap().items, cfgs)
}

#[test]
fn inherent_and_local_trait_names_match_original_root_location_contract() {
    let source = "struct Item; enum Choice { One } trait Measure { fn read(&self); } impl Item { pub fn read(&self) {} fn make() {} } impl Measure for Item { fn read(&self) {} } impl Choice { fn read(&mut self) {} }";
    let declarations = selected(source, &[]).unwrap();
    assert_eq!(
        declarations
            .iter()
            .map(|(name, _, kind)| (name.as_str(), *kind))
            .collect::<Vec<_>>(),
        [
            ("Item::read", "method"),
            ("Item::make", "method"),
            ("<Item as Measure>::read", "method"),
            ("Choice::read", "method"),
        ]
    );
    assert!(declarations[0].1.sig.receiver().is_some());
    assert!(declarations[1].1.sig.receiver().is_none());
}

#[test]
fn method_cfg_selection_precedes_identity_collisions_and_preserves_attributes() {
    let source = "struct Item; impl Item { #[cfg(feature=\"enabled\")] #[inline] fn read() {} #[cfg(not(feature=\"enabled\"))] fn read() {} }";
    let enabled = selected(source, &["feature=\"enabled\"".into()]).unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].0, "Item::read");
    assert_eq!(enabled[0].1.attrs.len(), 2);
    let disabled = selected(source, &[]).unwrap();
    assert_eq!(disabled.len(), 1);
    assert_eq!(disabled[0].1.attrs.len(), 1);
}

#[test]
fn aliases_imported_qualified_generic_and_missing_owners_remain_unavailable() {
    for source in [
        "struct Item; type Alias = Item; impl Alias { fn read() {} }",
        "use foreign::Item; impl Item { fn read() {} }",
        "mod nested { struct Item; } impl nested::Item { fn read() {} }",
        "struct Item<T>(T); impl<T> Item<T> { fn read() {} }",
        "struct Item<T>(T); impl Item { fn read() {} }",
        "impl Item { fn read() {} }",
        "struct Item; trait Measure<T> {} impl Measure for Item { fn read() {} }",
        "struct Item; use foreign::Measure; impl Measure for Item { fn read() {} }",
    ] {
        assert!(selected(source, &[]).unwrap().is_empty(), "{source}");
    }
}

#[test]
fn ambiguous_type_namespace_and_glob_imports_cannot_authorize_plain_owner_names() {
    for source in [
        "struct Item; struct Item; impl Item { fn read() {} }",
        "struct Item; type Item = Other; impl Item { fn read() {} }",
        "struct Item; use foreign::Item; impl Item { fn read() {} }",
        "struct Item; use foreign::{Other as Item}; impl Item { fn read() {} }",
        "struct Item; use foreign::Item::{self}; impl Item { fn read() {} }",
        "struct Item; extern crate foreign as Item; impl Item { fn read() {} }",
        "struct Item; use foreign::*; impl Item { fn read() {} }",
        "struct Item; trait Measure {} type Measure=Other; impl Measure for Item { fn read() {} }",
    ] {
        assert!(selected(source, &[]).unwrap().is_empty(), "{source}");
    }
    assert_eq!(
        selected(
            "struct Item; use foreign::Other; impl Item { fn read() {} }",
            &[]
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn default_trait_methods_and_nested_item_impls_are_not_external_locations() {
    let source = "trait Measure { fn read(&self) {} } fn enclosing() { struct Item; impl Item { fn read() {} } }";
    let declarations = selected(source, &[]).unwrap();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].0, "enclosing");
    assert_eq!(declarations[0].2, "free-function");
}

#[test]
fn converted_method_retains_exact_original_span_with_bom_crlf_unicode_and_attributes() {
    let source = "\u{feff}// 🦀\r\nstruct Item; impl Item { #[inline] pub fn café<T>(&self, value:T) {} }\r\n";
    let parsed = syn::parse_file(source.trim_start_matches('\u{feff}')).unwrap();
    let declarations = declarations(parsed.items, &[]).unwrap();
    let (_, function, _) = &declarations[0];
    let span = super::super::super::shapes::function_range(source, function);
    assert_eq!(
        crate::source_evidence::byte_range(source, span).unwrap(),
        "pub fn café<T>(&self, value:T) {}"
    );
}
