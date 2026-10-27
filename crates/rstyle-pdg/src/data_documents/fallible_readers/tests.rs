//! Independent complete-source exclusions; graph authority requires real capture.
use super::*;

const ORIGINAL: &str = "pub(crate) fn try_read_u32(slice: &[u8], what: &'static str) -> Result<(u32, usize), Error> { check_slice_len(slice, size_of::<u32>(), what)?; Ok((read_u32(slice), size_of::<u32>())) }";

#[test]
fn complete_terminal_shape_preserves_question_mark_and_success_tail() {
    for original in [ORIGINAL.to_owned(), ORIGINAL.replace("u32", "u16")] {
        let syntax = syn::parse_str(&original).unwrap();
        source::syntax_roles(&syntax).unwrap();
        let renamed = original.replace("try_read_u", "other_name_u");
        source::syntax_roles(&syn::parse_str(&renamed).unwrap()).unwrap();
    }
}

#[test]
fn source_shape_cannot_certify_a_supported_prefix_or_omitted_operation() {
    for original in [ORIGINAL.to_owned(), ORIGINAL.replace("u32", "u16")] {
        for (before, after) in [
            ("{ check_slice_len", "{ side_effect(); check_slice_len"),
            ("?; Ok", "?; side_effect(); Ok"),
            ("?; Ok", "?; let hidden = 0; Ok"),
            ("?; Ok", "; Ok"),
            ("?; Ok", "?; return Ok"),
            ("pub(crate) fn", "pub(crate) unsafe fn"),
            ("(slice:", "<T>(slice:"),
            ("{ check_slice_len", "{ #[cfg(any())] check_slice_len"),
        ] {
            let changed = original.replace(before, after);
            let syntax = syn::parse_str(&changed).unwrap();
            assert!(
                source::syntax_roles(&syntax).is_err(),
                "{before} -> {after}"
            );
        }
    }
}
