//! Caller substitution and independent serialized access checks; no caller lowering.

use super::*;
use crate::{Access, HirNode, MemoryLocation, PdgNode, helper_calls::Summary};
use std::collections::BTreeMap;

#[derive(Default)]
struct Effects {
    reads: Vec<Access>,
    locations: BTreeMap<String, MemoryLocation>,
}

pub(crate) fn verify_arguments(
    summary: &Summary,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    expected(summary, call, facts).map(|_| ())
}

fn expected(
    summary: &Summary,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Effects, String> {
    let Some(reads) = &summary.memory_reads else {
        return Ok(Effects::default());
    };
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-read-only-helper-calls")
    {
        return Err("Helper memory reads require the registered Data-calls profile".into());
    }

    let mut effects = Effects::default();
    for read in reads {
        let actual = call
            .children
            .get(read.parameter as usize)
            .ok_or("Helper read lacks its actual argument")?;
        let origins = crate::data_origins::origins(actual, facts)?;
        if origins.is_empty() {
            return Err("Helper read actual has no verified shared referent origin".into());
        }
        for origin in origins {
            install_read(&mut effects, read, origin, facts)?;
        }
    }
    Ok(effects)
}

fn install_read(
    effects: &mut Effects,
    read: &Read,
    origin: crate::data_origins::Origin,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    if effects.reads.len() == crate::concrete_types::MAX_ENTRIES {
        return Err(BUDGET_ERROR.into());
    }
    let path = substituted_path(&read.path, &origin.path)?;
    if type_at(facts, origin.parameter, &path)? != read.value {
        return Err("Substituted helper location disagrees with actual referent type".into());
    }

    let (id, location) =
        crate::data_documents::formal_location(origin.parameter, &path, read.value.clone(), facts)?;
    let root = facts
        .signature
        .params
        .get(origin.parameter as usize)
        .ok_or("Missing actual formal root")?;
    effects.reads.push(Access {
        symbol_id: Some(root.symbol.clone()),
        location_id: Some(id.clone()),
        unresolved_reason: None,
        operand_position: Some(u64::from(read.parameter)),
    });
    insert_location(&mut effects.locations, id, location)
}

fn substituted_path(
    callee: &[Projection],
    actual: &[Projection],
) -> Result<Vec<Projection>, String> {
    let tail = callee
        .strip_prefix(&[Projection::SharedReferent])
        .ok_or("Helper read is not rooted at a shared formal referent")?;
    Ok(actual.iter().cloned().chain(tail.iter().cloned()).collect())
}

fn insert_location(
    locations: &mut BTreeMap<String, MemoryLocation>,
    id: String,
    location: MemoryLocation,
) -> Result<(), String> {
    if locations
        .get(&id)
        .is_some_and(|previous| previous != &location)
    {
        return Err("One formal location has contradictory type or identity evidence".into());
    }
    locations.insert(id, location);
    Ok(())
}

pub(crate) fn install(
    pair: &mut Pair,
    facts: &dyn crate::unit_store::EvidenceFacts,
    locations: &mut BTreeMap<String, MemoryLocation>,
) -> Result<(), String> {
    for call in source_calls(facts) {
        install_call_locations(pair, call, facts, locations)?;
    }
    Ok(())
}

fn source_calls(facts: &dyn crate::unit_store::EvidenceFacts) -> Vec<&HirNode> {
    crate::helper_calls::call_nodes(facts)
}

fn install_call_locations(
    pair: &mut Pair,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    locations: &mut BTreeMap<String, MemoryLocation>,
) -> Result<(), String> {
    for (id, location) in install_call(pair, call, facts)? {
        insert_location(locations, id, location)?;
    }

    Ok(())
}

fn install_call(
    pair: &mut Pair,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let dependency = crate::helper_calls::verify_use(call, facts)?;
    let effects = expected(dependency.summary(), call, facts)?;
    let id = source_call(pair.as_view(), call)?.id.clone();
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == id)
        .ok_or("Missing source CALL during access installation")?;
    node.attributes.reads = effects.reads;

    Ok(effects.locations)
}

pub(crate) fn verify_locations(
    pair: crate::producer::PairView<'_>,
    facts: &dyn crate::unit_store::EvidenceFacts,
    locations: &mut BTreeMap<String, MemoryLocation>,
) -> Result<(), String> {
    for call in source_calls(facts) {
        let dependency = crate::helper_calls::verify_use(call, facts)?;
        verify_effects(source_call(pair, call)?, call, facts, dependency.summary())?;
        for (id, location) in expected(dependency.summary(), call, facts)?.locations {
            insert_location(locations, id, location)?;
        }
    }
    Ok(())
}

pub(crate) fn verify_effects(
    node: &PdgNode,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    summary: &Summary,
) -> Result<(), String> {
    let effects = expected(summary, call, facts)?;
    if node.synthetic
        || node.provenance.origin != crate::ProvenanceOrigin::Source
        || node.original_kind.known_value() != Some(call.kind.as_str())
        || node.attributes.reads != effects.reads
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.exit_kind.is_some()
        || node
            .attributes
            .operator
            .as_ref()
            .and_then(crate::StringFact::known_value)
            != call.dispatch.as_ref().map(|dispatch| dispatch.op.as_str())
        || node.annotations["compat.rust.hir-r0:type"]["value"] != summary.result_type
    {
        return Err(
            "Serialized helper CALL reads, effects, type or source identity disagree".into(),
        );
    }
    Ok(())
}

fn source_call<'a>(
    pair: crate::producer::PairView<'a>,
    call: &HirNode,
) -> Result<&'a PdgNode, String> {
    let nodes = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == NodeCategory::Call
                && node.original_kind.known_value() == Some(call.kind.as_str())
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start == u64::from(call.span.start) && span.end == u64::from(call.span.end)
                })
        })
        .collect::<Vec<_>>();
    let [node] = nodes.as_slice() else {
        return Err("Missing or ambiguous source-backed helper CALL".into());
    };
    Ok(node)
}

/// An unsupported call remains excluded; a read-evidence budget is incomplete capture.
pub(crate) fn check_budgets(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-read-only-helper-calls")
    {
        return Ok(());
    }

    for call in source_calls(facts) {
        if let Err(error) = crate::helper_calls::verify_use(call, facts)
            && error == BUDGET_ERROR
        {
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitution_preserves_storage_dereferences_and_field_order() {
        let callee = [
            Projection::SharedReferent,
            Projection::TupleField { position: 0 },
        ];
        let stored = [
            Projection::SharedReferent,
            Projection::TupleField { position: 1 },
            Projection::SharedReferent,
        ];
        assert_eq!(
            substituted_path(&callee, &stored).unwrap(),
            vec![
                Projection::SharedReferent,
                Projection::TupleField { position: 1 },
                Projection::SharedReferent,
                Projection::TupleField { position: 0 },
            ]
        );
        assert_eq!(
            substituted_path(&callee, &[Projection::SharedReferent]).unwrap(),
            callee
        );
        assert!(substituted_path(&callee[1..], &stored).is_err());
    }
}
