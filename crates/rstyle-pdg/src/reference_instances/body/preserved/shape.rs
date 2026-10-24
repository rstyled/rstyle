//! Preserve original metadata while checking concrete normalization separately.
use super::Result;
use crate::{HirNode, ResolvedTy, SignatureFacts};

pub(super) fn signature(before: &SignatureFacts, after: &SignatureFacts) -> Result<()> {
    if (
        &before.sig_span,
        &before.receiver,
        &before.return_ty_span,
        before.has_self,
        &before.abi,
        &before.generics,
        before.is_variadic,
        before.is_async,
        before.is_unsafe,
        before.is_const,
    ) != (
        &after.sig_span,
        &after.receiver,
        &after.return_ty_span,
        after.has_self,
        &after.abi,
        &after.generics,
        after.is_variadic,
        after.is_async,
        after.is_unsafe,
        after.is_const,
    ) || before.params.len() != after.params.len()
    {
        return Err("Constructor diagnostic changes original signature metadata".into());
    }
    ty(&before.return_ty, &after.return_ty)?;
    for (before, after) in before.params.iter().zip(&after.params) {
        if (
            &before.symbol,
            before.position,
            before.name_span,
            before.ty_span,
        ) != (
            &after.symbol,
            after.position,
            after.name_span,
            after.ty_span,
        ) {
            return Err("Constructor diagnostic changes original formal identity".into());
        }
        ty(&before.ty, &after.ty)?;
        node(&before.pat, &after.pat)?;
    }
    Ok(())
}

pub(super) fn node(before: &HirNode, after: &HirNode) -> Result<()> {
    if (
        &before.hir,
        &before.kind,
        before.span,
        &before.snippet,
        &before.res,
        &before.expansion,
        &before.binding,
        before.binding_mut,
        &before.lit,
        before.param_index,
        before.is_unsafe_block,
    ) != (
        &after.hir,
        &after.kind,
        after.span,
        &after.snippet,
        &after.res,
        &after.expansion,
        &after.binding,
        after.binding_mut,
        &after.lit,
        after.param_index,
        after.is_unsafe_block,
    ) || before.children.len() != after.children.len()
    {
        return Err(
            "Constructor diagnostic changes original complete HIR structure/evidence".into(),
        );
    }
    match (&before.ty, &after.ty) {
        (Some(before), Some(after)) => ty(before, after)?,
        (None, None) => {}
        _ => {
            return Err(
                "Constructor diagnostic adds or removes original node type evidence".into(),
            );
        }
    }

    dispatch(before.dispatch.as_ref(), after.dispatch.as_ref())?;
    for (before, after) in before.children.iter().zip(&after.children) {
        node(before, after)?;
    }
    Ok(())
}

pub(super) fn ty(before: &ResolvedTy, after: &ResolvedTy) -> Result<()> {
    if before.observed != after.observed
        || before.spelling != after.spelling
        || before
            .normalized
            .as_ref()
            .is_some_and(|value| after.normalized.as_ref() != Some(value))
    {
        return Err(
            "Constructor diagnostic changes original observed type/spelling or known normalization"
                .into(),
        );
    }
    // New canonical normalization is checked against the source-derived role
    // and complete canonical result tree in projection::verify.
    Ok(())
}

fn dispatch(before: Option<&crate::Dispatch>, after: Option<&crate::Dispatch>) -> Result<()> {
    match (before, after) {
        (None, None) => Ok(()),
        (Some(before), Some(after))
            if before.kind == after.kind
                && before.op == after.op
                && before.result_ty == after.result_ty
                && before.operand_tys.len() == after.operand_tys.len() =>
        {
            for (before, after) in before.operand_tys.iter().zip(&after.operand_tys) {
                if before != after
                    && !(before == "&[u8]" && super::super::projection::shared_bytes_token(after)?)
                {
                    return Err(
                        "Constructor dispatch changes more than proved shared-slice normalization"
                            .into(),
                    );
                }
            }
            Ok(())
        }
        _ => Err("Constructor diagnostic changes original operation dispatch".into()),
    }
}
