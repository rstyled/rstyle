//! Caller substitution and independent serialized access checks; no caller lowering.

use super::*;
use crate::data_documents::LocationContext;
use crate::{Access, HirNode, MemoryLocation, PdgNode, helper_calls::Summary};
use std::collections::BTreeMap;

#[derive(Default)]
struct Effects {
    reads: Vec<Access>,
    locations: BTreeMap<String, MemoryLocation>,
    snapshot_reads: Vec<Read>,
}

enum ReadOccurrence<'a> {
    Formal {
        read: &'a Read,
        parameter: u32,
        path: Vec<Projection>,
    },
    Snapshot(&'a Read),
}

pub(crate) fn verify_arguments(
    summary: &Summary,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<(), String> {
    visit_reads(summary, call, facts, |_| Ok(()))
}

fn expected(
    summary: &Summary,
    call: &HirNode,
    context: &mut LocationContext<'_>,
) -> Result<Effects, String> {
    let mut effects = Effects::default();
    visit_reads(summary, call, context.facts, |occurrence| {
        install_read(&mut effects, occurrence, context)
    })?;
    Ok(effects)
}

fn visit_reads(
    summary: &Summary,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    mut visit: impl FnMut(ReadOccurrence<'_>) -> Result<(), String>,
) -> Result<(), String> {
    let Some(reads) = &summary.memory_reads else {
        return Ok(());
    };
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-read-only-helper-calls")
    {
        return Err("Helper memory reads require the registered Data-calls profile".into());
    }

    let owned = borrowed_snapshot(summary, call, facts)?;
    let mut count = 0;
    for read in reads {
        let actual = call
            .children
            .get(read.parameter as usize)
            .ok_or("Helper read lacks its actual argument")?;
        let copied = read.path.first() != Some(&Projection::SharedReferent)
            && copied_input(summary, facts, read.parameter)?;
        if owned && read.parameter == 0 || copied {
            if count == crate::concrete_types::MAX_ENTRIES {
                return Err(BUDGET_ERROR.into());
            }
            visit(snapshot_read(read, actual)?)?;
            count += 1;
            continue;
        }
        let origins = crate::data_origins::origins(actual, facts)?;
        if origins.is_empty() {
            return Err("Helper read actual has no verified shared referent origin".into());
        }
        for origin in origins {
            if count == crate::concrete_types::MAX_ENTRIES {
                return Err(BUDGET_ERROR.into());
            }
            let occurrence = checked_read(read, origin, facts)?;
            visit(occurrence)?;
            count += 1;
        }
    }
    Ok(())
}

fn borrowed_snapshot(
    summary: &Summary,
    call: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    let receiver_type = call
        .children
        .first()
        .and_then(|child| child.ty.as_ref())
        .and_then(|ty| ty.normalized.as_ref());
    Ok(receiver_type != summary.argument_types.first()
        && crate::owned_receiver::checked(call, facts)?)
}

fn copied_input(
    summary: &Summary,
    facts: &dyn crate::unit_store::EvidenceFacts,
    parameter: u32,
) -> Result<bool, String> {
    Ok(
        crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.copy-value-helper-reads")
            && summary
                .argument_types
                .get(parameter as usize)
                .and_then(|ty| crate::data_types::parse(ty).ok())
                .is_some_and(|ty| matches!(ty, TypeToken::Struct { .. })),
    )
}

fn checked_read<'a>(
    read: &'a Read,
    origin: crate::data_origins::Origin,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<ReadOccurrence<'a>, String> {
    let path = substituted_path(&read.path, &origin.path)?;
    if type_at(facts, origin.parameter, &path)? != read.value {
        return Err("Substituted helper location disagrees with actual referent type".into());
    }
    Ok(ReadOccurrence::Formal {
        read,
        parameter: origin.parameter,
        path,
    })
}

fn snapshot_read<'a>(read: &'a Read, actual: &HirNode) -> Result<ReadOccurrence<'a>, String> {
    let tail = read
        .path
        .strip_prefix(&[Projection::SharedReferent])
        .unwrap_or(&read.path);
    let mut value = crate::data_types::parse(crate::data_aggregates::token(actual)?)?;
    for projection in tail {
        if !matches!(projection, Projection::StructField { .. }) {
            return Err("Owned receiver read requires exact Copy struct fields".into());
        }
        value = project(value, projection)?;
    }

    if value != read.value {
        return Err("Owned receiver snapshot read contradicts its input type".into());
    }
    Ok(ReadOccurrence::Snapshot(read))
}

