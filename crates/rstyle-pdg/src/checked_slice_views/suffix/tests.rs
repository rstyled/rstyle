//! Independent suffix bytes, address, bounds and saved-receiver expectations.
use super::*;
use syn::spanned::Spanned;

#[test]
fn original_suffix_source_preserves_receiver_before_effectful_start() {
    let bytes = "selected.get({ selected = other; start }..)";
    let expression: syn::Expr = syn::parse_str(bytes).unwrap();
    let (call, range) = source::shape(&expression).unwrap();
    let span = |span: proc_macro2::Span| crate::ByteSpan {
        start: crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };
    let receiver = span(call.receiver.span());
    let start = span(range.start.as_deref().unwrap().span());
    assert_eq!(
        &bytes[receiver.start as usize..receiver.end as usize],
        "selected"
    );
    assert_eq!(
        &bytes[start.start as usize..start.end as usize],
        "{ selected = other; start }"
    );
    assert!(receiver.end < start.start);
    assert!(!crate::source_evidence::corresponding_span(
        bytes, receiver, start
    ));
}

#[test]
fn prefix_inclusive_full_and_custom_get_syntax_are_not_suffix_operations() {
    for invalid in [
        "a.get(i)",
        "a.get(..end)",
        "a.get(start..end)",
        "a.get(start..=end)",
        "a.get(..)",
        "a.get::<usize>(start..)",
        "a.get(start.., extra)",
        "a.get_mut(start..)",
        "get(a, start..)",
    ] {
        let expression: syn::Expr = syn::parse_str(invalid).unwrap();
        assert!(source::shape(&expression).is_err(), "{invalid}");
    }
}

#[test]
fn every_suffix_preserves_exact_length_contents_and_including_empty_end_pointer() {
    let backing: Vec<u8> = (0..64).map(|index| index as u8).collect();
    for offset in [0, 1, 7] {
        for length in 0..=32 {
            let bytes = &backing[offset..offset + length];
            for start in (0..=length + 1).chain([usize::MAX / 2, usize::MAX]) {
                let result = bytes.get(start..);
                assert_eq!(result.is_some(), start <= length);
                if let Some(view) = result {
                    assert_eq!(view.len(), length - start);
                    assert_eq!(view.as_ptr(), bytes.as_ptr().wrapping_add(start));
                    assert!(
                        view.iter()
                            .enumerate()
                            .all(|(index, byte)| { *byte == (offset + start + index) as u8 })
                    );
                }
            }
        }
    }
}

#[test]
fn start_evaluation_cannot_replace_the_saved_backing_slice() {
    let a = [1, 2, 3];
    let b = [1, 2, 3];
    let mut selected = a.as_slice();
    let mut evaluations = 0;
    let view = selected
        .get(
            {
                selected = b.as_slice();
                evaluations += 1;
                1
            }..,
        )
        .unwrap();
    assert_eq!(view, &[2, 3]);
    assert_eq!(view.as_ptr(), a.as_ptr().wrapping_add(1));
    assert_ne!(view.as_ptr(), b.as_ptr().wrapping_add(1));
    assert_eq!(selected.as_ptr(), b.as_ptr());
    assert_eq!(evaluations, 1);
}

#[test]
fn suffix_lifetime_follows_stored_slice_instead_of_temporary_owner() {
    struct Line<'a> {
        remaining: &'a [u8],
    }
    let bytes = [11, 22, 33];
    for start in 0..=bytes.len() {
        let view = {
            let line = Line { remaining: &bytes };
            line.remaining.get(start..).unwrap()
        };
        assert_eq!(view.as_ptr(), bytes.as_ptr().wrapping_add(start));
        assert_eq!(view.len(), bytes.len() - start);
    }
}

#[test]
fn serialized_contract_states_saved_address_bounds_and_order_without_reads() {
    let rule = semantics();
    assert_eq!(rule["some_iff"], "saved_start <= saved_receiver_length");
    assert_eq!(rule["some_pointer"], "saved_receiver_pointer + saved_start");
    assert_eq!(rule["some_length"], "saved_receiver_length - saved_start");
    assert_eq!(
        rule["empty_pointer"],
        "saved_receiver_pointer + saved_receiver_length"
    );
    assert_eq!(
        rule["evaluation_order"],
        serde_json::json!(["receiver", "start"])
    );
    assert_eq!(rule["evaluation_count"], "each-once");
    assert_eq!(rule["content_read"], false);
    assert_eq!(rule["may_panic"], false);
}

#[test]
fn changed_get_definition_root_or_implementation_cannot_supply_suffix_authority() {
    let registry = crate::data_library::registry();
    let valid = crate::data_library::Callee {
        path: GET.into(),
        crate_name: "core".into(),
        crate_hash: registry.core_hash.clone(),
        definition_hash: registry.definitions[GET].clone(),
        implementation: None,
    };
    authority::method(&valid, registry).unwrap();
    for mutation in 0..5 {
        let mut changed = valid.clone();
        match mutation {
            0 => changed.path = "custom::get".into(),
            1 => changed.crate_name = "custom".into(),
            2 => changed.crate_hash = "other core".into(),
            3 => changed.definition_hash = "other definition".into(),
            _ => changed.implementation = Some(Box::new(valid.clone())),
        }
        assert!(authority::method(&changed, registry).is_err());
    }
}

#[test]
fn suffix_carrier_cannot_use_another_range_or_target_definition() {
    for registry in crate::data_library::registries() {
        let valid = crate::data_library::Callee {
            path: "core::ops::RangeFrom".into(),
            crate_name: "core".into(),
            crate_hash: registry.core_hash.clone(),
            definition_hash: registry.definitions["std::ops::RangeFrom"].clone(),
            implementation: None,
        };
        authority::range_definition(&valid, registry).unwrap();
        for mutation in 0..5 {
            let mut changed = valid.clone();
            match mutation {
                0 => changed.path = "core::ops::RangeTo".into(),
                1 => changed.path = "core::range::RangeFrom".into(),
                2 => changed.crate_hash = "other core".into(),
                3 => changed.definition_hash = "other definition".into(),
                _ => changed.implementation = Some(Box::new(valid.clone())),
            }
            assert!(authority::range_definition(&changed, registry).is_err());
        }
    }
}
