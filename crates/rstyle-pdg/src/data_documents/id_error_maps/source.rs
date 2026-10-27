//! Synthetic operations retain their exact original callback or method source roles.
use super::*;
use crate::{HirNode, concrete_types::TypeTree};

pub(crate) fn source_node<'a>(
    node: &PdgNode,
    facts: &'a dyn EvidenceFacts,
) -> Option<Result<&'a HirNode, String>> {
    let construct = node.attributes.construct.as_deref()?;

    if !CONSTRUCTS.contains(&construct) {
        return None;
    }

    Some((|| {
        for source in contract::sources(facts) {
            let checked = contract::verify(source, facts)?;
            let (source, _) = role(construct, &checked)?;
            let span = super::super::aggregates::source_span(node)?;
            if span.start == u64::from(source.span.start) && span.end == u64::from(source.span.end)
            {
                let kind = if construct == contract::RESULT {
                    "compat.rust.hir-r0.expression_merge"
                } else {
                    &source.kind
                };
                if node.original_kind.known_value() != Some(kind) {
                    return Err("ID map operation changes exact original source kind".into());
                }
                return Ok(source);
            }
        }
        Err("ID map operation has no complete original callback source".into())
    })())
}

const TYPES: super::super::checked_roles::TypeReplay = super::super::checked_roles::TypeReplay {
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
    for call in contract::sources(facts) {
        let checked = contract::verify(call, facts)?;
        let (expected, ty) = role(construct, &checked)?;
        if expected.hir == source.hir {
            return Ok(ty);
        }
    }
    Err("ID map value type has no checked source role".into())
}

fn role<'a>(
    construct: &str,
    checked: &contract::Checked<'a>,
) -> Result<(&'a HirNode, String), String> {
    let (source, tree) = match construct {
        contract::ENVIRONMENT => (checked.closure, checked.storage.clone()),
        contract::SELECTOR => (checked.source, TypeTree::Scalar("bool".into())),
        contract::PAYLOAD => (checked.binding, checked.error().clone()),
        contract::BINDING => (checked.binding, TypeTree::Scalar("unit".into())),
        contract::MESSAGE => (
            checked.message_read,
            TypeTree::Shared(Box::new(TypeTree::Str)),
        ),
        contract::SUCCESS => (checked.source, checked.success().clone()),
        contract::ERROR | contract::OK | contract::RESULT => {
            (checked.source, checked.output.clone())
        }
        _ => return Err("Unregistered ID error map source role".into()),
    };

    Ok((source, crate::data_types::token(&tree)?))
}
