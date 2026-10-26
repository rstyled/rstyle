//! Complete length-checked ID wrappers retain early errors and the selected reader's effects.
mod graph;
mod source;

use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.named-error-id-reader-flow";
pub(crate) const KEY: &str = "rstyle.data:id-reader-precheck-origins";

pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::source_evidence::initial_statement(facts, CAPABILITY, |first| {
        first.kind == "stmt_expr" && matches!(first.children.as_slice(), [branch]
            if branch.kind == "expr_if")
            && facts.signature.return_ty.normalized.as_deref().is_some_and(|text|
                matches!(crate::data_types::parse(text), Ok(crate::data_types::TypeToken::Result { ok, .. })
                    if crate::id_reader_types::shape(&ok)))
    })
}

pub(crate) fn verify_source(facts: &dyn EvidenceFacts) -> Result<bool, String> {
    crate::source_evidence::source_present(facts, source::verify)
}

pub(crate) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    super::guarded_sources::Policy {
        key: KEY,
        coverage: super::guarded_sources::Coverage::Complete(
            "Unaccounted checked ID reader operation",
        ),
    }
    .install(
        pair,
        facts,
        super::guarded_sources::Replay {
            source: source::verify,
            graph: graph::verify,
        },
    )
}

pub(crate) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = source::verify(facts)? else {
        if pair.pdg.extensions.contains_key(KEY)
            || pair
                .pdg
                .nodes
                .iter()
                .any(|node| node.annotations.contains_key(KEY))
        {
            return Err("Unproved source claims complete checked ID reader origins".into());
        }
        return Ok(false);
    };

    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        CAPABILITY,
        || true,
        "Checked ID readers require their registered document policy",
    )?;

    let expected = graph::verify(pair, facts, &checked)?;

    if pair.pdg.extensions.contains_key(KEY)
        || pair.pdg.nodes.iter().any(|node| {
            node.annotations.get(KEY) != expected.get(&node.id)
                || node.annotations.contains_key(crate::data_origins::KEY)
        })
    {
        return Err(
            "Checked ID reader origins differ from complete guarded source and effects".into(),
        );
    }

    Ok(true)
}
