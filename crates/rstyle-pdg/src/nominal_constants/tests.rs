//! Source-first arithmetic expectations, separately observed payloads, and hostile mutations.
mod authority;
mod fixture;
use super::*;
use fixture::Fixture;

#[test]
fn independent_offset_one_and_two_expectations() {
    for (offset, expected) in [("1", "2147483646"), ("2", "2147483645")] {
        let fixture = Fixture::new(offset, expected);
        assert!(verify_record(&fixture.record, &fixture).is_ok());
    }
}

#[test]
fn evaluated_payload_cannot_replace_independent_arithmetic() {
    let fixture = Fixture::new("1", "2147483646");
    for wrong in ["0", "2147483645", "2147483647", "02147483646", "4294967296"] {
        let record = ProductConstant {
            field: ScalarValue {
                ty: "u32".into(),
                value: wrong.into(),
            },
            ..fixture.record.clone()
        };
        assert!(verify_record(&record, &fixture).is_err(), "{wrong}");
    }
}

#[test]
fn observations_do_not_supply_build_or_conditional_authority() {
    let mut fixture = Fixture::new("1", "2147483646");
    fixture.deny_authority = true;
    assert!(verify_record(&fixture.record, &fixture).is_err());
    fixture.deny_authority = false;
    fixture.cfgs = vec!["target_pointer_width=\"16\"".into()];
    assert!(verify_record(&fixture.record, &fixture).is_err());
    fixture.cfgs.clear();
    assert!(verify_record(&fixture.record, &fixture).is_err());
}

#[test]
fn selected_local_core_and_tuple_identities_are_independently_required() {
    let fixture = Fixture::new("1", "2147483646");
    let mut record = fixture.record.clone();
    record.definition.definition_hash.push_str("wrong");
    assert!(verify_record(&record, &fixture).is_err());
    let mut record = fixture.record.clone();
    record
        .constructor
        .tuple_constructor
        .definition_hash
        .push_str("wrong");
    assert!(verify_record(&record, &fixture).is_err());
    let mut record = fixture.record.clone();
    record
        .initializer
        .core_constant
        .definition
        .definition_hash
        .push_str("wrong");
    assert!(verify_record(&record, &fixture).is_err());
}

#[test]
fn scalar_shape_and_original_coordinates_are_not_optional() {
    let fixture = Fixture::new("1", "2147483646");
    let mut record = fixture.record.clone();
    record.field.ty = "usize".into();
    assert!(verify_record(&record, &fixture).is_err());
    let mut record = fixture.record.clone();
    record.constructor.parameter_span.end += 1;
    assert!(verify_record(&record, &fixture).is_err());
    let mut record = fixture.record.clone();
    record.initializer.span.start += 1;
    assert!(verify_record(&record, &fixture).is_err());
    let mut record = fixture.record.clone();
    record.declaration.file = "other.rs".into();
    assert!(verify_record(&record, &fixture).is_err());
}

#[test]
fn hidden_statements_wrong_fields_and_dependencies_cannot_be_certified() {
    let fixture = Fixture::new("1", "2147483646");
    for (original, replacement) in [
        (
            "SmallIndex(index as u32)",
            "panic!(); SmallIndex(index as u32)",
        ),
        ("SmallIndex(index as u32)", "SmallIndex(0u32)"),
        ("core::i32::MAX", "core::u32::MAX"),
        ("MAX as usize - 1", "MAX as usize + 1"),
        ("index as u32", "index as u16"),
        ("new_unchecked(core", "other(core"),
    ] {
        let mut hostile = Fixture::new("1", "2147483646");
        hostile.bytes = hostile.bytes.replace(original, replacement);
        hostile.syntax = syn::parse_str(&hostile.bytes).unwrap();
        assert!(
            verify_record(&fixture.record, &hostile).is_err(),
            "{replacement}"
        );
    }
}

#[test]
fn initializer_syntax_is_checked_even_when_source_coordinates_are_refreshed() {
    for (original, replacement) in [
        ("core::i32::MAX", "core::u32::MAX"),
        ("MAX as usize - 1", "MAX as usize + 1"),
        ("MAX as usize - 1", "MAX as usize - 0"),
        ("new_unchecked(core", "other(core"),
    ] {
        let mut hostile = Fixture::new("1", "2147483646");
        hostile.replace_source(original, replacement);
        assert!(verify_record(&hostile.record, &hostile).is_err());
    }
}

