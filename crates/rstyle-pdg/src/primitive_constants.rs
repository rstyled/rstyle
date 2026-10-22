//! Pinned primitive associated constants; separate from owned local const values.
use crate::{HirNode, PathRes, data_library, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

pub const RESOLUTION: &str = "resolved_primitive_constant";
pub const CAPABILITY: &str = "rust.pinned-primitive-constants";
pub const KEY: &str = "core::num::u32::MAX";
pub const PATH: &str = "core::num::<impl u32>::MAX";
pub const SOURCE_PATH: &str = "num/uint_macros.rs";
pub const SOURCE_DIGEST: &str =
    "sha256:5525546f81f07f28c0d3bc475edf2f86770fe659b35dd4bae55c0a3b890ab12b";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveConstant {
    pub definition: data_library::Callee,
    pub primitive: String,
    pub member: String,
    pub value: String,
}

pub fn decode(resolution: &PathRes) -> Result<PrimitiveConstant, String> {
    crate::scalar_constants::decode_record(resolution, RESOLUTION)
}

pub fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<PrimitiveConstant, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || node.kind != "expr_path"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u32")
        || node.ty.as_ref().map(|ty| ty.observed.as_str()) != Some("u32")
    {
        return Err("Primitive constant requires its registered original u32 path".into());
    }

    let record = decode(
        node.res
            .as_ref()
            .ok_or("Missing primitive constant resolution")?,
    )?;
    value(&record)?;

    let registry =
        data_library::registry_for(facts).ok_or("Unpinned primitive constant compiler/target")?;
    width(
        facts.build.pointer_width.status,
        facts.build.pointer_width.value,
        registry.pointer_width,
    )?;

    data_library::verify_sysroot_roots(facts, &["core", "std"], "Primitive constant")?;
    authority(&record, registry)?;

    let expression = crate::source_evidence::original_expression(node, facts)?;
    source_path(&expression)?;
    facts
        .nominal_resolution()?
        .verify_primitive_constant_source(node, facts)?;

    Ok(record)
}

pub(crate) fn verify_document_policy(
    profile: &crate::ProfileId,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::extraction_profile::require_document(
        profile,
        CAPABILITY,
        || contains(&facts.body),
        "Primitive constant requires its registered document capability",
    )
}

fn contains(node: &HirNode) -> bool {
    node.res
        .as_ref()
        .is_some_and(|resolution| resolution.kind == RESOLUTION)
        || node.children.iter().any(contains)
}

fn value(record: &PrimitiveConstant) -> Result<(), String> {
    if record.primitive != "u32" || record.member != "MAX" || record.value != u32::MAX.to_string() {
        return Err("Primitive constant disagrees with the exact u32 maximum".into());
    }
    Ok(())
}

fn width(status: crate::FactStatus, captured: Option<u64>, reviewed: u64) -> Result<(), String> {
    if status != crate::FactStatus::Known || captured != Some(64) || reviewed != 64 {
        return Err("Primitive constant requires its reviewed known 64-bit target".into());
    }
    Ok(())
}

fn authority(record: &PrimitiveConstant, registry: &data_library::Registry) -> Result<(), String> {
    if record.definition.path != PATH
        || !registry.exact_core_definition(&record.definition, KEY)
        || registry.source_digests.get(SOURCE_PATH).map(String::as_str) != Some(SOURCE_DIGEST)
    {
        return Err("Primitive constant lacks its exact reviewed core definition/source".into());
    }
    Ok(())
}

fn source_path(expression: &syn::Expr) -> Result<(), String> {
    let syn::Expr::Path(path) = expression else {
        return Err("Primitive constant requires an original bare primitive path".into());
    };
    if path.qself.is_some()
        || !path.attrs.is_empty()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 2
        || path
            .path
            .segments
            .iter()
            .any(|segment| !segment.arguments.is_empty())
        || path.path.segments[0].ident != "u32"
        || path.path.segments[1].ident != "MAX"
    {
        return Err("Primitive constant source must be exactly u32::MAX".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(registry: &data_library::Registry) -> PrimitiveConstant {
        PrimitiveConstant {
            definition: data_library::Callee {
                path: PATH.into(),
                crate_name: "core".into(),
                crate_hash: registry.core_hash.clone(),
                definition_hash: registry.definitions[KEY].clone(),
                implementation: None,
            },
            primitive: "u32".into(),
            member: "MAX".into(),
            value: u32::MAX.to_string(),
        }
    }

    #[test]
    fn primitive_value_and_wire_never_default_missing_or_unknown_data() {
        let original = record(data_library::registry());
        value(&original).unwrap();
        let wire = serde_json::to_value(&original).unwrap();
        for key in ["definition", "primitive", "member", "value"] {
            let mut changed = wire.clone();
            changed.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<PrimitiveConstant>(changed).is_err());
        }
        for (key, replacement) in [
            ("primitive", "u64"),
            ("member", "MIN"),
            ("value", "4294967296"),
            ("value", "04294967295"),
            ("value", "-1"),
            ("value", "unknown"),
        ] {
            let mut changed = wire.clone();
            changed[key] = replacement.into();
            assert!(value(&serde_json::from_value(changed).unwrap()).is_err());
        }
        let mut changed = wire;
        changed["assumed"] = true.into();
        assert!(serde_json::from_value::<PrimitiveConstant>(changed).is_err());
    }

    #[test]
    fn primitive_authority_requires_reviewed_core_and_target_definition() {
        let registry = data_library::registry();
        let original = record(registry);
        authority(&original, registry).unwrap();
        for key in ["path", "crate", "crate_hash", "definition_hash"] {
            let mut wire = serde_json::to_value(&original).unwrap();
            wire["definition"][key] = "wrong-authority".into();
            assert!(authority(&serde_json::from_value(wire).unwrap(), registry).is_err());
        }
        let mut changed = original;
        changed.definition.implementation = Some(Box::new(changed.definition.clone()));
        assert!(authority(&changed, registry).is_err());
        assert!(authority(&record(&data_library::registries()[1]), registry).is_err());
    }

    #[test]
    fn primitive_source_syntax_and_width_are_bounded() {
        source_path(&syn::parse_str("u32::MAX").unwrap()).unwrap();
        for source in [
            "u64::MAX",
            "u32::MIN",
            "Other::MAX",
            "::u32::MAX",
            "u32::MAX()",
            "core::primitive::u32::MAX",
            "<u32 as Other>::MAX",
            "u32::MAX::<u8>",
        ] {
            assert!(
                source_path(&syn::parse_str(source).unwrap()).is_err(),
                "{source}"
            );
        }
        width(crate::FactStatus::Known, Some(64), 64).unwrap();
        for (status, captured, reviewed) in [
            (crate::FactStatus::Unknown, Some(64), 64),
            (crate::FactStatus::Known, None, 64),
            (crate::FactStatus::Known, Some(32), 64),
            (crate::FactStatus::Known, Some(64), 32),
        ] {
            assert!(width(status, captured, reviewed).is_err());
        }
    }
}
