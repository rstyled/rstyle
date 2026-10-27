//! Original captured selections plus independently specified source/effect attacks.
use super::*;
use crate::product_instances::test_support::conversion as fixture;
use serde_json::json;

#[test]
fn original_span_conversion_preserves_complete_receiver_value() {
    let (canonical, instance, span) = fixture();
    let checked = verify(&canonical, &instance, &span, SpanArgument::Span).unwrap();
    let (call, receiver) = source::site(&canonical).unwrap();
    assert_eq!(checked.call(), call);
    assert_eq!(checked.receiver(), receiver);
    assert!(verify(&canonical, &instance, &span, SpanArgument::RangeUsize).is_err());
    let mut display = instance.clone();
    display["conversions"][0]["receiver_type"][0]["value"] = json!("display is not authority");
    assert!(verify(&canonical, &display, &span, SpanArgument::Span).is_ok());
}

#[test]
fn edited_selection_owner_identity_or_adjustment_cannot_mint_identity() {
    let (canonical, instance, span) = fixture();
    for selection in ["into", "from"] {
        for role in [
            "requested",
            "requested_trait",
            "selected",
            "selected_impl",
            "selected_trait",
        ] {
            for field in [
                "definition_identity",
                "crate_hash",
                "stable_crate_id",
                "path",
            ] {
                let mut changed = instance.clone();
                changed["conversions"][0][selection][role][field] = json!("changed");
                assert!(
                    verify(&canonical, &changed, &span, SpanArgument::Span).is_err(),
                    "{selection}/{role}/{field}"
                );
            }
        }
    }

    for (field, value) in [
        ("adjustments", json!([{"kind":"borrow"}])),
        ("receiver", json!({"krate":0,"owner":843,"local":4})),
        ("receiver_type", json!([])),
        ("result_type", json!([])),
        ("identity", json!("invented-purity")),
    ] {
        let mut changed = instance.clone();
        changed["conversions"][0][field] = value;
        assert!(
            verify(&canonical, &changed, &span, SpanArgument::Span).is_err(),
            "{field}"
        );
    }
}

#[test]
fn incomplete_or_mismatched_whole_value_substitutions_fail() {
    let (canonical, instance, span) = fixture();
    for selection in ["into", "from"] {
        for field in [
            "requested_arguments",
            "selected_arguments",
            "substituted_impl_trait_arguments",
            "original_impl_trait_arguments",
        ] {
            let mut changed = instance.clone();
            changed["conversions"][0][selection][field]
                .as_array_mut()
                .unwrap()
                .pop();
            assert!(verify(&canonical, &changed, &span, SpanArgument::Span).is_err());
        }
        for signature in ["requested_signature", "selected_signature"] {
            let mut changed = instance.clone();
            changed["conversions"][0][selection][signature]["bound_variables"] = json!(1);
            assert!(verify(&canonical, &changed, &span, SpanArgument::Span).is_err());
        }
    }

    let mut changed = canonical.clone();
    changed
        .build
        .frontend_options
        .get_mut("rstyle.rust.call-instances")
        .unwrap()["complete"] = json!(false);
    assert!(verify(&changed, &instance, &span, SpanArgument::Span).is_err());
    let mut changed_span = span.clone();
    let TypeTree::Struct { fields, .. } = &mut changed_span else {
        panic!("captured Span");
    };
    fields.swap(0, 1);
    assert!(verify(&canonical, &instance, &changed_span, SpanArgument::Span).is_err());
}
