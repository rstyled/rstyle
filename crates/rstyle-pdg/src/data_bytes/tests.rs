use super::*;
use crate::{data_library::Callee, data_origins::Origin, field_locations::Projection};

fn registry() -> crate::data_library::Registry {
    serde_json::from_str(include_str!("../../contracts/read-only-lengths-v2.json")).unwrap()
}

#[test]
fn borrowed_conversion_authority_rejects_forged_selected_implementation() {
    let registry = registry();
    let authority = |path: &str| Callee {
        path: path.into(),
        crate_name: "core".into(),
        crate_hash: registry.core_hash.clone(),
        definition_hash: registry.definitions[path].clone(),
        implementation: None,
    };
    let mut callee = authority("std::convert::TryInto::try_into");
    callee.implementation = Some(Box::new(authority(
        "<T as std::convert::TryInto<U>>::try_into",
    )));
    pinned_callee(&callee, Kind::Borrow(4), &registry, true).unwrap();
    for mutation in 0..6 {
        let mut forged = callee.clone();
        match mutation {
            0 => forged.crate_name = "foreign".into(),
            1 => forged.definition_hash = "another method".into(),
            2 => forged.implementation = None,
            3 => forged.implementation.as_mut().unwrap().crate_hash = "another core".into(),
            4 => forged.implementation.as_mut().unwrap().definition_hash = "custom impl".into(),
            _ => {
                forged.implementation.as_mut().unwrap().implementation =
                    Some(Box::new(callee.clone()))
            }
        }
        assert!(pinned_callee(&forged, Kind::Borrow(4), &registry, true).is_err());
    }
}

#[test]
fn little_endian_interpretation_matches_independent_header_arithmetic() {
    let semantics = Kind::LittleU32.semantics();
    assert_eq!(semantics["rule"], "little-endian-u32");
    let shifts = semantics["byte_significance_bits"].as_array().unwrap();
    for lane in 0..4 {
        for byte in 0..=255u32 {
            let mut header = [0u8; 4];
            header[lane] = byte as u8;
            let interpreted = header
                .iter()
                .zip(shifts)
                .fold(0u32, |value, (byte, shift)| {
                    value | (u32::from(*byte) << shift.as_u64().unwrap())
                });
            assert_eq!(interpreted, byte * 256u32.pow(lane as u32));
            assert_eq!(interpreted, u32::from_le_bytes(header));
        }
    }
    assert_ne!(Kind::LittleU32.construct(), Kind::Native.construct());
    assert_ne!(Kind::LittleU32.semantics(), Kind::Native.semantics());
}

#[test]
fn byte_order_source_and_lengths_cannot_be_interchanged() {
    for (source, accepted) in [
        ("u32::from_le_bytes(a)", true),
        ("u32::from_ne_bytes(a)", false),
        ("u32::from_be_bytes(a)", false),
        ("u32::from_le_bytes::<u32>(a)", false),
    ] {
        let expression: syn::ExprCall = syn::parse_str(source).unwrap();
        assert_eq!(
            source::native_invocation(&expression, Kind::LittleU32),
            accepted
        );
    }
    assert!(Kind::Borrow(4).requires_checked_flow());
    assert!(Kind::UnwrapBorrow(4).requires_checked_flow());
    assert!(!Kind::Borrow(8).requires_checked_flow());
}

#[test]
fn borrowed_window_keeps_saved_formal_and_never_becomes_an_owned_copy() {
    let saved = Origin {
        parameter: 2,
        path: vec![Projection::SharedReferent],
    };
    let roots = origins::project(Kind::Borrow(4), vec![saved.clone()]);
    assert_eq!(
        roots,
        vec![Origin {
            parameter: 2,
            path: vec![
                Projection::SharedReferent,
                Projection::ByteWindow {
                    operand_position: 0,
                    length: 4
                }
            ]
        }]
    );
    assert_eq!(
        origins::project(Kind::UnwrapBorrow(4), roots.clone()),
        roots
    );
    assert_eq!(
        origins::project(Kind::Copy(4), vec![saved.clone()]),
        vec![saved]
    );
}

#[test]
fn unregistered_borrow_lengths_cannot_reuse_either_registered_contract() {
    for length in [0, 1, 2, 3, 5, 6, 7, 9, 16, u64::MAX] {
        assert!(Kind::Borrow(length).payload_capability().is_err());
        assert!(Kind::UnwrapBorrow(length).payload_capability().is_err());
    }
    assert_eq!(
        Kind::Borrow(4).payload_capability().unwrap(),
        Some(CHECKED_FLOW_CAPABILITY)
    );
    assert_eq!(
        Kind::Borrow(8).payload_capability().unwrap(),
        Some("rust.native-borrowed-byte-readers")
    );
}
