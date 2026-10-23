//! Check source inventory, exact saved operands, view semantics and absent reads.
use super::*;
use crate::{checked_slice_views::suffix as views, unit_store::EvidenceFacts};

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<(), String> {
    for source in views::sources(facts) {
        views::verify(source, facts)?;
        let node = super::serialized_operation_mut(pair, source)?;
        if node.attributes.construct.as_deref() != Some(views::CONSTRUCT) {
            return Err("Suffix get source lacks its exact constructed call".into());
        }
        node.annotations
            .insert(views::KEY.into(), views::semantics());
    }

    Ok(())
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let sources = views::sources(facts);
    let nodes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.attributes.construct.as_deref() == Some(views::CONSTRUCT)
                || node.annotations.contains_key(views::KEY)
        })
        .collect();
    if nodes.len() != sources.len() {
        return Err("Checked suffix view source/semantic inventory disagrees".into());
    }
    if !nodes.is_empty()
        && !crate::extraction_profile::policy_supports(&pair.pdg.profile, views::CAPABILITY)
    {
        return Err("Checked suffix view requires its registered document policy".into());
    }
    for source in sources {
        let checked = views::verify(source, facts)?;
        let node = super::typed_operation(
            pair,
            source,
            views::CONSTRUCT,
            crate::data_aggregates::token(source)?,
        )?;
        if node.category != NodeCategory::Call
            || node.annotations.get(views::KEY) != Some(&views::semantics())
        {
            return Err("Checked suffix get loses its precise no-read bounds semantics".into());
        }
        super::verify_saved_operands(
            pair,
            node,
            &[
                super::value_operation(pair, checked.receiver)?,
                super::value_operation(pair, checked.start)?,
            ],
        )?;
        super::callbacks::order::operands(pair, checked.receiver, checked.start, node)?;
        super::summary_records::Record {
            source,
            node,
            summary: views::resolve(source, facts)?,
        }
        .verify_pure(pair, facts, views::CONSTRUCT)?;
    }

    Ok(())
}
