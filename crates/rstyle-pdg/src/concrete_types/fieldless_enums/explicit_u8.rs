//! Bounded source grammar; complete lexical repr proof remains distinct from
//! rustc item snippets, which may omit outer attributes.
pub const CAPABILITY: &str = "rust.explicit-u8-fieldless-enums";

pub fn observation(source: &str) -> Result<Vec<(String, i64)>, String> {
    super::parse_with(source, checked)
}

pub fn declaration(item: &syn::ItemEnum) -> Result<Vec<(String, i64)>, String> {
    if item.attrs.iter().filter(|attr| repr_u8(&attr.meta)).count() != 1 {
        return Err("Explicit u8 enum requires one original repr(u8) attribute".into());
    }
    checked(item, 0)
}

fn checked(item: &syn::ItemEnum, source_len: usize) -> Result<Vec<(String, i64)>, String> {
    let mut bounded = item.clone();
    remove_unique(
        &mut bounded.attrs,
        repr_u8,
        "Explicit u8 enum repeats representation attributes",
    )?;
    for variant in &mut bounded.variants {
        remove_unique(
            &mut variant.attrs,
            variant_expect,
            "Explicit u8 enum repeats variant expectation attributes",
        )?;
    }

    let values = super::variants(&bounded, source_len)?;
    if values
        .iter()
        .any(|(_, value)| u8::try_from(*value).is_err())
    {
        return Err("Explicit u8 enum discriminant is outside u8".into());
    }
    Ok(values)
}

fn remove_unique(
    attributes: &mut Vec<syn::Attribute>,
    matches: impl Fn(&syn::Meta) -> bool,
    error: &str,
) -> Result<(), String> {
    let occurrences = attributes
        .iter()
        .filter(|attribute| matches(&attribute.meta))
        .count();
    if occurrences == 0 {
        return Ok(());
    }
    if occurrences > 1 {
        return Err(error.into());
    }

    attributes.retain(|attribute| !matches(&attribute.meta));
    Ok(())
}

pub fn repr_u8(attribute: &syn::Meta) -> bool {
    crate::source_evidence::single_identifier_attribute(attribute, "repr", "u8")
}

pub fn variant_expect(attribute: &syn::Meta) -> bool {
    crate::source_evidence::single_identifier_attribute(attribute, "expect", "missing_docs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independently_checks_u8_endpoints_and_implicit_values() {
        let source = "#[repr(u8)] #[derive(Clone, Copy)] enum E { #[expect(missing_docs)] A = 0, B = 254, C }";
        let item = syn::parse_str(source).unwrap();
        assert_eq!(
            declaration(&item).unwrap(),
            vec![("A".into(), 0), ("B".into(), 254), ("C".into(), 255)]
        );
        assert!(super::super::fieldless_variants(source).is_err());
        assert!(observation("enum E { #[expect(missing_docs)] A = 1 }").is_ok());
        assert!(declaration(&syn::parse_str("enum E { A = 1 }").unwrap()).is_err());
    }

    #[test]
    fn complete_source_rejects_altered_layout_presence_and_values() {
        for source in [
            "#[repr(C)] enum E { A }",
            "#[repr(u8, C)] enum E { A }",
            "#[repr(u8)] #[repr(u8)] enum E { A }",
            "#[repr(u8)] #[repr(align(8))] enum E { A }",
            "#[repr(u8)] enum E { A = -1 }",
            "#[repr(u8)] enum E { A = 255, B }",
            "#[repr(u8)] enum E { A = 1, B = 1 }",
            "#[repr(u8)] enum E { A(u8) }",
            "#[repr(u8)] enum E<T> { A }",
            "#[repr(u8)] enum E { #[cfg(any())] A, B }",
            "#[repr(u8)] enum E { #[expect(other)] A }",
            "#[repr(u8)] enum E { #[expect(missing_docs, reason = \"x\")] A }",
            "#[repr(u8)] enum E { #[expect = \"missing_docs\"] A }",
            "#[repr(u8)] enum E { #[expect(missing_docs)] #[expect(missing_docs)] A }",
        ] {
            let item = syn::parse_str(source).unwrap();
            assert!(declaration(&item).is_err(), "{source}");
        }
    }
}
