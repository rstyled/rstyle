//! Indexing narrows candidates while retaining exact bytes and lexical owners.
use super::*;

const METHOD: &str = "pub fn target() -> u8 { 0 }";
const OTHER: &str = "pub fn target() -> u8 { 1 }";

fn span(source: &str, method: &str) -> ByteSpan {
    let start = source.find(method).unwrap() as u32;
    ByteSpan {
        start,
        end: start + method.len() as u32,
    }
}

fn environment(source: &str) -> Environment {
    let mut census = (*super::super::tests::census()).clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = census.files[0].bytes.identity().to_owned();
    Environment::with_options(Arc::new(census), true, Some(&[]), false).unwrap()
}

fn facts(source: &str, method: &str) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts.fn_span = span(source, method);
    facts.source.bytes = method.into();
    facts
}

fn mixed_source() -> String {
    format!(
        "pub struct First; pub struct Second; impl First {{ pub fn before() {{}} {METHOD} }} impl Second {{ {OTHER} }}"
    )
}

fn candidates(environment: &Environment, facts: &HirFnFacts) -> Vec<FunctionOwner> {
    let bytes = &environment.census.files[0].bytes;
    module_functions(0, bytes, facts, environment).unwrap()
}

#[test]
fn mixed_implementations_keep_the_exact_member_and_owner() {
    let source = mixed_source();
    let mut environment = environment(&source);
    environment.copy_boundary = false;
    let facts = facts(&source, METHOD);

    let owners = candidates(&environment, &facts);
    assert_eq!(owners.len(), 1);
    assert!(
        matches!(owners[0].1.as_deref(), Some(syn::Type::Path(path)) if path.path.is_ident("First"))
    );

    assert_eq!(environment.owner_candidates(0, facts.fn_span).count(), 1);
    assert_eq!(environment.function_candidates(0, facts.fn_span).count(), 0);
}

#[test]
fn wrong_method_bytes_wrong_spans_and_changed_source_stay_unmatched() {
    let source = mixed_source();
    let mut original = environment(&source);
    original.copy_boundary = false;
    let mut selected = facts(&source, METHOD);

    selected.source.bytes = OTHER.into();
    assert!(candidates(&original, &selected).is_empty());

    selected.source.bytes = METHOD.into();
    selected.fn_span = span(&source, OTHER);
    assert!(candidates(&original, &selected).is_empty());

    let mut changed = environment(&source.replace(METHOD, "pub fn target() -> u8 { 9 }"));
    changed.copy_boundary = false;
    assert!(candidates(&changed, &facts(&source, METHOD)).is_empty());
}

#[test]
fn duplicate_implementation_and_member_handles_preserve_ambiguity() {
    let source = mixed_source();
    let mut environment = environment(&source);
    environment.copy_boundary = false;

    let original = environment.modules[0]
        .items
        .iter()
        .position(|item| matches!(item, syn::Item::Impl(_)))
        .unwrap();
    let duplicate = environment.modules[0].items[original].clone();
    environment.modules[0].items.push(duplicate);
    environment.functions = Index::new(&environment.modules, &environment.census).unwrap();

    let selected = facts(&source, METHOD);
    assert_eq!(candidates(&environment, &selected).len(), 2);

    let syn::Item::Impl(owner) = &mut environment.modules[0].items[original] else {
        panic!()
    };
    let duplicate = owner.items[1].clone();
    owner.items.push(duplicate);

    environment.functions = Index::new(&environment.modules, &environment.census).unwrap();
    assert_eq!(candidates(&environment, &selected).len(), 3);
}

#[test]
fn span_lookup_keeps_one_method_candidate_as_nonmatching_siblings_grow() {
    for size in [16, 128, 1024] {
        let methods = (0..size)
            .map(|i| format!("pub fn m{i}() -> u8 {{ 0 }}\n"))
            .collect::<String>();
        let source = format!("impl Owner {{\n{methods}}}");
        let file = syn::parse_file(&source).unwrap();
        let mut index = Index::default();
        index.module(0, &source, &file.items);

        for member in [0, size / 2, size - 1] {
            let method = format!("pub fn m{member}() -> u8 {{ 0 }}");
            let handles = index.handles(0, span(&source, &method)).collect::<Vec<_>>();
            assert!(
                matches!(handles.as_slice(), [Handle::Method { implementation: 0, member: found }] if *found == member)
            );
        }

        assert_eq!(index.0.len(), size);
    }
}

#[test]
fn free_functions_keep_exact_ranges_and_ambiguous_duplicate_handles() {
    let source = "pub fn f() -> u8 { 0 }\npub fn g() -> u8 { 1 }";
    let file = syn::parse_file(source).unwrap();
    let duplicate = vec![
        file.items[0].clone(),
        file.items[0].clone(),
        file.items[1].clone(),
    ];

    let mut index = Index::default();
    index.module(0, source, &duplicate);
    let positions = |text| {
        index
            .handles(0, span(source, text))
            .map(|handle| match handle {
                Handle::Function(position) => position,
                Handle::Method { .. } => panic!("Expected free function handle"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(positions("pub fn f() -> u8 { 0 }"), [0, 1]);
    assert_eq!(positions("pub fn g() -> u8 { 1 }"), [2]);
}

#[test]
fn original_byte_spans_handle_bom_utf8_and_crlf() {
    for prefix in ["// π\r\n", "\u{feff}// π\r\n"] {
        let source = format!("{prefix}pub struct Owner; impl Owner {{ {METHOD} }}");
        let parsed = source.strip_prefix('\u{feff}').unwrap_or(&source);
        let file = syn::parse_file(parsed).unwrap();

        let mut index = Index::default();
        index.module(0, &source, &file.items);
        assert_eq!(
            index.handles(0, span(&source, METHOD)).count(),
            1,
            "{prefix:?}"
        );
    }
}

#[test]
fn shebang_span_reset_preserves_existing_exact_predicate_rejection() {
    let source = format!("#!/usr/bin/env rust-script\n// π\nimpl Owner {{ {METHOD} }}");
    let file = syn::parse_file(&source).unwrap();
    let syn::Item::Impl(owner) = &file.items[0] else {
        panic!()
    };
    let syn::ImplItem::Fn(method) = &owner.items[0] else {
        panic!()
    };

    // syn strips the shebang before assigning byte ranges. The existing exact
    // predicate rejects that shifted range; indexing must grant no new owner.
    let selected = facts(&source, METHOD);
    assert!(!exact_method(method, &source, &selected, false));
    let mut index = Index::default();
    index.module(0, &source, &file.items);
    assert_eq!(index.handles(0, selected.fn_span).count(), 0);
}

#[test]
fn selected_matching_implementation_and_method_attributes_are_still_checked() {
    let source = mixed_source();
    for implementation in [false, true] {
        let mut environment = environment(&source);
        environment.copy_boundary = true;
        let owner = environment.modules[0]
            .items
            .iter_mut()
            .find_map(|item| {
                if let syn::Item::Impl(owner) = item {
                    Some(owner)
                } else {
                    None
                }
            })
            .unwrap();

        if implementation {
            owner.attrs.push(syn::parse_quote!(#[unknown]));
        } else {
            let syn::ImplItem::Fn(method) = &mut owner.items[1] else {
                panic!()
            };
            method.attrs.push(syn::parse_quote!(#[unknown]));
        }
        assert!(candidates(&environment, &facts(&source, METHOD)).is_empty());
    }
}
