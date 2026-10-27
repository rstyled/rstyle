//! Precheck source topology is specified before evaluating admission.
use super::*;

const ORIGINAL: &str = "fn checked(slice: &[u8], what: &'static str) -> Result<(Id, usize), Error> { if slice.len() < Id::SIZE { return Err(Error::buffer_too_small(what)); } read(slice, what) }";

#[test]
fn checked_reader_requires_complete_precheck_then_reader_topology() {
    let syntax: syn::ItemFn = syn::parse_str(ORIGINAL).unwrap();
    assert!(syntax_shape(&syntax).is_ok());

    for (before, after) in [
        ("return Err", "audit(); return Err"),
        ("} read", "} else { audit(); } read"),
        ("} read", "} audit(); read"),
        ("read(slice, what)", "{ read(slice, what) }"),
        ("fn checked", "unsafe fn checked"),
        ("fn checked", "async fn checked"),
        ("fn checked", "const fn checked"),
        ("checked(", "checked<T>("),
        ("fn checked", "#[unknown] fn checked"),
        ("if slice.len()", "#[cfg(any())] if slice.len()"),
    ] {
        let changed: syn::ItemFn = syn::parse_str(&ORIGINAL.replace(before, after)).unwrap();
        assert!(syntax_shape(&changed).is_err(), "{before} -> {after}");
    }
}
