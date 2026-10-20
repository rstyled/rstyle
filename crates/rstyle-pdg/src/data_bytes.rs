//! Pinned, concrete slice-to-array / Result / native-byte call semantics.
//! Each source call remains a distinct typed operation; failures are not erased.

mod origins;
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
    Borrow,
    UnwrapBorrow,
    NativeUsize,
}

pub(crate) fn kind(node: &HirNode) -> Option<Kind> {
    match node.dispatch.as_ref()?.op.as_str() {
        "std::convert::TryInto::try_into" => Some(if borrowed_result(node) {
            Kind::Borrow
        } else {
            Kind::Copy
        }),
        "std::result::Result::<T, E>::unwrap" => {
            Some(if node.children.first().is_some_and(borrowed_result) {
                Kind::UnwrapBorrow
            } else {
                Kind::Unwrap
            })
        }
        "core::num::<impl u32>::from_ne_bytes" => Some(Kind::Native),
        "core::num::<impl usize>::from_ne_bytes" => Some(Kind::NativeUsize),
        _ => None,
    }
}

fn borrowed_result(node: &HirNode) -> bool {
    crate::data_aggregates::token(node)
        .ok()
        .and_then(|token| crate::data_types::parse(token).ok())
        .is_some_and(|ty| {
            matches!(
                ty,
                crate::data_types::TypeToken::SliceArrayResult { borrowed: true, .. }
            )
        })
}

impl Kind {
    pub fn construct(self) -> &'static str {
        match self {
            Self::Copy => "data.bytes.copy-result",
            Self::Unwrap => "data.bytes.unwrap",
            Self::Native => "data.bytes.native-u32",
            Self::Borrow => "data.bytes.borrow-result",
            Self::UnwrapBorrow => "data.bytes.unwrap-borrow",
            Self::NativeUsize => "data.bytes.native-usize",
        }
    }

    pub fn native(self) -> bool {
        matches!(self, Self::Native | Self::NativeUsize)
    }

    pub fn conversion(self) -> bool {
        matches!(self, Self::Copy | Self::Borrow)
    }

    pub fn unwrap(self) -> bool {
        matches!(self, Self::Unwrap | Self::UnwrapBorrow)
    }

    pub fn length(self) -> u64 {
        if matches!(self, Self::Copy | Self::Unwrap | Self::Native) {
            4
        } else {
            8
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
        may_panic: kind.unwrap(),
        always_returns: !kind.unwrap(),
        overflow_behavior: "not_applicable".into(),
    }))
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Kind, String> {
    let kind = kind(node)
        .filter(|_| enabled(facts))
        .ok_or("Unregistered byte call")?;
    if matches!(kind, Kind::Borrow | Kind::UnwrapBorrow | Kind::NativeUsize)
        && !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.native-borrowed-byte-readers")
    {
        return Err("Borrowed byte readers require their registered capability".into());
    }
    authority(node, kind, facts)?;
    source::verify(node, kind, facts)?;
    types::verify(node, kind, facts)?;

    Ok(kind)
}

fn authority(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts).ok_or("Unpinned byte-call build")?;
    let callee = crate::data_library::callee(node)?;
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry.definitions.get(&callee.path) != Some(&callee.definition_hash)
        || node
            .dispatch
            .as_ref()
            .is_none_or(|dispatch| dispatch.op != callee.path)
    {
        return Err("Byte call disagrees with pinned core definition".into());
    }

    if kind.conversion() {
        let selected = callee
            .implementation
            .ok_or("Missing selected slice conversion implementation")?;
        if selected.crate_name != "core"
            || selected.crate_hash != registry.core_hash
            || registry.definitions.get(&selected.path) != Some(&selected.definition_hash)
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
        location.value = types::array(4);
    }

    if locations.is_empty() {
        return Err("Byte copy lacks complete formal origins".into());
    }
    Ok(locations)
}

pub(crate) use origins::{project, source_origins};
