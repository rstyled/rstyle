//! The selected u32 route has independent fixed expectations and policy.
use super::*;

#[test]
fn original_u32_trait_helper_calls_agree_with_reviewed_64_bit_core_instances() {
    let original: Value = serde_json::from_str(include_str!("original-u32-usize.json")).unwrap();
    assert_eq!(original["function"], "<u32 as util::int::U32>::as_usize");
    let registry: crate::data_library::Registry =
        serde_json::from_str(include_str!("../../../contracts/read-only-lengths-v2.json")).unwrap();
    assert_eq!(original["target"], registry.target);
    authority::sources(&registry).unwrap();
    let calls = original["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    for call in calls {
        let kind = if call["requested"]["path"] == "core::convert::TryFrom::try_from" {
            Kind::ConvertU32Usize
        } else {
            Kind::ExpectUsize
        };
        selection::relationship(call, kind, &registry).unwrap();
    }
}

#[test]
fn u32_usize_has_distinct_policy_constructs_and_concrete_signatures() {
    let conversion = Kind::ConvertU32Usize;
    let expect = Kind::ExpectUsize;
    assert_eq!(conversion.capability(), "rust.lossless-u32-usize-calls");
    assert_eq!(expect.capability(), conversion.capability());
    assert_ne!(conversion.capability(), Kind::Convert.capability());
    assert_eq!(expect.conversion(), conversion);
    assert_ne!(conversion.construct(), Kind::Convert.construct());
    assert_eq!(
        conversion.signature().unwrap(),
        (
            vec!["u32".into()],
            "{\"kind\":\"int_conversion_result\",\"value\":\"usize\"}".into()
        )
    );
    assert_eq!(expect.trees().1, TypeTree::Scalar("usize".into()));
    let mut historical = crate::extraction_profile::ExtractionProfile::DataV3.policy();
    historical.version = "3.108.0".into();
    assert!(!crate::extraction_profile::policy_supports(
        &historical,
        U32_USIZE_CAPABILITY
    ));
}

#[test]
fn selected_u32_substitutions_cannot_be_u64_signed_swapped_or_narrowing() {
    let registry = registry();
    for input in ["u64", "i32", "usize"] {
        let mut forged = call(Kind::ConvertU32Usize);
        forged["requested_arguments"][1] = builtin(input);
        assert!(selection::relationship(&forged, Kind::ConvertU32Usize, &registry).is_err());
    }
    let mut reversed = call(Kind::ConvertU32Usize);
    reversed["requested_arguments"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(selection::relationship(&reversed, Kind::ConvertU32Usize, &registry).is_err());
    for selected in [
        definition(IMPLEMENTATION, "exact-impl"),
        definition(CONVERT, "try-trait"),
    ] {
        let mut forged = call(Kind::ConvertU32Usize);
        forged["selected"] = selected;
        assert!(selection::relationship(&forged, Kind::ConvertU32Usize, &registry).is_err());
    }
}

#[test]
fn usize_expect_keeps_actual_error_identity_and_both_structural_substitutions() {
    let registry = registry();
    assert!(
        selection::relationship(&call(Kind::ExpectUsize), Kind::ExpectUsize, &registry).is_ok()
    );
    for identity in [
        definition("std::convert::Infallible", "actual-error"),
        definition(ERROR, "different-error"),
    ] {
        let mut forged = call(Kind::ExpectUsize);
        for arguments in ["requested_arguments", "selected_arguments"] {
            forged[arguments][1]["identity"]["definition"] = identity.clone();
        }
        assert!(selection::relationship(&forged, Kind::ExpectUsize, &registry).is_err());
    }
    let mut unknown = call(Kind::ExpectUsize);
    unknown["requested_arguments"][1]["identity_status"] = json!("unknown");
    assert!(selection::relationship(&unknown, Kind::ExpectUsize, &registry).is_err());
}

#[test]
fn exact_usize_conversion_source_and_static_expect_message_are_retained() {
    assert_eq!(
        source_arguments("usize::try_from(self)", Kind::ConvertU32Usize).unwrap(),
        1
    );
    assert_eq!(
        source_arguments(
            "usize::try_from(self).expect(\"u32 overflowed usize\")",
            Kind::ExpectUsize
        )
        .unwrap(),
        2
    );
    for bytes in [
        "u64::try_from(self)",
        "u32::try_from(self)",
        "Custom::try_from(self)",
        "usize::try_from::<u32>(self)",
        "<usize as TryFrom<u32>>::try_from(self)",
    ] {
        assert!(
            source_arguments(bytes, Kind::ConvertU32Usize).is_err(),
            "{bytes}"
        );
    }
    for bytes in [
        "result.expect(effect())",
        "result.expect(SAVED)",
        "result.unwrap()",
    ] {
        assert!(
            source_arguments(bytes, Kind::ExpectUsize).is_err(),
            "{bytes}"
        );
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn independently_specified_u32_boundary_values_are_exact_and_no_error_is_created() {
    for (input, expected) in [
        (0u32, 0usize),
        (1, 1),
        (255, 255),
        (65_535, 65_535),
        (2_147_483_648, 2_147_483_648),
        (4_294_967_295, 4_294_967_295),
    ] {
        let converted: Result<usize, std::num::TryFromIntError> = usize::try_from(input);
        assert_eq!(converted, Ok(expected));
        assert_eq!(converted.expect("u32 overflowed usize"), expected);
    }
    // A shared error type does not prove an unrelated Err arm impossible.
    let error: Result<usize, std::num::TryFromIntError> = u8::try_from(256u32).map(usize::from);
    assert!(std::panic::catch_unwind(|| error.expect("real error")).is_err());
}
