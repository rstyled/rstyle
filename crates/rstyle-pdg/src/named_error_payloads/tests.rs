use super::*;

#[test]
fn explicit_static_spelling_is_independent_of_erased_shared_str() {
    assert!(source::static_type(&syn::parse_str::<syn::Type>("&'static str").unwrap()).is_ok());
    for ty in [
        "&str",
        "&'a str",
        "&'static mut str",
        "&'static String",
        "&'static [u8]",
        "&'static Alias",
        "&'static core::primitive::str",
    ] {
        assert!(
            source::static_type(&syn::parse_str::<syn::Type>(ty).unwrap()).is_err(),
            "{ty}"
        );
    }
}

fn syntax(source: &str) -> syn::ExprStruct {
    syn::parse_str(source).unwrap()
}

#[test]
fn named_field_slots_retain_source_order_and_raw_identifier_identity() {
    assert_eq!(
        source::slots(
            &syntax("Error::Fault { second: b, first: a }"),
            &["first", "second"]
        )
        .unwrap(),
        [1, 0]
    );
    assert_eq!(
        source::slots(&syntax("crate::Error::Fault { r#type: what }"), &["type"]).unwrap(),
        [0]
    );
    assert_eq!(
        source::slots(&syntax("Error::Fault { what }"), &["what"]).unwrap(),
        [0]
    );
}

#[test]
fn missing_duplicate_unknown_update_and_generic_fields_remain_closed() {
    for expression in [
        "Error::Fault {}",
        "Error::Fault { what, what }",
        "Error::Fault { other: what }",
        "Error::Fault { what, ..saved }",
        "Error::<u8>::Fault { what }",
        "Error::Fault::<u8> { what }",
        "Fault { what }",
        "Error::Fault { #[cfg(any())] what }",
    ] {
        assert!(
            source::slots(&syntax(expression), &["what"]).is_err(),
            "{expression}"
        );
    }
}

#[test]
fn source_signature_does_not_invent_generic_or_unsafe_callable_permission() {
    for signature in [
        "fn make(what: &'static str) {}",
        "pub(crate) fn renamed(what: &'static str) {}",
    ] {
        assert!(source::signature(&syn::parse_str(signature).unwrap()).is_ok());
    }
    for signature in [
        "const fn make(what: &'static str) {}",
        "unsafe fn make(what: &'static str) {}",
        "async fn make(what: &'static str) {}",
        "fn make<'a>(what: &'a str) {}",
        "extern \"C\" fn make(what: &'static str) {}",
        "fn make<T>(what: &'static str) {}",
    ] {
        assert!(
            source::signature(&syn::parse_str(signature).unwrap()).is_err(),
            "{signature}"
        );
    }
}

#[test]
fn scalar_payload_spelling_does_not_admit_aliases_or_other_widths() {
    let token = TypeToken::Scalar("u32".into());
    assert!(source::payload_type(&syn::parse_str("u32").unwrap(), &token).is_ok());
    for spelling in [
        "u64",
        "usize",
        "Alias",
        "core::primitive::u32",
        "&u32",
        "(u32, u32)",
    ] {
        assert!(
            source::payload_type(&syn::parse_str(spelling).unwrap(), &token).is_err(),
            "{spelling}"
        );
    }
    for token in [
        TypeToken::Scalar("u64".into()),
        TypeToken::Scalar("usize".into()),
    ] {
        assert!(source::payload_type(&syn::parse_str("u32").unwrap(), &token).is_err());
    }
}

#[test]
fn owned_error_grammar_requires_generated_provenance_and_exact_scalar_child() {
    // These independently constructed shapes provide no cleanup or admission proof.
    let (original, child) = owned_error_fixture();
    assert!(owned::shape(&original));
    let ordinary = TypeTree::Struct {
        nominal: original.nominal_identity().unwrap().clone(),
        source: "struct Wrapper(Error);".into(),
        fields: original.nominal_fields().cloned().collect(),
    };
    assert!(!owned::shape(&ordinary));
    for change in 0..4 {
        let mut changed = original.clone();
        let TypeTree::GeneratedStruct { fields, .. } = &mut changed else {
            unreachable!()
        };
        match change {
            0 => fields[0].name = "other".into(),
            1 => fields[0].position = 1,
            2 => fields[0].ty = TypeTree::Shared(Box::new(child.clone())),
            _ => fields[0].ty = TypeTree::Scalar("u64".into()),
        }
        assert!(!owned::shape(&changed), "change {change}");
    }
    assert!(!owned::shape(&child));
}

fn owned_error_fixture() -> (TypeTree, TypeTree) {
    let nominal = Nominal {
        source_identity: Some("package".into()),
        package_name: "fixture".into(),
        package_version: "1".into(),
        manifest_digest: crate::source_identity::checksum(b"manifest"),
        definition_path: "Error".into(),
        declaration_digest: crate::source_identity::checksum(b"declaration"),
    };
    let leaf = crate::concrete_types::Field {
        position: 0,
        name: "attempted".into(),
        declaration: "u64".into(),
        ty: TypeTree::Scalar("u64".into()),
    };
    let child = TypeTree::Struct {
        nominal: nominal.clone(),
        source: "struct Error { attempted: u64 }".into(),
        fields: vec![leaf],
    };
    let field = crate::concrete_types::Field {
        position: 0,
        name: "0".into(),
        declaration: "Error".into(),
        ty: child.clone(),
    };
    let definition = crate::generated_sources::capture::Definition {
        path: "Wrapper".into(),
        definition_identity: "definition".into(),
        crate_name: "fixture".into(),
        stable_crate_id: "crate".into(),
        local: true,
    };
    let original = TypeTree::GeneratedStruct {
        nominal: nominal.clone(),
        template: "struct Wrapper(Error);".into(),
        definition: Box::new(definition),
        fields: vec![field.clone()],
    };
    (original, child)
}
