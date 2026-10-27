//! Shared dispatch consistency; callers separately prove library semantics.
use crate::{HirNode, unit_store::EvidenceFacts};

pub(crate) struct InvocationSyntax {
    pub capability: &'static str,
    pub kind: &'static str,
    pub paths: &'static [&'static str],
}

pub(crate) fn candidate(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    syntax: InvocationSyntax,
) -> bool {
    crate::extraction_profile::gated_node(node, facts, syntax.capability, |node| {
        node.kind == syntax.kind
            && node
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| syntax.paths.contains(&dispatch.op.as_str()))
    })
}

pub(crate) fn verify_dispatch(node: &HirNode, path: &str, results: &[&str]) -> Result<(), String> {
    let observed = node
        .children
        .iter()
        .map(|child| {
            child
                .ty
                .as_ref()
                .map(|ty| ty.observed.as_str())
                .ok_or("Missing method operand observation")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !node.dispatch.as_ref().is_some_and(|dispatch| {
        dispatch.kind == "resolved_method"
            && dispatch.op == path
            && results.contains(&dispatch.result_ty.as_str())
            && dispatch.operand_tys == observed
    }) {
        return Err("Resolved method dispatch loses its operand/result observations".into());
    }
    Ok(())
}
