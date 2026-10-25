use super::*;

const IMPL: &str = r#"
impl U32 for u32 {
    fn as_usize(self) -> usize {
        #[cfg(debug_assertions)] { usize::try_from(self).expect("u32 overflowed usize") }
        #[cfg(not(debug_assertions))] { self as usize }
    }
    fn low_u8(self) -> u8 { self as u8 }
    fn low_u16(self) -> u16 { self as u16 }
    fn high_u16(self) -> u16 { (self >> 16) as u16 }
}
"#;
const TRAIT: &str = "trait U32 { fn as_usize(self)->usize; fn low_u8(self)->u8; fn low_u16(self)->u16; fn high_u16(self)->u16; }";

fn member(bytes: &str) -> Result<source::SourceMember, String> {
    let item: syn::ItemImpl = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let Some(syn::ImplItem::Fn(method)) = item.items.first() else {
        return Err("Missing selected member".into());
    };
    source::syntax(&item, method)
}

pub(in crate::local_value_trait_calls) fn inventory() -> Value {
    let mut value = super::inventory();
    let call = &mut value["calls"][0];
    call["requested"]["path"] = json!("util::int::U32::as_usize");
    call["selected"]["path"] = json!("<u32 as util::int::U32>::as_usize");
    call["requested_arguments"][0]["identity"]["name"] = json!("u32");
    call["signature"]["inputs"] = json!(["u32"]);
    call["signature"]["output"] = json!("usize");
    value
}

#[test]
fn original_complete_u32_trait_retains_both_bodies_and_all_siblings() {
    let item = syn::parse_str(TRAIT).unwrap();
    let checked = member(IMPL).unwrap();
    assert_eq!(checked.method_name, "as_usize");
    source::verify_trait(&checked, &item).unwrap();
    let effectful = IMPL.replace("self as usize", "panic!(\"independently verify body\")");
    source::verify_trait(&member(&effectful).unwrap(), &item).unwrap();
    assert!(source::selected_path("<u32 as util::int::U32>::as_usize", Contract::U32Usize).is_ok());
    assert!(
        source::selected_path("<usize as util::int::U32>::as_usize", Contract::U32Usize).is_err()
    );
}

#[test]
fn u32_source_rejects_other_routes_and_incomplete_or_changed_trait_rosters() {
    for (before, after) in [
        ("for u32", "for u16"),
        ("for u32", "for usize"),
        ("-> usize", "-> u64"),
        ("as_usize(self)", "as_usize(&self)"),
        ("impl U32", "impl<T> U32"),
        ("impl U32", "impl !U32"),
        ("fn low_u8(self) -> u8 { self as u8 }", "const X:u8=0;"),
    ] {
        assert!(member(&IMPL.replace(before, after)).is_err(), "{after}");
    }
    let checked = member(IMPL).unwrap();
    for changed in [
        TRAIT.replace("trait U32", "trait Other"),
        TRAIT.replace("fn low_u8(self)->u8;", ""),
        TRAIT.replace("fn low_u8(self)->u8;", "fn low_u8(self)->u8 { 0 }"),
        TRAIT.replace("fn high_u16(self)->u16;", "fn high_u16(self)->u32;"),
    ] {
        assert!(source::verify_trait(&checked, &syn::parse_str(&changed).unwrap()).is_err());
    }
}

#[test]
fn requested_structural_self_selected_owner_and_signature_must_agree() {
    let baseline = inventory();
    relationship(&baseline).unwrap();
    for (field, key, value) in [
        (
            "selected",
            "path",
            json!("<usize as util::int::U32>::as_usize"),
        ),
        ("requested", "path", json!("util::int::Other::as_usize")),
        ("signature", "inputs", json!(["usize"])),
        ("signature", "output", json!("u64")),
        ("signature", "display_only", json!(false)),
    ] {
        let mut changed = baseline.clone();
        changed["calls"][0][field][key] = value;
        assert!(relationship(&changed).is_err(), "{field}.{key}");
    }
    let mut wrong_self = baseline;
    wrong_self["calls"][0]["requested_arguments"][0]["identity"]["name"] = json!("usize");
    assert!(relationship(&wrong_self).is_err());
}

#[test]
fn original_usize_route_does_not_gain_usize_sibling_results() {
    assert!(
        super::source_member(&super::IMPL.replace("as_u8(self) -> u8", "as_u8(self) -> usize"))
            .is_err()
    );
    assert_ne!(
        Contract::U32Usize.capability(),
        Contract::UsizeU64.capability()
    );
    assert_eq!(Contract::UsizeU64.capability(), super::super::CAPABILITY);
}

#[test]
fn new_route_keeps_all_historical_document_policies_closed() {
    let current = crate::extraction_profile::ExtractionProfile::DataV3.policy();
    assert!(crate::extraction_profile::policy_supports(
        &current,
        Contract::U32Usize.capability()
    ));
    let historical = crate::ProfileId {
        id: current.id.clone(),
        version: "3.113.0".into(),
    };
    assert!(!crate::extraction_profile::policy_supports(
        &historical,
        Contract::U32Usize.capability()
    ));
    assert!(crate::extraction_profile::policy_supports(
        &current,
        Contract::UsizeU64.capability()
    ));
    // Superseded Data-v3 revisions are unregistered, rather than upgraded.
    assert!(!crate::extraction_profile::registered(&historical));
}
