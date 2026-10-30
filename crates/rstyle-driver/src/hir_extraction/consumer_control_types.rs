//! Retain builtin never types separately from graph type vocabulary.
use super::hir::{Context, hir_id};
use rstyle_pdg::{HirFnFacts, consumer_facts::control_types::*};
use rustc_middle::ty;

pub(super) fn capture(ctx: &Context<'_>, facts: &mut HirFnFacts) -> Result<(), String> {
    let mut never = Vec::new();
    let mut pending = vec![&facts.body];
    while let Some(node) = pending.pop() {
        if control_kind(&node.kind)
            && node.expansion.is_none()
            && node.ty.as_ref().is_some_and(|ty| ty.normalized.is_none())
            && ctx
                .typeck
                .node_type_opt(hir_id(&node.hir))
                .is_some_and(|ty| matches!(ty.kind(), ty::Never))
        {
            never.push(Control {
                hir: node.hir.clone(),
                span: node.span,
                kind: node.kind.clone(),
            });
        }
        pending.extend(&node.children);
    }

    if !never.is_empty() {
        facts.build.frontend_options.insert(
            KEY.into(),
            serde_json::to_value(Observations {
                identity: IDENTITY.into(),
                never,
            })
            .map_err(|error| error.to_string())?,
        );
    }
    Ok(())
}
