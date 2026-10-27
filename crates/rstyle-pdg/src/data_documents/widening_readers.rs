//! Whole checked widening readers retain every conditional callback and origin.
mod graph;
mod operations;
mod origins;

use super::*;
use crate::{unit_store::EvidenceFacts, widening_readers as contract};

pub(crate) const KEY: &str = "rstyle.data:widening-reader-origins";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    contract::candidate(facts)
}

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    contract::verify_source(facts)
}

pub(crate) fn source_node<'a>(
    node: &PdgNode,
    facts: &'a dyn EvidenceFacts,
) -> Option<Result<&'a crate::HirNode, String>> {
    let construct = node.attributes.construct.as_deref()?;
    construct.starts_with("data.widening-reader.").then(|| {
        let checked = contract::verify(facts)?;
        for spec in operations::specs(&checked)? {
            if construct == operations::construct(spec.name)
                && super::aggregates::source_span(node).is_ok_and(|span| {
                    span.start == u64::from(spec.source.span.start)
                        && span.end == u64::from(spec.source.span.end)
                })
            {
                let kind = if spec.category == NodeCategory::Merge {
                    "compat.rust.hir-r0.expression_merge"
                } else {
                    &spec.source.kind
                };
                if node.original_kind.known_value() != Some(kind) {
                    return Err("Widening reader operation changes its original source kind".into());
                }
                return Ok(spec.source);
            }
        }
        Err("Widening reader operation has no complete original source role".into())
    })
}

const TYPES: super::checked_roles::TypeReplay = super::checked_roles::TypeReplay {
    source: source_node,
    expected: expected_type,
};

pub(crate) fn verify_type(node: &PdgNode, facts: &dyn EvidenceFacts) -> Option<Result<(), String>> {
    TYPES.verify(node, facts)
}

fn expected_type(
    facts: &dyn EvidenceFacts,
    source: &crate::HirNode,
    construct: &str,
) -> Result<String, String> {
    let checked = contract::verify(facts)?;
    for spec in operations::specs(&checked)? {
        if spec.source.hir == source.hir && construct == operations::construct(spec.name) {
            return Ok(spec.token);
        }
    }
    Err("Widening reader value type has no checked source role".into())
}

fn source(facts: &dyn EvidenceFacts) -> Result<Option<contract::Checked<'_>>, String> {
    crate::source_evidence::when_candidate(facts, contract::candidate(facts), contract::verify)
}

pub(crate) fn source_contract(
    facts: &dyn EvidenceFacts,
) -> Result<crate::copy_tagged_values::origins::WireContract, String> {
    let checked = source(facts)?.ok_or("Missing complete original widening reader")?;
    origins::contract(facts, &checked)
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    super::guarded_sources::Policy {
        key: KEY,
        coverage: super::guarded_sources::Coverage::Complete(
            "Unaccounted widening reader operation",
        ),
    }
    .install(
        pair,
        facts,
        super::guarded_sources::Replay {
            source,
            graph: graph::verify,
        },
    )
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = source(facts)? else {
        if pair
            .pdg
            .nodes
            .iter()
            .any(|node| node.annotations.contains_key(KEY))
            || pair.pdg.extensions.contains_key(KEY)
        {
            return Err("Unproved source claims complete widening reader origins".into());
        }
        return Ok(false);
    };

    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        contract::CAPABILITY,
        || true,
        "Widening readers require their registered document policy",
    )?;
    if checked.roles.width == crate::native_reader_width::Width::U16 {
        crate::extraction_profile::require_document(
            &pair.pdg.profile,
            contract::U16_CAPABILITY,
            || true,
            "U16 widening reader requires its distinct registered document policy",
        )?;
    }
    let expected = graph::verify(pair, facts, &checked)?;

    for node in &pair.pdg.nodes {
        if node.annotations.get(KEY) != expected.get(&node.id)
            || node.annotations.contains_key(crate::data_origins::KEY)
        {
            return Err(
                "Widening reader origins differ from complete source and graph roles".into(),
            );
        }
    }
    if pair.pdg.extensions.contains_key(KEY) {
        return Err("Widening reader origins require per-operation evidence".into());
    }
    Ok(true)
}
