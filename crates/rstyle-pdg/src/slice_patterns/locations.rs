//! Exact saved-input regions; physical reads require independent CFG replay.
use super::{
    graph,
    semantics::{Checked, Region},
};
use crate::{
    Access, MemoryLocation, NodeCategory, PdgNode,
    data_documents::locations::LocationContext,
    data_origins::Origin,
    field_locations::{Projection, derived::Located},
    producer::{Pair, PairView},
};
use std::collections::BTreeMap;

type Reads = BTreeMap<String, Vec<Access>>;
type Locations = BTreeMap<String, MemoryLocation>;

impl Region {
    /// Source-proved region only. Graph consumers must first replay the length
    /// and digit true-edge guards; this conversion grants no execution proof.
    pub(crate) fn checked_origin(&self, checked: &Checked<'_>) -> Result<Origin, String> {
        let projection = match *self {
            Self::Element { position } if position < checked.consumed() => {
                Projection::PrefixElement { position }
            }
            Self::Suffix { start } if start == checked.consumed() => {
                Projection::PrefixSuffix { start }
            }
            _ => {
                return Err(
                    "Prefix region is outside its independently checked source contract".into(),
                );
            }
        };
        if !(1..=3).contains(&checked.consumed()) {
            return Err("Prefix region requires the bounded one-to-three-byte contract".into());
        }
        Ok(Origin {
            parameter: 0,
            path: vec![Projection::SharedReferent, projection],
        })
    }
}

pub(crate) fn install(
    pair: &mut Pair,
    context: &mut LocationContext<'_>,
    checked: &Checked<'_>,
) -> Result<Locations, String> {
    let (reads, locations) = expected(pair.as_view(), context, checked)?;
    install_reads(pair, &reads)?;

    Ok(locations)
}

fn install_reads(pair: &mut Pair, reads: &Reads) -> Result<(), String> {
    for node in &mut pair.pdg.nodes {
        if let Some(reads) = reads.get(&node.id) {
            install_read(node, reads)?;
        }
    }
    Ok(())
}

fn install_read(node: &mut PdgNode, reads: &[Access]) -> Result<(), String> {
    effects(node)?;
    if !node.attributes.reads.is_empty() && node.attributes.reads != reads {
        return Err("Prefix installation cannot replace contradictory physical accesses".into());
    }
    node.attributes.reads = reads.to_vec();

    Ok(())
}

pub(crate) fn verify(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    checked: &Checked<'_>,
) -> Result<Locations, String> {
    let (reads, locations) = expected(pair, context, checked)?;
    for node in &pair.pdg.nodes {
        if let Some(reads) = reads.get(&node.id) {
            effects(node)?;
            if node.attributes.reads != *reads {
                return Err("Prefix physical read loses its exact saved byte location".into());
            }
        }
    }
    Ok(locations)
}

#[derive(Default)]
struct ReadLocations {
    reads: Reads,
    locations: Locations,
}

fn expected(
    pair: PairView<'_>,
    context: &mut LocationContext<'_>,
    checked: &Checked<'_>,
) -> Result<(Reads, Locations), String> {
    if !context
        .facts
        .signature
        .params
        .first()
        .is_some_and(|formal| std::ptr::eq(checked.formal(), &formal.pat))
    {
        return Err("Prefix location context differs from the checked original formal".into());
    }
    let replay = graph::verify(pair, checked)?;
    let mut collected = ReadLocations::default();
    for ((selector, region), digit) in replay.selector_reads.iter().zip(checked.digits()) {
        let (accesses, entries) = region_reads(context, checked, region)?;
        let body = crate::data_documents::value_operation(pair, digit.read())?;
        if body.category != NodeCategory::Read
            || body.attributes.construct.as_deref() != Some("data.indirect-deref")
        {
            return Err(
                "Prefix body byte must retain its original distinct physical dereference".into(),
            );
        }
        insert_reads(&mut collected.reads, selector, body, accesses)?;
        collected.locations.extend(entries);
    }

    Ok((collected.reads, collected.locations))
}

fn insert_reads(
    reads: &mut Reads,
    selector: &PdgNode,
    body: &PdgNode,
    accesses: Vec<Access>,
) -> Result<(), String> {
    if reads
        .insert(selector.id.clone(), accesses.clone())
        .is_some()
        || reads.insert(body.id.clone(), accesses).is_some()
    {
        return Err(
            "Prefix selector and body reads must have distinct operation identities".into(),
        );
    }

    Ok(())
}

fn region_reads(
    context: &mut LocationContext<'_>,
    checked: &Checked<'_>,
    region: &Region,
) -> Result<(Vec<Access>, Locations), String> {
    let origin = region.checked_origin(checked)?;
    if !matches!(region, Region::Element { .. }) {
        return Err("Prefix suffix reference construction is not a physical byte read".into());
    }
    context.read_accesses(vec![Located {
        parameter: origin.parameter,
        path: origin.path,
        value: crate::concrete_types::TypeTree::Scalar("u8".into()),
    }])
}

fn effects(node: &PdgNode) -> Result<(), String> {
    if node.category != NodeCategory::Read
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.operator.is_some()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.exit_kind.is_some()
        || node.attributes.position.is_some()
        || node.annotations.contains_key(crate::data_places::KEY)
        || node.attributes.construct.as_deref() == Some("data.indirect-deref")
            && node.attributes.literal.is_some()
    {
        return Err(
            "Prefix physical read invents an unrelated effect or stored-slot access".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
