//! Source and observation adversaries; these tests grant no body/admission proof.
use super::{model, selection, source};
use serde_json::{Value, json};

const IMPL: &str = r#"
impl Usize for usize {
    fn as_u8(self) -> u8 { self as u8 }
    fn as_u16(self) -> u16 { self as u16 }
    fn as_u32(self) -> u32 { self as u32 }
    fn as_u64(self) -> u64 {
        #[cfg(debug_assertions)] { u64::try_from(self).expect("usize overflowed u64") }
        #[cfg(not(debug_assertions))] { self as u64 }
    }
}
"#;
const TRAIT: &str = "trait Usize { fn as_u8(self)->u8; fn as_u16(self)->u16; fn as_u32(self)->u32; fn as_u64(self)->u64; }";

fn source_member(bytes: &str) -> Result<source::SourceMember, String> {
    let item: syn::ItemImpl = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let Some(syn::ImplItem::Fn(method)) = item.items.last() else {
        return Err("Missing selected member".into());
    };
    source::syntax(&item, method)
}

#[test]
fn original_primitive_trait_roster_preserves_debug_and_release_body_alternatives() {
    let member = source_member(IMPL).unwrap();
    let item = syn::parse_str(TRAIT).unwrap();
    assert!(source::verify_trait(&member, &item).is_ok());
    assert_eq!(member.method_name, "as_u64");
    // The source association is deliberately not a totality/effects shortcut.
    let body_with_panic = IMPL.replace(
        "self as u64",
        "panic!(\"requires ordinary body verification\")",
    );
    assert!(source_member(&body_with_panic).is_ok());
}

#[test]
fn primitive_trait_source_rejects_other_owners_receivers_generics_and_signatures() {
    for (before, after) in [
        ("impl Usize for usize", "impl Usize for u32"),
        ("impl Usize for usize", "impl<T> Usize for usize"),
        ("impl Usize for usize", "unsafe impl Usize for usize"),
        ("impl Usize for usize", "impl !Usize for usize"),
        ("fn as_u64(self)", "fn as_u64(&self)"),
        ("fn as_u64(self)", "fn as_u64(mut self)"),
        ("fn as_u64(self)", "fn as_u64(self: usize)"),
        ("fn as_u64(self)", "fn as_u64(self, other:usize)"),
        ("fn as_u64(self)", "fn as_u64<T>(self)"),
        ("fn as_u64(self)", "async fn as_u64(self)"),
        ("fn as_u64(self)", "unsafe fn as_u64(self)"),
        ("fn as_u64(self) -> u64", "fn as_u64(self) -> u32"),
    ] {
        assert!(
            source_member(&IMPL.replace(before, after)).is_err(),
            "{after}"
        );
    }
}

#[test]
fn original_trait_declaration_must_match_the_complete_impl_without_defaults() {
    let member = source_member(IMPL).unwrap();
    for bytes in [
        TRAIT.replace("trait Usize", "trait Other"),
        TRAIT.replace("trait Usize", "trait Usize<T>"),
        TRAIT.replace("trait Usize", "trait Usize: Copy"),
        TRAIT.replace("fn as_u8(self)->u8;", ""),
        TRAIT.replace("fn as_u8(self)->u8;", "fn as_u8(self)->u8 { 0 }"),
        TRAIT.replace("fn as_u8(self)->u8;", "fn as_u8(&self)->u8;"),
        TRAIT.replace("fn as_u8(self)->u8;", "type Output;"),
        TRAIT.replace("fn as_u8(self)->u8;", "const VALUE:u8;"),
        TRAIT.replace("fn as_u8(self)->u8;", "fn as_u8(self)->u16;"),
    ] {
        let item = syn::parse_str(&bytes).unwrap();
        assert!(source::verify_trait(&member, &item).is_err(), "{bytes}");
    }
}

#[test]
fn selected_path_keeps_the_concrete_owner_and_trait_member_distinct() {
    assert!(source::selected_path("<usize as util::int::Usize>::as_u64").is_ok());
    for name in [
        "util::int::Usize::as_u64",
        "<u32 as util::int::Usize>::as_u64",
        "<usize as util::int::Usize<u32>>::as_u64",
        "<usize>::as_u64",
    ] {
        assert!(source::selected_path(name).is_err(), "{name}");
    }
}

fn node() -> crate::HirNode {
    serde_json::from_value(json!({
        "hir":{"krate":0,"owner":4,"local":8},"span":{"start":10,"end":24},
        "kind":"expr_method_call","snippet":"index.as_u64()", "ty":null,
        "res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,
        "lit":null,"param_index":null,"is_unsafe_block":false,"children":[]
    }))
    .unwrap()
}

fn inventory() -> Value {
    json!({"identity":"rstyle.rust.call-instance-observations/1","complete":true,"calls":[{
        "hir":{"krate":0,"owner":4,"local":8},"span":{"start":10,"end":24},
        "expanded":false,"status":"resolved",
        "requested":{"path":"util::int::Usize::as_u64","definition_identity":"trait-member",
            "crate":"sample","stable_crate_id":"same-build","local":true},
        "selected":{"path":"<usize as util::int::Usize>::as_u64","definition_identity":"impl-member",
            "crate":"sample","stable_crate_id":"same-build","local":true},
        "requested_arguments":[{"kind":"type","value":"display only","display_only":true,
            "identity_status":"known","identity":{"kind":"builtin","name":"usize"}}],
        "selected_arguments":[],"signature":{"display_only":true,"inputs":["usize"],"output":"u64"}
    }]})
}

fn relationship(inventory: &Value) -> Result<(), String> {
    let call: model::Call = selection::original_call(inventory, &node())?;
    selection::relationship(&call)
}

#[test]
fn raw_selected_local_implementation_cannot_be_inferred_from_requested_name_or_display() {
    let baseline = inventory();
    assert!(relationship(&baseline).is_ok());
    for (field, key, value) in [
        ("requested", "path", json!("util::int::Other::as_u64")),
        ("requested", "definition_identity", json!("impl-member")),
        ("requested", "stable_crate_id", json!("another-build")),
        ("selected", "crate", json!("another-package")),
        ("selected", "local", json!(false)),
    ] {
        let mut attack = baseline.clone();
        attack["calls"][0][field][key] = value;
        assert!(relationship(&attack).is_err());
    }
    let mut wrong_self = baseline;
    wrong_self["calls"][0]["requested_arguments"][0]["identity"]["name"] = json!("u32");
    assert!(relationship(&wrong_self).is_err());
}

#[test]
fn local_value_trait_inventory_rejects_missing_duplicate_incomplete_or_wrong_site() {
    let baseline = inventory();
    for (key, value) in [
        ("expanded", json!(true)),
        ("status", json!("unresolved")),
        ("span", json!({"start":0,"end":24})),
        ("compiled_dependency", json!({"identity":"foreign"})),
    ] {
        let mut attack = baseline.clone();
        attack["calls"][0][key] = value;
        assert!(relationship(&attack).is_err());
    }
    let mut duplicate = baseline.clone();
    duplicate["calls"]
        .as_array_mut()
        .unwrap()
        .push(baseline["calls"][0].clone());
    assert!(relationship(&duplicate).is_err());
    let mut incomplete = baseline;
    incomplete["complete"] = json!(false);
    assert!(relationship(&incomplete).is_err());
}
