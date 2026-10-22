use super::*;
use syn::spanned::Spanned;

#[test]
fn only_original_exclusive_prefix_get_syntax_selects_receiver_and_end() {
    let original = "selected.get(..{ selected = other; end })";
    let expression: syn::Expr = syn::parse_str(original).unwrap();
    let (call, range) = source::shape(&expression).unwrap();
    let span = |span: proc_macro2::Span| crate::ByteSpan {
        start: crate::source_evidence::byte_offset(original, span.start()) as u32,
        end: crate::source_evidence::byte_offset(original, span.end()) as u32,
    };
    let receiver = span(call.receiver.span());
    let end = span(range.end.as_deref().unwrap().span());
    assert_eq!(
        &original[receiver.start as usize..receiver.end as usize],
        "selected"
    );
    assert_eq!(
        &original[end.start as usize..end.end as usize],
        "{ selected = other; end }"
    );
    assert!(
        receiver.end < end.start,
        "the saved receiver precedes endpoint effects"
    );
    assert!(!crate::source_evidence::corresponding_span(
        original, receiver, end
    ));
    for invalid in [
        "a.get(i)",
        "a.get(1..end)",
        "a.get(..=end)",
        "a.get(..)",
        "a.get::<usize>(..end)",
        "a.get(..end, extra)",
        "a.get_mut(..end)",
    ] {
        let expression: syn::Expr = syn::parse_str(invalid).unwrap();
        assert!(source::shape(&expression).is_err(), "{invalid}");
    }
}

#[test]
fn checked_prefix_boundaries_match_independent_view_and_failure_expectations() {
    for length in 0..=16 {
        let bytes: Vec<_> = (0..length).map(|i| i as u8).collect();
        for end in (0..=length + 1).chain([usize::MAX]) {
            let result = bytes.get(..end);
            assert_eq!(result.is_some(), end <= length);
            if let Some(view) = result {
                assert_eq!(view.len(), end);
                assert_eq!(view.as_ptr(), bytes.as_ptr());
                for (index, byte) in view.iter().enumerate() {
                    assert_eq!(usize::from(*byte), index);
                }
            }
        }
    }
    let a = [1, 2, 3];
    let b = [9, 8, 7];
    let mut selected = a.as_slice();
    let view = selected
        .get(
            ..{
                selected = b.as_slice();
                2
            },
        )
        .unwrap();
    assert_eq!(view, &[1, 2]);
    assert_eq!(selected, b);
}

#[test]
fn changed_definition_root_or_implementation_cannot_supply_prefix_authority() {
    let registry: crate::data_library::Registry =
        serde_json::from_str(include_str!("../../contracts/read-only-lengths-v2.json")).unwrap();
    let valid = crate::data_library::Callee {
        path: GET.into(),
        crate_name: "core".into(),
        crate_hash: registry.core_hash.clone(),
        definition_hash: registry.definitions[GET].clone(),
        implementation: None,
    };
    authority::method(&valid, &registry).unwrap();
    for mutation in 0..5 {
        let mut forged = valid.clone();
        match mutation {
            0 => forged.path = "custom::get".into(),
            1 => forged.crate_name = "custom".into(),
            2 => forged.crate_hash = "other core".into(),
            3 => forged.definition_hash = "other definition".into(),
            _ => forged.implementation = Some(Box::new(valid.clone())),
        }
        assert!(authority::method(&forged, &registry).is_err());
    }
}
