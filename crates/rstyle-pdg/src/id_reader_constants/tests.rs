//! Source-first layout and initializer expectations, independent of captured values.
use super::source;

#[test]
fn transparent_u32_size_requires_the_actual_original_layout_contract() {
    let original = "#[repr(transparent)] pub struct SmallIndex(u32);";
    source::transparent_u32(&syn::parse_str(original).unwrap()).unwrap();
    for source in [
        "pub struct SmallIndex(u32);",
        "#[repr(C)] pub struct SmallIndex(u32);",
        "#[repr(transparent, packed)] pub struct SmallIndex(u32);",
        "#[repr(transparent)] pub struct SmallIndex(u64);",
        "#[repr(transparent)] pub struct SmallIndex(u32, ());",
        "#[repr(transparent)] pub struct SmallIndex { field: u32 }",
        "#[repr(transparent)] pub struct SmallIndex<T>(T);",
    ] {
        let item = syn::parse_str(source).unwrap();
        assert!(source::transparent_u32(&item).is_err(), "{source}");
    }
}

#[test]
fn source_size_of_wrapper_must_not_be_rewritten_to_a_primitive_or_evaluated_literal() {
    let original = "pub const SIZE: usize = core::mem::size_of::<SmallIndex>();";
    source::original(&syn::parse_str(original).unwrap(), "SmallIndex").unwrap();
    for original in [
        "pub const SIZE: usize = core::mem::size_of::<u32>();",
        "pub const SIZE: usize = core::mem::size_of::<Other>();",
        "pub const SIZE: usize = std::mem::size_of::<SmallIndex>();",
        "pub const SIZE: usize = user::mem::size_of::<SmallIndex>();",
        "pub const SIZE: usize = core::mem::size_of::<SmallIndex>(side_effect());",
        "pub const SIZE: usize = core::mem::size_of::<SmallIndex>() + 0;",
        "pub const SIZE: usize = 4;",
        "pub const SIZE: u64 = core::mem::size_of::<SmallIndex>();",
        "pub const OTHER: usize = core::mem::size_of::<SmallIndex>();",
    ] {
        assert!(
            source::original(&syn::parse_str(original).unwrap(), "SmallIndex").is_err(),
            "{original}"
        );
    }
}

#[test]
fn associated_paths_keep_concrete_owner_member_and_no_computation() {
    for original in [
        "SmallIndex::SIZE",
        "crate::util::primitives::SmallIndex::SIZE",
    ] {
        source::owner_path(&syn::parse_str(original).unwrap()).unwrap();
    }
    for original in [
        "SIZE",
        "SmallIndex::OTHER",
        "SmallIndex::<u32>::SIZE",
        "::SmallIndex::SIZE",
        "SmallIndex::SIZE + 0",
    ] {
        assert!(
            source::owner_path(&syn::parse_str(original).unwrap()).is_err(),
            "{original}"
        );
    }
}

#[test]
fn associated_size_requires_the_selected_concrete_inherent_owner() {
    let item = syn::parse_str("impl PatternID { const SIZE: usize = SmallIndex::SIZE; }").unwrap();
    source::implementation(&item, "PatternID").unwrap();
    for original in [
        "impl Other { const SIZE: usize = SmallIndex::SIZE; }",
        "impl<T> PatternID<T> { const SIZE: usize = SmallIndex::SIZE; }",
        "impl SomeTrait for PatternID { const SIZE: usize = SmallIndex::SIZE; }",
        "unsafe impl SomeTrait for PatternID { const SIZE: usize = SmallIndex::SIZE; }",
    ] {
        assert!(
            source::implementation(&syn::parse_str(original).unwrap(), "PatternID").is_err(),
            "{original}"
        );
    }
}
