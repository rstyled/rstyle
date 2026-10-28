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

#[test]
fn lifetime_only_owners_keep_nominal_method_names_without_lifetime_spelling() {
    for implementation in [
        "impl Item<'_> { fn read() {} }",
        "impl Item<'static> { fn read() {} }",
        "impl<'a> Item<'a> { fn read() {} }",
        "impl<'renamed> Item<'renamed> { fn read() {} }",
    ] {
        let source = format!("struct Item<'a>(&'a str); {implementation}");
        let declarations = selected(&source, &[]).unwrap();
        assert_eq!(declarations.len(), 1, "{source}");
        assert_eq!(declarations[0].0, "Item::read");
        assert_eq!(declarations[0].2, "method");
    }
    let source = "enum Item<'a,'b> { Value(&'a str,&'b str) } trait Read {} impl<'a> Read for Item<'a,'static> { fn read() {} }";
    let declarations = selected(source, &[]).unwrap();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].0, "<Item as Read>::read");
}

#[test]
fn lifetime_owners_require_exact_arity_and_resolved_plain_lifetime_arguments() {
    for implementation in [
        "impl Item { fn read() {} }",
        "impl Item<'_,'_> { fn read() {} }",
        "impl Item<'unbound> { fn read() {} }",
        "impl<'a> Item<'unbound> { fn read() {} }",
        "impl Item<u32> { fn read() {} }",
        "impl Item<3> { fn read() {} }",
        "impl Item<Assoc = u32> { fn read() {} }",
        "impl Item::<'_> { fn read() {} }",
        "impl<'a: 'static> Item<'a> { fn read() {} }",
        "impl<'a> Item<'a> where 'a: 'static { fn read() {} }",
        "impl<T> Item<'_> { fn read() {} }",
        "impl<const N: usize> Item<'_> { fn read() {} }",
        "impl<'a,T> Item<'a> { fn read() {} }",
    ] {
        let source = format!("struct Item<'a>(&'a str); {implementation}");
        assert!(selected(&source, &[]).unwrap().is_empty(), "{source}");
    }
    let source = "struct Item; impl Item<'_> { fn read() {} }";
    assert!(selected(source, &[]).unwrap().is_empty());
}

#[test]
fn lifetime_declarations_do_not_admit_bounds_aliases_or_generic_traits() {
    for source in [
        "struct Item<'a: 'static>(&'a str); impl Item<'_> { fn read() {} }",
        "struct Item<'a>(&'a str) where 'a: 'static; impl Item<'_> { fn read() {} }",
        "struct Item<'a,T>(&'a T); impl Item<'_> { fn read() {} }",
        "struct Item<'a,const N: usize>(&'a str); impl Item<'_> { fn read() {} }",
        "type Item<'a> = &'a str; impl Item<'_> { fn read() {} }",
        "use foreign::Item; impl Item<'_> { fn read() {} }",
        "mod nested { struct Item<'a>(&'a str); } impl nested::Item<'_> { fn read() {} }",
        "struct Item<'a>(&'a str); trait Read<'a> {} impl Read for Item<'_> { fn read() {} }",
        "struct Item<'a>(&'a str); trait Read<'a> {} impl Read<'_> for Item<'_> { fn read() {} }",
    ] {
        assert!(selected(source, &[]).unwrap().is_empty(), "{source}");
    }
}

#[test]
fn lifetime_parameter_attributes_and_duplicate_binders_remain_unavailable() {
    for source in [
        "struct Item<#[allow(dead_code)] 'a>(&'a str); impl Item<'_> { fn read() {} }",
        "struct Item<'a>(&'a str); impl<#[allow(dead_code)] 'a> Item<'a> { fn read() {} }",
        "struct Item<'a,'a>(&'a str); impl Item<'_,'_> { fn read() {} }",
        "struct Item<'a>(&'a str); impl<'a,'a> Item<'a> { fn read() {} }",
    ] {
        assert!(selected(source, &[]).unwrap().is_empty(), "{source}");
    }
}

#[test]
fn lifetime_owners_preserve_cfg_selection_and_type_namespace_ambiguity() {
    let source = "struct Item<'a>(&'a str); impl Item<'_> { #[cfg(feature=\"enabled\")] fn read() {} #[cfg(not(feature=\"enabled\"))] fn read() {} }";
    for cfgs in [vec![], vec!["feature=\"enabled\"".into()]] {
        let declarations = selected(source, &cfgs).unwrap();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].0, "Item::read");
    }
    for conflicting in [
        "struct Item<'b>(&'b str);",
        "type Item<'a> = &'a str;",
        "use foreign::Item;",
        "use foreign::*;",
    ] {
        let source =
            format!("struct Item<'a>(&'a str); {conflicting} impl Item<'_> {{ fn read() {{}} }}");
        assert!(selected(&source, &[]).unwrap().is_empty(), "{source}");
    }
}
