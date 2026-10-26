//! u64 native-width conversion is total only under the reviewed 64-bit contract.
use super::*;

#[test]
fn captured_u64_calls_bind_each_targets_actual_core_implementation() {
    let originals: Value = serde_json::from_str(include_str!("captured-u64-usize.json")).unwrap();
    assert_eq!(
        originals.as_array().unwrap().len(),
        crate::data_library::registries().len()
    );
    for (original, registry) in originals
        .as_array()
        .unwrap()
        .iter()
        .zip(crate::data_library::registries())
    {
        assert_eq!(original["target"], registry.target);
        authority::sources(registry).unwrap();
        assert_eq!(original["calls"].as_array().unwrap().len(), 2);
        for call in original["calls"].as_array().unwrap() {
            let kind = if call["requested"]["path"] == "std::convert::TryFrom::try_from" {
                Kind::ConvertU64Usize
            } else {
                Kind::ExpectU64Usize
            };
            selection::relationship(call, kind, registry).unwrap();
        }
    }
}

#[test]
fn u64_substitutions_and_selected_impl_are_independent_authority() {
    let kind = Kind::ConvertU64Usize;
    let registry = registry();
    selection::relationship(&call(kind), kind, &registry).unwrap();
    for input in ["usize", "u32", "i64", "u128"] {
        let mut forged = call(kind);
        forged["requested_arguments"][1] = builtin(input);
        assert!(selection::relationship(&forged, kind, &registry).is_err());
    }
    let mut forged = call(kind);
    forged["selected"] = definition(U32_USIZE_IMPLEMENTATION, "exact-u32-usize-impl");
    assert!(selection::relationship(&forged, kind, &registry).is_err());
    assert_ne!(kind.capability(), Kind::ConvertU32Usize.capability());
}

#[cfg(target_pointer_width = "64")]
#[test]
fn high_u64_bits_are_preserved_and_narrowing_errors_remain_real() {
    for value in [0u64, 1, 2_147_483_646, 4_294_967_296, 1 << 63, u64::MAX] {
        assert_eq!(usize::try_from(value).unwrap() as u64, value);
    }
    assert!(u32::try_from(4_294_967_296u64).is_err());
}
