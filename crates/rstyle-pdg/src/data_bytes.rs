//! Pinned, concrete slice-to-array / Result / native-byte call semantics.
//! Each source call remains a distinct typed operation; failures are not erased.

mod origins;
mod semantics;
mod source;
#[cfg(test)]
mod tests;
mod types;

use crate::{HirNode, call_summaries::Summary, unit_store::EvidenceFacts};
use serde_json::Value;

pub(crate) const CHECKED_FLOW_CAPABILITY: &str = "rust.checked-borrowed-result-flow";

pub(crate) const KEY: &str = "rstyle.data:byte-call";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Copy(u64),
    Unwrap(u64),
    Native,
    Borrow(u64),
    UnwrapBorrow(u64),
    NativeUsize,
    NativeU16,
    NativeU128,
    LittleU32,
}

pub(crate) fn kind(node: &HirNode) -> Option<Kind> {
    match node.dispatch.as_ref()?.op.as_str() {
        "std::convert::TryInto::try_into" | "core::convert::TryInto::try_into" => {
            Some(if borrowed_result(node) {
                Kind::Borrow(result_length(node)?)
            } else {
                Kind::Copy(result_length(node)?)
            })
        }
        "std::result::Result::<T, E>::unwrap" | "core::result::Result::<T, E>::unwrap" => {
            Some(if node.children.first().is_some_and(borrowed_result) {
                Kind::UnwrapBorrow(result_length(node.children.first()?)?)
            } else {
                Kind::Unwrap(result_length(node.children.first()?)?)
            })
        }
        "core::num::<impl u16>::from_ne_bytes" => Some(Kind::NativeU16),
        "core::num::<impl u128>::from_ne_bytes" => Some(Kind::NativeU128),
        "core::num::<impl u32>::from_le_bytes" => Some(Kind::LittleU32),
        "core::num::<impl u32>::from_ne_bytes" => Some(Kind::Native),
        "core::num::<impl usize>::from_ne_bytes" => Some(Kind::NativeUsize),
        _ => None,
    }
}

fn result_length(node: &HirNode) -> Option<u64> {
    match crate::data_aggregates::parsed_type(node).ok()? {
        crate::data_types::TypeToken::SliceArrayResult { length, .. } => Some(length),
        _ => None,
    }
}

fn borrowed_result(node: &HirNode) -> bool {
    matches!(
        crate::data_aggregates::parsed_type(node),
        Ok(crate::data_types::TypeToken::SliceArrayResult { borrowed: true, .. })
    )
}

impl Kind {
    pub fn construct(self) -> &'static str {
        match self {
            Self::Copy(_) => "data.bytes.copy-result",
            Self::Unwrap(_) => "data.bytes.unwrap",
            Self::Native => "data.bytes.native-u32",
            Self::LittleU32 => "data.bytes.little-u32",
            Self::Borrow(_) => "data.bytes.borrow-result",
            Self::UnwrapBorrow(_) => "data.bytes.unwrap-borrow",
            Self::NativeUsize => "data.bytes.native-usize",
            Self::NativeU16 => "data.bytes.native-u16",
            Self::NativeU128 => "data.bytes.native-u128",
        }
    }

    pub fn native(self) -> bool {
        matches!(
            self,
            Self::Native | Self::NativeUsize | Self::NativeU16 | Self::NativeU128 | Self::LittleU32
        )
    }

    pub fn conversion(self) -> bool {
        matches!(self, Self::Copy(_) | Self::Borrow(_))
    }

    pub fn unwrap(self) -> bool {
        matches!(self, Self::Unwrap(_) | Self::UnwrapBorrow(_))
    }

    pub fn length(self) -> u64 {
        match self {
            Self::Copy(length)
            | Self::Unwrap(length)
            | Self::Borrow(length)
            | Self::UnwrapBorrow(length) => length,
            Self::Native | Self::LittleU32 => 4,
            Self::NativeU16 => 2,
            Self::NativeU128 => 16,
            Self::NativeUsize => 8,
        }
    }

    fn payload_capability(self) -> Result<Option<&'static str>, String> {
        match self {
            Self::Borrow(4) | Self::UnwrapBorrow(4) | Self::LittleU32 => {
                Ok(Some(CHECKED_FLOW_CAPABILITY))
            }
            Self::Borrow(8) | Self::UnwrapBorrow(8) | Self::NativeUsize => {
                Ok(Some("rust.native-borrowed-byte-readers"))
            }
            Self::Borrow(_) | Self::UnwrapBorrow(_) => {
                Err("Borrowed byte conversion requires a registered exact array length".into())
            }
            _ => Ok(None),
        }
    }

    pub fn requires_checked_flow(self) -> bool {
        matches!(
            self,
            Self::Borrow(4) | Self::UnwrapBorrow(4) | Self::LittleU32
        )
    }

    pub fn requires_prefix(self, node: &HirNode) -> bool {
        matches!(self, Self::NativeU16 | Self::NativeU128)
            || matches!(self, Self::Copy(length) | Self::Unwrap(length) if length != 4)
            || node.dispatch.as_ref().is_some_and(|dispatch| {
                matches!(
                    dispatch.op.as_str(),
                    "core::convert::TryInto::try_into" | "core::result::Result::<T, E>::unwrap"
                )
            })
    }

    pub fn semantics(self) -> Value {
        semantics::value(self)
    }
}

