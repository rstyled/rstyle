use super::{Argument, ArgumentValue, Availability, Call, native};
use crate::{HirFnFacts, HirNode};
use syn::{Expr, spanned::Spanned};

pub(super) fn signature(
    facts: &HirFnFacts,
    function: &syn::ItemFn,
) -> Result<Availability<()>, String> {
    if !ordinary_signature(function) {
        return Ok(unsupported(
            "Scalar helper requires an ordinary nongeneric free signature",
        ));
    }
    for (syntax, parameter) in function.sig.inputs.iter().zip(&facts.signature.params) {
        let availability = formal(facts, syntax, parameter)?;
        if !matches!(availability, Availability::Known(())) {
            return Ok(availability);
        }
    }

    result_type(facts, &function.sig.output)
}

fn ordinary_signature(function: &syn::ItemFn) -> bool {
    let signature = &function.sig;
    function.attrs.is_empty()
        && signature.receiver().is_none()
        && signature.generics.params.is_empty()
        && signature.generics.where_clause.is_none()
        && signature.unsafety.is_none()
        && signature.asyncness.is_none()
        && signature.constness.is_none()
        && signature.abi.is_none()
        && signature.variadic.is_none()
}

fn formal(
    facts: &HirFnFacts,
    syntax: &syn::FnArg,
    parameter: &crate::ParamFact,
) -> Result<Availability<()>, String> {
    let syn::FnArg::Typed(argument) = syntax else {
        return Ok(unsupported("Scalar helper receiver is unsupported"));
    };
    if !matches!(&*argument.pat, syn::Pat::Ident(ident) if ident.by_ref.is_none() && ident.subpat.is_none() && ident.attrs.is_empty())
    {
        return Ok(unsupported(
            "Scalar helper parameter requires a direct binding",
        ));
    }

    if crate::source_evidence::plain_type_path(&argument.ty, "u32") {
        return Ok(match parameter.ty.normalized.as_deref() {
            Some("u32") => Availability::Known(()),
            None => unknown("Scalar helper formal type was not captured"),
            Some(_) => unsupported("Scalar helper has an unsupported or unproved formal type"),
        });
    }
    if matches!(&*argument.ty, syn::Type::Reference(reference) if reference.mutability.is_some() && crate::source_evidence::plain_type_path(&reference.elem, "u32"))
    {
        return Ok(
            if super::super::formal_types::mutable_u32(
                facts,
                parameter.position,
                &parameter.pat.hir,
            )? {
                Availability::Known(())
            } else {
                unknown("Scalar helper mutable formal type was not captured")
            },
        );
    }

    Ok(unsupported(
        "Scalar helper has an unsupported or unproved formal type",
    ))
}

fn result_type(facts: &HirFnFacts, syntax: &syn::ReturnType) -> Result<Availability<()>, String> {
    let output = match syntax {
        syn::ReturnType::Default => Some("unit"),
        syn::ReturnType::Type(_, ty) if crate::source_evidence::plain_type_path(ty, "u32") => {
            Some("u32")
        }
        syn::ReturnType::Type(_, ty) if matches!(&**ty, syn::Type::Tuple(tuple) if tuple.elems.is_empty()) => {
            Some("unit")
        }
        _ => None,
    };

    if output.is_some() && facts.signature.return_ty.normalized.is_none() {
        return Ok(unknown("Scalar helper result type was not captured"));
    }
    if output.is_none() || output != facts.signature.return_ty.normalized.as_deref() {
        return Ok(unsupported(
            "Scalar helper result type is unsupported or unproved",
        ));
    }

    Ok(Availability::Known(()))
}

pub(super) struct Site<'a, 'b> {
    pub facts: &'a HirFnFacts,
    pub source: &'a str,
    pub index: &'b native::Index<'a>,
    pub call: &'b syn::ExprCall,
    pub boundary_index: usize,
}

pub(super) fn observe(
    site: Site<'_, '_>,
    mut argument: impl FnMut(
        &Expr,
        &HirNode,
    ) -> Result<(Availability<ArgumentValue>, Option<u32>), String>,
) -> Result<Option<Call>, String> {
    let Site {
        facts,
        source,
        index,
        call,
        boundary_index,
    } = site;
    let span = crate::source_evidence::original_span(source, call.span());
    let Some(node) = index.node("expr_call", span)? else {
        return Ok(None);
    };
    if node.children.len() != call.args.len() {
        return Err("Call native argument count contradicts original syntax".into());
    }

    let arguments = call
        .args
        .iter()
        .zip(&node.children)
        .map(|(syntax, child)| observe_argument(source, syntax, child, &mut argument))
        .collect::<Result<_, _>>()?;
    let target = target(facts, node, call)?;
    let target_path = if matches!(target, Availability::Known(_)) {
        node.dispatch.as_ref().map(|dispatch| dispatch.op.clone())
    } else {
        None
    };

    Ok(Some(Call {
        span,
        native: node.hir.clone(),
        boundary_index,
        target,
        target_path,
        arguments,
    }))
}

