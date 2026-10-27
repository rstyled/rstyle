use super::*;

fn resolution(bytes: &str) -> (Resolution, TypeTree) {
    let mut resolution = super::super::tests::lexical(bytes);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../product_instances/conversion/fixture.json"
    ))
    .unwrap();
    let span: TypeTree = serde_json::from_value(fixture["span"].clone()).unwrap();
    resolution.trees.insert(vec!["Span".into()], span.clone());
    (resolution, span)
}

#[test]
fn reverse_owner_requires_lexical_from_and_range_with_captured_local_result() {
    let (resolution, span) = resolution("use core::ops::Range; struct Span;");
    let scope = Scope {
        resolution: &resolution,
        namespace: &resolution.namespace,
    };
    let item: syn::ItemImpl = syn::parse_quote! {
        impl From<Range<usize>> for Span {
            fn from(range: Range<usize>) -> Span { Span }
        }
    };
    assert_eq!(trait_owner(&scope, &item).unwrap(), span);
    for text in [
        "impl From<Range<u8>> for Span {}",
        "impl Other<Range<usize>> for Span {}",
        "impl From<Span> for Range<usize> {}",
        "impl From<Range<usize>> for Unknown {}",
    ] {
        let changed: syn::ItemImpl = syn::parse_str(text).unwrap();
        assert!(trait_owner(&scope, &changed).is_err(), "{text}");
    }
}

#[test]
fn reverse_method_preserves_owned_immutable_input_and_exact_output() {
    let (resolution, span) = resolution("use core::ops::Range; struct Span;");
    let scope = Scope {
        resolution: &resolution,
        namespace: &resolution.namespace,
    };
    for text in [
        "fn from(range: Range<usize>) -> Span { Span }",
        "fn from(range: Range<usize>) -> Self { Span }",
    ] {
        let method = syn::parse_str(text).unwrap();
        method_signature(&scope, &method, &span).unwrap();
    }
    for text in [
        "fn from(mut range: Range<usize>) -> Span { Span }",
        "fn from(ref range: Range<usize>) -> Span { Span }",
        "fn from(range: &Range<usize>) -> Span { Span }",
        "fn from(range: Range<u8>) -> Span { Span }",
        "fn from(range: Range<usize>) -> Range<usize> { range }",
        "fn from<T>(range: Range<usize>) -> Span { Span }",
        "pub fn from(range: Range<usize>) -> Span { Span }",
    ] {
        let method = syn::parse_str(text).unwrap();
        assert!(method_signature(&scope, &method, &span).is_err(), "{text}");
    }
}

#[test]
fn reverse_trait_rejects_local_library_shadowing_and_unresolved_globs() {
    for bytes in [
        "use core::ops::Range; struct Span; struct From;",
        "use core::ops::Range; struct Span; type usize = u8;",
        "use core::ops::*; struct Span;",
        "use other::Range; struct Span;",
    ] {
        let (resolution, _) = resolution(bytes);
        let scope = Scope {
            resolution: &resolution,
            namespace: &resolution.namespace,
        };
        let item: syn::ItemImpl = syn::parse_quote! {
            impl From<Range<usize>> for Span {}
        };
        assert!(trait_owner(&scope, &item).is_err(), "{bytes}");
    }
}