fn install_read(
    effects: &mut Effects,
    occurrence: ReadOccurrence<'_>,
    context: &mut LocationContext<'_>,
) -> Result<(), String> {
    let (read, parameter, path) = match occurrence {
        ReadOccurrence::Snapshot(read) => {
            effects.snapshot_reads.push(read.clone());
            return Ok(());
        }
        ReadOccurrence::Formal {
            read,
            parameter,
            path,
        } => (read, parameter, path),
    };
    let facts = context.facts;
    let (id, location) = context.formal_location(parameter, &path, read.value.clone())?;
    let root = facts
        .signature
        .params
        .get(parameter as usize)
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
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    source_calls(context.facts)
        .into_iter()
        .map(|call| install_call(pair, call, context))
        .try_fold(BTreeMap::new(), |locations, added| {
            merge_locations(locations, added?)
        })
}

pub(crate) fn merge_locations(
    mut locations: BTreeMap<String, MemoryLocation>,
    added: BTreeMap<String, MemoryLocation>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    for (id, location) in added {
        insert_location(&mut locations, id, location)?;
    }
    Ok(locations)
}

fn source_calls(facts: &dyn crate::unit_store::EvidenceFacts) -> Vec<&HirNode> {
    crate::helper_calls::call_nodes(facts)
}

fn install_call(
    pair: &mut Pair,
    call: &HirNode,
    context: &mut LocationContext<'_>,
) -> Result<BTreeMap<String, MemoryLocation>, String> {
    let dependency = crate::helper_calls::verify_use(call, context.facts)?;
    let effects = expected(dependency.summary(), call, context)?;
    let id = source_call(pair.as_view(), call)?.id.clone();
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.id == id)
        .ok_or("Missing source CALL during access installation")?;
    node.attributes.reads = effects.reads;
    if !effects.snapshot_reads.is_empty() {
        node.annotations.insert(
            SNAPSHOT_KEY.into(),
            serde_json::to_value(&effects.snapshot_reads).map_err(|error| error.to_string())?,
        );
    }

    Ok(effects.locations)
}

pub(crate) fn verify_locations(
    pair: crate::producer::PairView<'_>,
    context: &mut LocationContext<'_>,
    locations: &mut BTreeMap<String, MemoryLocation>,
) -> Result<(), String> {
    for call in source_calls(context.facts) {
        let dependency = crate::helper_calls::verify_use(call, context.facts)?;
        let effects = expected(dependency.summary(), call, context)?;
        verify_snapshot_policy(pair, &effects)?;
        check_effects(
            source_call(pair, call)?,
            call,
            dependency.summary(),
            &effects,
        )?;
        for (id, location) in effects.locations {
            insert_location(locations, id, location)?;
        }
    }
    Ok(())
}

fn verify_snapshot_policy(
    pair: crate::producer::PairView<'_>,
    effects: &Effects,
) -> Result<(), String> {
    if crate::extraction_profile::policy_supports(&pair.pdg.profile, "rust.copy-value-helper-reads")
    {
        return Ok(());
    }
    if effects
        .snapshot_reads
        .iter()
        .any(|read| read.path.first() != Some(&Projection::SharedReferent))
    {
        return Err("Copy value helper reads require their registered document policy".into());
    }
    Ok(())
}

pub(crate) fn verify_effects(
    node: &PdgNode,
    call: &HirNode,
    summary: &Summary,
    context: &mut LocationContext<'_>,
) -> Result<(), String> {
    let effects = expected(summary, call, context)?;
    check_effects(node, call, summary, &effects)
}

const SNAPSHOT_KEY: &str = "rstyle.data:owned-receiver-reads";

fn snapshot_annotation(effects: &Effects) -> Result<Option<serde_json::Value>, String> {
    if effects.snapshot_reads.is_empty() {
        return Ok(None);
    }
    serde_json::to_value(&effects.snapshot_reads)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn check_effects(
    node: &PdgNode,
    call: &HirNode,
    summary: &Summary,
    effects: &Effects,
) -> Result<(), String> {
    if node.synthetic
        || node.provenance.origin != crate::ProvenanceOrigin::Source
        || node.original_kind.known_value() != Some(call.kind.as_str())
        || node.attributes.reads != effects.reads
        || node.annotations.get(SNAPSHOT_KEY) != snapshot_annotation(effects)?.as_ref()
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
    crate::data_documents::serialized_operation(pair, &call.span, |node| {
        node.category == NodeCategory::Call
            && node.original_kind.known_value() == Some(call.kind.as_str())
    })
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