fn observe_argument(
    source: &str,
    syntax: &Expr,
    child: &HirNode,
    argument: &mut impl FnMut(
        &Expr,
        &HirNode,
    ) -> Result<(Availability<ArgumentValue>, Option<u32>), String>,
) -> Result<Argument, String> {
    let span = crate::source_evidence::original_span(source, syntax.span());
    let inner = crate::data_aggregates::ungroup(syntax);
    if child.span != crate::source_evidence::original_span(source, inner.span()) {
        return Err("Call native argument order contradicts original syntax".into());
    }

    let (value, ordinal) = if !crate::conditional_source::expression_attributes(syntax).is_empty()
        || child.expansion.is_some()
    {
        (
            unsupported("Call argument attributes or expansion are unsupported"),
            None,
        )
    } else {
        argument(syntax, child)?
    };

    let formal = match &value {
        Availability::Known(_) => Availability::Known(ordinal),
        Availability::Unknown { reason } => Availability::Unknown {
            reason: reason.clone(),
        },
        Availability::Unsupported { reason } => Availability::Unsupported {
            reason: reason.clone(),
        },
        Availability::NotApplicable { reason } => Availability::NotApplicable {
            reason: reason.clone(),
        },
    };
    Ok(Argument {
        span,
        value,
        formal,
    })
}

fn target(
    facts: &HirFnFacts,
    node: &HirNode,
    call: &syn::ExprCall,
) -> Result<Availability<String>, String> {
    if !call.attrs.is_empty()
        || !matches!(&*call.func, Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() && path.path.segments.iter().all(|segment| segment.arguments.is_empty()))
    {
        return Ok(unsupported(
            "Call target syntax is outside direct free functions",
        ));
    }
    let Some(dispatch) = &node.dispatch else {
        return Ok(unknown("Call dispatch evidence was not captured"));
    };
    check_dispatch(node, dispatch)?;
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(dispatch.result_ty.as_str())
    {
        return Ok(unknown("Call result type evidence is unavailable"));
    }

    let Some(resolution) = &node.res else {
        return Ok(unknown("Direct call resolution was not captured"));
    };
    if dispatch.kind != "resolved_direct"
        || resolution.kind != "resolved_direct_callee"
        || resolution.target.is_some()
    {
        return Ok(unknown(
            "Call has no original direct free-function resolution",
        ));
    }
    local_target(facts, &resolution.description, &dispatch.op)
}

fn check_dispatch(node: &HirNode, dispatch: &crate::Dispatch) -> Result<(), String> {
    if dispatch.operand_tys.len() != node.children.len() {
        return Err("Call dispatch argument roster contradicts native children".into());
    }
    for (child, declared) in node.children.iter().zip(&dispatch.operand_tys) {
        if let Some(actual) = child.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
            && actual != declared
        {
            return Err("Call dispatch argument type contradicts native argument".into());
        }
    }

    Ok(())
}

fn local_target(
    facts: &HirFnFacts,
    description: &str,
    path: &str,
) -> Result<Availability<String>, String> {
    let record: serde_json::Value = serde_json::from_str(description)
        .map_err(|_| "Direct call resolution record is malformed")?;
    let local = record
        .get("local")
        .and_then(serde_json::Value::as_bool)
        .ok_or("Direct callee record has no local ownership observation")?;
    if !local {
        return Ok(unknown(
            "External call has no same-unit scalar leaf summary",
        ));
    }

    if record.get("inherent").is_some() || record.get("trait_selection").is_some() {
        return Ok(unsupported(
            "Associated call requires a separate effect contract",
        ));
    }
    if record.get("path").and_then(serde_json::Value::as_str) != Some(path) {
        return Err("Call dispatch disagrees with original resolved callee".into());
    }

    let definition = record
        .get("definition_identity")
        .and_then(serde_json::Value::as_str)
        .filter(|identity| !identity.is_empty())
        .ok_or("Direct local call has no definition identity")?;
    // Context ownership was checked before parsing; definition IDs remain local handles.
    if facts
        .build
        .frontend_options
        .get(crate::nominal_boundary::CONTEXT_KEY)
        .is_none()
    {
        return Ok(unknown("Call selected-unit context was not captured"));
    }

    Ok(Availability::Known(definition.into()))
}

fn unsupported<T>(reason: &str) -> Availability<T> {
    Availability::Unsupported {
        reason: reason.into(),
    }
}

fn unknown<T>(reason: &str) -> Availability<T> {
    Availability::Unknown {
        reason: reason.into(),
    }
}
