use super::*;

#[test]
fn enum_family_rejects_noncopy_drop_reference_generic_and_guarded_variants() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub enum NonCopy { Byte(u8), End(u16) }
        pub fn noncopy(value: NonCopy) -> u8 { match value { NonCopy::Byte(b) => b, NonCopy::End(_) => 0 } }
        pub enum Dropped { Byte(u8), End(u16) }
        impl Drop for Dropped { fn drop(&mut self) {} }
        pub fn dropped(value: Dropped) -> u8 { match value { Dropped::Byte(b) => b, Dropped::End(_) => 0 } }
        #[derive(Clone, Copy)] pub enum Borrowed<'a> { Bytes(&'a [u8]), End(u16) }
        pub fn borrowed(value: Borrowed<'_>) -> u8 { match value { Borrowed::Bytes(b) => b[0], Borrowed::End(_) => 0 } }
        #[derive(Clone, Copy)] pub enum Generic<T> { Byte(T), End(u16) }
        pub fn generic(value: Generic<u8>) -> u8 { match value { Generic::Byte(b) => b, Generic::End(_) => 0 } }
        #[derive(Clone, Copy)] pub enum Scalar { Byte(u8), End(u16) }
        pub fn guarded(value: Scalar) -> u8 { match value { Scalar::Byte(b) if b > 0 => b, Scalar::Byte(_) => 0, Scalar::End(_) => 0 } }
        pub fn discard_call() { let _ = noncopy(NonCopy::Byte(1)); }
        pub fn discard_drop(value: Dropped) { let _ = value; }
    "#).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for name in [
        "noncopy",
        "dropped",
        "borrowed",
        "generic",
        "guarded",
        "discard_call",
        "discard_drop",
    ] {
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(function["outcome"], "excluded", "{function}");
        assert!(!function["reasons"].as_array().unwrap().is_empty());
    }
}
