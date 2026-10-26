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
