//! Independently specified source-family expectations, never admission authority.
use super::*;

const ORIGINAL: &str = "pub(crate) fn check_slice_len<T>(\n    slice: &[T],\n    at_least_len: usize,\n    what: &'static str,\n) -> Result<(), DeserializeError> {\n    if slice.len() < at_least_len {\n        return Err(DeserializeError::buffer_too_small(what));\n    }\n    Ok(())\n}";

fn text(source: &str, site: ByteSpan) -> &str {
    &source[site.start as usize..site.end as usize]
}

#[test]
fn original_complete_source_has_exact_roles_and_multiline_offsets() {
    let actual = verify(ORIGINAL).unwrap();
    assert_eq!(actual.function, "check_slice_len");
    assert_eq!(actual.generic, "T");
    assert_eq!(text(ORIGINAL, actual.generic_span), "T");
    assert_eq!(text(ORIGINAL, actual.condition.receiver), "slice");

    assert_eq!(text(ORIGINAL, actual.condition.minimum), "at_least_len");
    assert_eq!(text(ORIGINAL, actual.condition.length_method), "len");
    assert_eq!(text(ORIGINAL, actual.condition.operator), "<");
    assert_eq!(text(ORIGINAL, actual.error.what), "what");

    assert_eq!(
        text(ORIGINAL, actual.error.constructor_callee),
        "DeserializeError::buffer_too_small"
    );
    assert_eq!(text(ORIGINAL, actual.error.err_callee), "Err");
    assert_eq!(text(ORIGINAL, actual.success.callee), "Ok");
    assert_eq!(text(ORIGINAL, actual.success.unit), "()");
}

#[test]
fn names_are_source_roles_and_grant_no_nominal_or_constructor_authority() {
    let source = ORIGINAL
        .replace("check_slice_len", "different_name")
        .replace("at_least_len", "required")
        .replace("slice", "bytes")
        .replace("what", "description")
        .replace("DeserializeError", "errors::Problem")
        .replace("buffer_too_small", "make_problem")
        .replace("<T>", "<Element>")
        .replace("[T]", "[Element]");
    let checked = verify(&source).unwrap();
    assert_eq!(checked.function, "different_name");
    assert_eq!(checked.generic, "Element");

    assert_eq!(checked.parameters[0].name, "bytes");
    assert_eq!(checked.parameters[1].name, "required");
    assert_eq!(checked.parameters[2].name, "description");
    assert_eq!(
        text(&source, checked.error.constructor_callee),
        "errors::Problem::make_problem"
    );
}

#[test]
fn literal_function_metadata_is_preserved_but_other_attributes_reject() {
    let source = format!("#[doc = \"π length check\"]\n#[inline]\n{ORIGINAL}");
    let checked = verify(&source).unwrap();
    assert_eq!(text(&source, checked.function_name), "check_slice_len");
    assert_eq!(text(&source, checked.condition.receiver), "slice");

    for prefix in [
        "#[inline(always)]",
        "#[cfg(any())]",
        "#[doc = include_str!(\"x\")]",
        "#[allow(dead_code)]",
    ] {
        assert!(
            verify(&format!("{prefix}\n{ORIGINAL}")).is_err(),
            "{prefix}"
        );
    }
    for (before, after) in [
        ("slice.len()", "#[cfg(any())] slice.len()"),
        ("return Err", "#[allow(unreachable_code)] return Err"),
    ] {
        assert!(verify(&ORIGINAL.replace(before, after)).is_err());
    }
}

#[test]
fn changed_comparison_and_argument_roles_are_not_a_supported_prefix() {
    for (before, after) in [
        (" < ", " <= "),
        (" < ", " > "),
        (" < ", " == "),
        ("slice.len() < at_least_len", "at_least_len < slice.len()"),
        ("slice.len()", "slice.len(0)"),
        ("slice.len()", "slice.len::<T>()"),
        ("slice.len()", "slice.is_empty()"),
        (" < at_least_len", " < at_least_len + 0"),
        ("buffer_too_small(what)", "buffer_too_small(\"fixed\")"),
        ("buffer_too_small(what)", "buffer_too_small()"),
        ("buffer_too_small(what)", "buffer_too_small(what, what)"),
        ("Err(DeserializeError", "Ok(DeserializeError"),
        ("Ok(())", "Ok(0)"),
    ] {
        assert!(
            verify(&ORIGINAL.replace(before, after)).is_err(),
            "{before} -> {after}"
        );
    }
}

#[test]
fn extra_rebinding_operations_and_neighboring_items_reject() {
    for (before, after) in [
        ("    if slice.len()", "    let slice = &[]; if slice.len()"),
        ("    if slice.len()", "    side_effect(); if slice.len()"),
        ("        return Err", "        side_effect(); return Err"),
        ("    Ok(())", "    side_effect(); Ok(())"),
        ("    Ok(())", "    fn hidden() {} Ok(())"),
        ("    Ok(())", "    return Ok(());"),
        ("return Err(DeserializeError", "return Err(OtherError"),
        ("    }\n    Ok", "    } else { side_effect(); }\n    Ok"),
    ] {
        assert!(
            verify(&ORIGINAL.replace(before, after)).is_err(),
            "{before} -> {after}"
        );
    }
    assert!(verify(&format!("{ORIGINAL}\nfn omitted() {{}}")).is_err());
}

#[test]
fn generic_interface_mutability_and_static_lifetime_are_exact() {
    for (before, after) in [
        ("<T>", "<T: Copy>"),
        ("<T>", "<T: ?Sized>"),
        ("<T>", "<T = u8>"),
        ("<T>", "<T, U>"),
        ("<T>", "<'a, T>"),
        ("<T>", "<const T: usize>"),
        (
            "Result<(), DeserializeError> {",
            "Result<(), DeserializeError> where T: Copy {",
        ),
        ("&[T]", "&[u8]"),
        ("&[T]", "&mut [T]"),
        ("&[T]", "&'static [T]"),
        ("&'static str", "&str"),
        ("&'static str", "&'static mut str"),
        ("slice: &[T]", "mut slice: &[T]"),
        ("what: &'static str", "ref what: &'static str"),
        ("at_least_len: usize", "at_least_len: u32"),
        ("fn check", "unsafe fn check"),
        ("fn check", "const fn check"),
        ("fn check", "async fn check"),
        ("Result<(),", "Result<bool,"),
    ] {
        assert!(
            verify(&ORIGINAL.replace(before, after)).is_err(),
            "{before} -> {after}"
        );
    }
}
