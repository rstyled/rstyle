//! Pinned, concrete slice-to-array / Result / native-byte call semantics.
//! Each source call remains a distinct typed operation; failures are not erased.

mod semantics;
mod source;
mod types;

use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};
use serde_json::Value;

pub(crate) const KEY: &str = "rstyle.data:byte-call";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Copy,
    Unwrap,
    Native,
}

pub(crate) fn kind(node: &HirNode) -> Option<Kind> {
    match node.dispatch.as_ref()?.op.as_str() {
        "std::convert::TryInto::try_into" => Some(Kind::Copy),
        "std::result::Result::<T, E>::unwrap" => Some(Kind::Unwrap),
        "core::num::<impl u32>::from_ne_bytes" => Some(Kind::Native),
        _ => None,
    }
}

impl Kind {
    pub fn construct(self) -> &'static str {
        match self {
            Self::Copy => "data.bytes.copy-result",
            Self::Unwrap => "data.bytes.unwrap",
            Self::Native => "data.bytes.native-u32",
        }
    }

    fn definition(self) -> u64 {
        match self {
            Self::Copy => 6122392695558844466,
            Self::Unwrap => 9871469522270606756,
            Self::Native => 1786064518753250710,
        }
    }

    pub fn semantics(self) -> Value {
        semantics::value(self)
    }
}

pub(crate) fn enabled(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::ExtractionProfile::from_facts(facts)
        .is_ok_and(|profile| profile.supports("rust.slice-array-byte-calls"))
}

pub(crate) fn resolve(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Option<Summary>, String> {
    let Some(kind) = kind(node).filter(|_| enabled(facts)) else {
        return Ok(None);
    };
    verify(node, facts)?;
    let dispatch = node.dispatch.as_ref().ok_or("Missing byte-call dispatch")?;
    Ok(Some(Summary {
        id: format!("rstyle.{}", kind.construct()),
        version: "1".into(),
        callee_path: dispatch.op.clone(),
        callee_crate: "core".into(),
        argument_types: dispatch.operand_tys.clone(),
        result_type: dispatch.result_ty.clone(),
        memory_effects: kind == Kind::Copy,
        mutates_arguments: false,
        may_panic: kind == Kind::Unwrap,
        always_returns: kind != Kind::Unwrap,
        overflow_behavior: "not_applicable".into(),
    }))
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Kind, String> {
    let kind = kind(node)
        .filter(|_| enabled(facts))
        .ok_or("Unregistered byte call")?;
    authority(node, kind, facts)?;
    source::verify(node, kind, facts)?;
    types::verify(node, kind, facts)?;

    Ok(kind)
}

fn authority(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts).ok_or("Unpinned byte-call build")?;
    if registry.target != "aarch64-unknown-linux-gnu" {
        return Err("Byte-call authority is not registered for this target".into());
    }
    let callee = crate::data_library::callee(node)?;
    let hash = |value| format!("DefPathHash(Fingerprint(5614226571623188065, {value}))");
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || callee.definition_hash != hash(kind.definition())
        || node
            .dispatch
            .as_ref()
            .is_none_or(|dispatch| dispatch.op != callee.path)
    {
        return Err("Byte call disagrees with pinned core definition".into());
    }

    if kind == Kind::Copy {
        let selected = callee
            .implementation
            .ok_or("Missing selected slice conversion implementation")?;
        if selected.crate_name != "core"
            || selected.crate_hash != registry.core_hash
            || selected.definition_hash != hash(11163084439670324212)
            || selected.path != "<T as std::convert::TryInto<U>>::try_into"
            || selected.implementation.is_some()
        {
            return Err("Slice conversion lacks the pinned blanket TryInto implementation".into());
        }
    } else if callee.implementation.is_some() {
        return Err("Unexpected trait implementation on inherent byte call".into());
    }
    Ok(())
}

pub(crate) fn locations(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    kind: Kind,
) -> Result<Vec<crate::field_locations::derived::Located>, String> {
    if kind != Kind::Copy {
        return Ok(Vec::new());
    }
    let receiver = &node.children[0];
    let base = if crate::data_subslices::candidate(receiver, facts) {
        crate::data_subslices::verify(receiver, facts)?.base
    } else {
        receiver
    };
    let roots = crate::data_origins::origins(base, facts)?;
    let mut locations = crate::field_locations::derived::referents(base, facts, roots)?;
    for location in &mut locations {
        location
            .path
            .push(crate::field_locations::Projection::ByteWindow {
                operand_position: 0,
                length: 4,
            });
        location.value = types::array();
    }

    if locations.is_empty() {
        return Err("Byte copy lacks complete formal origins".into());
    }
    Ok(locations)
}
