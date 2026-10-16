//! Exact read occurrences derived from independently verified callee documents.

mod calls;
pub(crate) use calls::{
    check_budgets, install, merge_locations, verify_arguments, verify_effects, verify_locations,
};

use crate::{NodeCategory, data_types::TypeToken, field_locations::Projection, producer::Pair};
use serde::{Deserialize, Serialize};

pub const BUDGET_ERROR: &str = "Verified helper read evidence exceeds explicit 4096-entry budget";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Read {
    pub operation: String,
    pub access: usize,
    pub parameter: u32,
    pub path: Vec<Projection>,
    pub value: TypeToken,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturedLocation {
    parameter: u32,
    root_symbol: String,
    path: Vec<Projection>,
    #[serde(rename = "type")]
    value: TypeToken,
}

pub(crate) fn supported(
    facts: &dyn crate::unit_store::EvidenceFacts,
    pair: crate::producer::PairView<'_>,
) -> bool {
    crate::extraction_profile::ExtractionProfile::from_facts(facts)
        .is_ok_and(|profile| profile.supports("rust.verified-read-only-helper-calls"))
        && facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .and_then(|ty| crate::data_types::parse(ty).ok())
            .is_some_and(|ty| matches!(ty, TypeToken::Scalar(_)))
        && pair.pdg.memory_locations.iter().all(|location| {
            captured(location).is_ok_and(|location| {
                formal_type(facts, location.parameter)
                    .is_ok_and(|ty| matches!(ty, TypeToken::Shared(_)))
                    && location.path.first() == Some(&Projection::SharedReferent)
            })
        })
}

pub(crate) fn derive(
    facts: &dyn crate::unit_store::EvidenceFacts,
    pair: crate::producer::PairView<'_>,
) -> Result<Option<Vec<Read>>, String> {
    if pair.pdg.memory_locations.is_empty() {
        return Ok(None);
    }
    if !supported(facts, pair) {
        return Err(
            "Read-only helper requires shared formal locations and scalar/unit results".into(),
        );
    }

    let mut reads = Vec::new();
    for node in &pair.pdg.nodes {
        for (access, target) in node.attributes.reads.iter().enumerate() {
            let Some(id) = &target.location_id else {
                continue;
            };
            if !matches!(node.category, NodeCategory::Read | NodeCategory::Call) {
                return Err("Helper location access is not a source read/CALL".into());
            }
            let location = pair
                .pdg
                .memory_locations
                .iter()
                .find(|location| &location.id == id)
                .ok_or("Helper read has no declared location")?;
            let location = captured(location)?;
            verify_location(facts, &location, target)?;
            if reads.len() == crate::concrete_types::MAX_ENTRIES {
                return Err(BUDGET_ERROR.into());
            }
            reads.push(Read {
                operation: node.id.clone(),
                access,
                parameter: location.parameter,
                path: location.path,
                value: location.value,
            });
        }
    }
    Ok((!reads.is_empty()).then_some(reads))
}

fn captured(location: &crate::MemoryLocation) -> Result<CapturedLocation, String> {
    if location.abstraction != "rstyle.formal-field/2" {
        return Err("Helper read lacks registered formal location interpretation".into());
    }
    serde_json::from_value(
        location
            .extensions
            .get(crate::data_documents::LOCATION_KEY)
            .ok_or("Helper location lacks formal evidence")?
            .clone(),
    )
    .map_err(|error| error.to_string())
}

fn verify_location(
    facts: &dyn crate::unit_store::EvidenceFacts,
    location: &CapturedLocation,
    access: &crate::Access,
) -> Result<(), String> {
    let formal = facts
        .signature
        .params
        .get(location.parameter as usize)
        .ok_or("Unknown helper read formal")?;
    if location.root_symbol != formal.symbol
        || access.symbol_id.as_ref() != Some(&formal.symbol)
        || access.unresolved_reason.is_some()
        || type_at(facts, location.parameter, &location.path)? != location.value
    {
        return Err("Helper read contradicts verified formal root/path/type".into());
    }
    Ok(())
}

fn formal_type(
    facts: &dyn crate::unit_store::EvidenceFacts,
    parameter: u32,
) -> Result<TypeToken, String> {
    crate::data_types::parse(
        facts
            .signature
            .params
            .get(parameter as usize)
            .and_then(|formal| formal.ty.normalized.as_deref())
            .ok_or("Missing helper read formal type")?,
    )
}

pub(crate) fn type_at(
    facts: &dyn crate::unit_store::EvidenceFacts,
    parameter: u32,
    path: &[Projection],
) -> Result<TypeToken, String> {
    let mut value = formal_type(facts, parameter)?;
    for projection in path {
        value = project(value, projection)?;
    }
    Ok(value)
}

fn project(value: TypeToken, projection: &Projection) -> Result<TypeToken, String> {
    match (value, projection) {
        (TypeToken::Shared(value), Projection::SharedReferent) => Ok(*value),
        (TypeToken::Tuple(fields), Projection::TupleField { position }) => fields
            .get(*position)
            .cloned()
            .ok_or_else(|| "Unknown helper tuple field".into()),
        (
            TypeToken::Struct { nominal, fields },
            Projection::StructField {
                nominal: owner,
                position,
                name,
            },
        ) if nominal == *owner => fields
            .get(*position)
            .filter(|(field, _)| field == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| "Helper field identity disagrees with concrete type".into()),
        _ => Err("Helper read projection contradicts concrete referent type".into()),
    }
}