#[test]
fn noncanonical_or_underflowing_offsets_remain_unsupported() {
    for offset in ["0x1", "1u32", "4_294_967_295", "4294967295"] {
        let fixture = Fixture::new(offset, "2147483646");
        assert!(
            verify_record(&fixture.record, &fixture).is_err(),
            "{offset}"
        );
    }
}

#[test]
fn constructor_body_and_signature_are_checked_after_coordinates_are_refreshed() {
    for (original, replacement) in [
        (
            "SmallIndex(index as u32)",
            "panic!(); SmallIndex(index as u32)",
        ),
        ("SmallIndex(index as u32)", "SmallIndex(0u32)"),
        ("index as u32", "index as u16"),
        ("index: usize", "mut index: usize"),
        ("const fn new_unchecked", "const unsafe fn new_unchecked"),
    ] {
        let mut hostile = Fixture::new("1", "2147483646");
        hostile.replace_source(original, replacement);
        assert!(
            verify_record(&hostile.record, &hostile).is_err(),
            "{replacement}"
        );
    }
}

#[test]
fn original_node_type_resolution_and_actual_source_remain_required() {
    let fixture = Fixture::new("1", "2147483646");
    let mut node = path_node(&fixture.record);
    assert_eq!(verify(&node, &fixture).unwrap().field.value, "2147483646");
    node.ty.as_mut().unwrap().normalized = Some("u32".into());
    assert!(verify(&node, &fixture).is_err());
    let mut node = path_node(&fixture.record);
    node.snippet = "Other::MAX".into();
    assert!(verify(&node, &fixture).is_err());
    let mut node = path_node(&fixture.record);
    node.res.as_mut().unwrap().description = "{}".into();
    assert!(verify(&node, &fixture).is_err());
}

fn path_node(record: &ProductConstant) -> crate::HirNode {
    crate::HirNode {
        hir: crate::HirRef {
            krate: 0,
            owner: 1,
            local: 2,
        },
        kind: "expr_path".into(),
        span: crate::ByteSpan { start: 0, end: 15 },
        snippet: "SmallIndex::MAX".into(),
        ty: Some(crate::ResolvedTy {
            normalized: Some(crate::data_types::token(&record.ty).unwrap()),
            observed: "SmallIndex".into(),
            spelling: String::new(),
        }),
        res: Some(crate::PathRes {
            kind: RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(record).unwrap(),
        }),
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: Vec::new(),
    }
}

#[test]
fn conflicting_active_constants_and_hidden_attributes_fail() {
    for (from, to) in [
        (
            "#[cfg(target_pointer_width = \"16\")]",
            "#[cfg(target_pointer_width = \"64\")]",
        ),
        ("#[inline]", "#[some_unproved_attribute]"),
    ] {
        let mut hostile = Fixture::new("1", "2147483646");
        hostile.replace_source(from, to);
        assert!(verify_record(&hostile.record, &hostile).is_err());
    }
}

#[test]
fn nominal_product_shape_cannot_collapse_to_scalar_or_other_fields() {
    let fixture = Fixture::new("1", "2147483646");
    let mut record = fixture.record.clone();
    record.ty = crate::concrete_types::TypeTree::Scalar("u32".into());
    assert!(verify_record(&record, &fixture).is_err());
    let mut record = fixture.record.clone();
    if let crate::concrete_types::TypeTree::Struct { fields, .. } = &mut record.ty {
        fields[0].ty = crate::concrete_types::TypeTree::Scalar("usize".into());
    }
    assert!(verify_record(&record, &fixture).is_err());
}

#[test]
fn unregistered_or_extended_observations_cannot_gain_authority() {
    let fixture = Fixture::new("1", "2147483646");
    let mut record = fixture.record.clone();
    record.identity = "rstyle.rust.product-constant/0".into();
    assert!(verify_record(&record, &fixture).is_err());
    let mut value = serde_json::to_value(&fixture.record).unwrap();
    value["unchecked_purity"] = serde_json::json!(true);
    let resolution = crate::PathRes {
        kind: RESOLUTION.into(),
        target: None,
        description: serde_json::to_string(&value).unwrap(),
    };
    assert!(decode(&resolution).is_err());
}
