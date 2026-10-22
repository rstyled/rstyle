use super::*;

#[test]
fn original_concrete_byte_owners_and_required_shared_members_have_bounded_shapes() {
    for source in [
        "struct Bytes(u8);",
        "struct Bytes(u8,u8);",
        "struct Bytes(u8,u8,u8);",
    ] {
        scalar_structure(&syn::parse_str(source).unwrap()).unwrap();
    }
    let item: syn::ItemTrait = syn::parse_str(
        "trait Local { fn prefix(&self, haystack: &[u8], span: Span) -> Option<Span>; }",
    )
    .unwrap();
    trait_header(&item).unwrap();
    let syn::TraitItem::Fn(method) = &item.items[0] else {
        panic!()
    };
    required_method(method).unwrap();
}

#[test]
fn local_trait_source_identity_cannot_follow_reexports_aliases_or_duplicate_names() {
    let file = syn::parse_file("trait Local { fn prefix(&self); }").unwrap();
    assert!(direct_trait(&file.items, "Local").is_ok());
    for source in [
        "use external::Hidden as Local;",
        "type Local = external::Hidden;",
        "trait Local {} struct Local;",
        "trait Local {} trait Local {}",
        "macro_rules! declare { () => { trait Local {} }; } declare!();",
    ] {
        let file = syn::parse_file(source).unwrap();
        assert!(direct_trait(&file.items, "Local").is_err(), "{source}");
    }
}

#[test]
fn generic_unsafe_auto_traits_and_default_methods_do_not_receive_member_authority() {
    for source in [
        "trait Local<T> {}",
        "trait Local where Self: Send {}",
        "unsafe trait Local {}",
        "auto trait Local {}",
    ] {
        assert!(
            trait_header(&syn::parse_str(source).unwrap()).is_err(),
            "{source}"
        );
    }
    for source in [
        "fn prefix(&self) {}",
        "fn prefix<T>(&self);",
        "fn prefix(&mut self);",
        "fn prefix(self);",
        "unsafe fn prefix(&self);",
        "fn prefix(&self, x: [u8; { hidden!(); 1 }]);",
        "fn prefix(&self, x: hidden!());",
    ] {
        assert!(
            required_method(&syn::parse_str(source).unwrap()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn incomplete_generic_aliased_or_effectful_owner_fields_remain_unsupported() {
    for source in [
        "struct Bytes;",
        "struct Bytes<T>(T);",
        "struct Bytes(Alias);",
        "struct Bytes(core::cell::Cell<u8>);",
        "struct Bytes([u8; { hidden!(); 1 }]);",
        "struct Bytes(&'static u8);",
    ] {
        assert!(
            scalar_structure(&syn::parse_str(source).unwrap()).is_err(),
            "{source}"
        );
    }
}