pub(crate) fn enabled(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, "rust.slice-array-byte-calls")
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
        memory_effects: matches!(kind, Kind::Copy(_)),
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
    if let Some(capability) = kind.payload_capability()?
        && !crate::extraction_profile::facts_support(facts, capability)
    {
        return Err("Byte payload requires its registered capability".into());
    }
    if kind.requires_prefix(node)
        && !crate::extraction_profile::facts_support(
            facts,
            crate::data_subslices::PREFIX_CAPABILITY,
        )
        || matches!(kind, Kind::Copy(length) | Kind::Unwrap(length) if !matches!(length, 2 | 4 | 16))
    {
        return Err("Byte signature/display requires the registered prefix-reader contract".into());
    }
    authority(node, kind, facts)?;
    source::verify(node, kind, facts)?;
    types::verify(node, kind, facts)?;

    Ok(kind)
}

fn authority(node: &HirNode, kind: Kind, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts).ok_or("Unpinned byte-call build")?;
    let callee = crate::data_library::callee(node)?;
    if node
        .dispatch
        .as_ref()
        .is_none_or(|dispatch| dispatch.op != callee.path)
    {
        return Err("Byte call dispatch disagrees with its resolved definition".into());
    }
    pinned_callee(
        &callee,
        kind,
        registry,
        crate::extraction_profile::facts_support(facts, crate::data_subslices::PREFIX_CAPABILITY),
    )
}

fn pinned_callee(
    callee: &crate::data_library::Callee,
    kind: Kind,
    registry: &crate::data_library::Registry,
    aliases: bool,
) -> Result<(), String> {
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || registry
            .definitions
            .get(definition_path(&callee.path, aliases))
            != Some(&callee.definition_hash)
    {
        return Err("Byte call disagrees with pinned core definition".into());
    }

    if kind.conversion() {
        let selected = callee
            .implementation
            .as_ref()
            .ok_or("Missing selected slice conversion implementation")?;
        if selected.crate_name != "core"
            || selected.crate_hash != registry.core_hash
            || registry
                .definitions
                .get(definition_path(&selected.path, aliases))
                != Some(&selected.definition_hash)
            || definition_path(&selected.path, aliases)
                != "<T as std::convert::TryInto<U>>::try_into"
            || selected.implementation.is_some()
        {
            return Err("Slice conversion lacks the pinned blanket TryInto implementation".into());
        }
    } else if callee.implementation.is_some() {
        return Err("Unexpected trait implementation on inherent byte call".into());
    }
    Ok(())
}

fn definition_path(path: &str, aliases: bool) -> &str {
    if !aliases {
        return path;
    }
    match path {
        "core::convert::TryInto::try_into" => "std::convert::TryInto::try_into",
        "<T as core::convert::TryInto<U>>::try_into" => "<T as std::convert::TryInto<U>>::try_into",
        "core::result::Result::<T, E>::unwrap" => "std::result::Result::<T, E>::unwrap",
        _ => path,
    }
}

pub(crate) fn locations(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    kind: Kind,
) -> Result<Vec<crate::field_locations::derived::Located>, String> {
    if !matches!(kind, Kind::Copy(_)) {
        return Ok(Vec::new());
    }
    let receiver = &node.children[0];
    let base = if crate::data_subslices::candidate(receiver, facts) {
        crate::data_subslices::verify(receiver, facts)?.base
    } else {
        receiver
    };
    let mut locations =
        if matches!(
            crate::data_aggregates::parsed_type(base)?,
            crate::data_types::TypeToken::Array { .. }
        ) && crate::extraction_profile::facts_support(facts, "rust.accelerator-packing")
        {
            let root = crate::data_places::field_origin(base, facts)?;
            crate::field_locations::derived::from_origins(
                crate::data_aggregates::parsed_type(base)?,
                facts,
                vec![root],
            )?
        } else {
            let roots = crate::data_origins::origins(base, facts)?;
            crate::field_locations::derived::referents(base, facts, roots)?
        };
    for location in &mut locations {
        location
            .path
            .push(crate::field_locations::Projection::ByteWindow {
                operand_position: 0,
                length: kind.length(),
            });
        location.value = types::array(kind.length());
    }

    if locations.is_empty() {
        return Err("Byte copy lacks complete formal origins".into());
    }
    Ok(locations)
}

pub(crate) use origins::{project, source_origins};
