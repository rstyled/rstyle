//! Preserve receiver/range/start source and allow only half-open suffix syntax.
use super::*;

pub(super) fn verify(
    node: &HirNode,
    checked: &Checked<'_>,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let syntax = crate::source_evidence::original_expression(node, facts)?;
    let (call, range) = shape(&syntax)?;
    for (syntax, child) in [
        (call.receiver.as_ref(), checked.receiver),
        (&call.args[0], checked.range),
        (
            range.start.as_deref().ok_or("Missing suffix start")?,
            checked.start,
        ),
    ] {
        crate::source_evidence::verify_child_source(syntax, child, node, facts)?;
    }
    if checked.range.dispatch.is_some() || checked.range.kind != "expr_struct" {
        return Err("Suffix range must remain an original structural carrier".into());
    }
    let expansion = checked
        .range
        .expansion
        .as_ref()
        .ok_or("Missing compiler range desugaring witness")?;
    if expansion.callsite != checked.range.span || expansion.snippet != checked.range.snippet {
        return Err("Suffix range desugaring identifies another source expression".into());
    }

    Ok(())
}

pub(super) fn shape(
    expression: &syn::Expr,
) -> Result<(&syn::ExprMethodCall, &syn::ExprRange), String> {
    let syn::Expr::MethodCall(call) = crate::data_aggregates::ungroup(expression) else {
        return Err("Checked suffix get requires original method syntax".into());
    };
    if call.method != "get"
        || call.turbofish.is_some()
        || call.args.len() != 1
        || !call.attrs.is_empty()
    {
        return Err("Checked suffix get requires one exact original range argument".into());
    }
    let syn::Expr::Range(range) = crate::data_aggregates::ungroup(&call.args[0]) else {
        return Err("Checked suffix get requires suffix range syntax".into());
    };
    if !range.attrs.is_empty()
        || range.start.is_none()
        || range.end.is_some()
        || !matches!(range.limits, syn::RangeLimits::HalfOpen(_))
    {
        return Err("Checked suffix get permits only start..".into());
    }

    Ok((call, range))
}
