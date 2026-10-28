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

pub(super) struct ArgumentEvidence {
    pub value: Availability<ArgumentValue>,
    pub formal: Option<u32>,
    pub unchanged_formal: Option<u32>,
}

impl ArgumentEvidence {
    pub fn unrooted(value: Availability<ArgumentValue>) -> Self {
        Self {
            value,
            formal: None,
            unchanged_formal: None,
        }
    }

    fn origin(&self, ordinal: Option<u32>) -> Availability<Option<u32>> {
        match &self.value {
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
        }
    }
}

pub(super) fn observe(
    site: Site<'_, '_>,
    mut argument: impl FnMut(&Expr, &HirNode) -> Result<ArgumentEvidence, String>,
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
    let resolved_callee = resolved_callee(node, call)?;
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
        resolved_callee,
        arguments,
    }))
}

#[derive(serde::Deserialize)]
struct CalleeRecord {
    path: Option<String>,
    #[serde(rename = "crate")]
    crate_name: Option<String>,
    definition_identity: Option<String>,
    local: Option<bool>,
    safe: Option<bool>,
    inherent: Option<serde_json::Value>,
    trait_selection: Option<serde_json::Value>,
}

fn resolved_callee(
    node: &HirNode,
    call: &syn::ExprCall,
) -> Result<Availability<super::ResolvedCallee>, String> {
    if !direct_free_syntax(call) {
        return Ok(unsupported(
            "Call target syntax is outside direct free functions",
        ));
    }
    let Some(dispatch) = &node.dispatch else {
        return Ok(unknown("Call dispatch evidence was not captured"));
    };
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

    let record: CalleeRecord = serde_json::from_str(&resolution.description)
        .map_err(|_| "Direct call resolution record is malformed")?;
    if record.inherent.is_some() || record.trait_selection.is_some() {
        return Ok(unsupported(
            "Associated call requires a separate effect contract",
        ));
    }
    check_dispatch(node, dispatch)?;
    resolved_record(node, dispatch, record)
}

fn resolved_record(
    node: &HirNode,
    dispatch: &crate::Dispatch,
    record: CalleeRecord,
) -> Result<Availability<super::ResolvedCallee>, String> {
    if let Some(path) = &record.path
        && path != &dispatch.op
    {
        return Err("Call dispatch disagrees with original resolved callee".into());
    }
    // Never is retained as an observed native type rather than a normalized token.
    let actual = crate::frontend::normalized(node).or_else(|| {
        node.ty
            .as_ref()
            .filter(|ty| ty.normalized.is_none() && ty.observed == "!")
            .map(|_| "!")
    });
    if actual != Some(dispatch.result_ty.as_str()) {
        return Ok(unknown("Call result type evidence is unavailable"));
    }

    let (Some(path), Some(crate_name), Some(identity), Some(local), Some(safe)) = (
        record.path,
        record.crate_name,
        record.definition_identity,
        record.local,
        record.safe,
    ) else {
        return Ok(unknown(
            "Direct callee identity or safety evidence was not captured",
        ));
    };
    if path.is_empty() || crate_name.is_empty() || identity.is_empty() {
        return Ok(unknown("Direct callee identity evidence is empty"));
    }
    Ok(Availability::Known(super::ResolvedCallee {
        path,
        local,
        safe,
        result_type: dispatch.result_ty.clone(),
    }))
}

fn direct_free_syntax(call: &syn::ExprCall) -> bool {
    call.attrs.is_empty()
        && matches!(&*call.func, Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() && path.path.segments.iter().all(|segment| segment.arguments.is_empty()))
}

fn observe_argument(
    source: &str,
    syntax: &Expr,
    child: &HirNode,
    argument: &mut impl FnMut(&Expr, &HirNode) -> Result<ArgumentEvidence, String>,
) -> Result<Argument, String> {
    let span = crate::source_evidence::original_span(source, syntax.span());
    let inner = crate::data_aggregates::ungroup(syntax);
    if child.span != crate::source_evidence::original_span(source, inner.span()) {
        return Err("Call native argument order contradicts original syntax".into());
    }

    let evidence = if !crate::conditional_source::expression_attributes(syntax).is_empty()
        || child.expansion.is_some()
    {
        ArgumentEvidence::unrooted(unsupported(
            "Call argument attributes or expansion are unsupported",
        ))
    } else {
        argument(syntax, child)?
    };

    Ok(Argument {
        span,
        formal: evidence.origin(evidence.formal),
        unchanged_formal: evidence.origin(evidence.unchanged_formal),
        value: evidence.value,
    })
}

fn target(
    facts: &HirFnFacts,
    node: &HirNode,
    call: &syn::ExprCall,
) -> Result<Availability<String>, String> {
    if !direct_free_syntax(call) {
        return Ok(unsupported(
            "Call target syntax is outside direct free functions",
        ));
    }
    let Some(dispatch) = &node.dispatch else {
        return Ok(unknown("Call dispatch evidence was not captured"));
    };
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

    let record: serde_json::Value = serde_json::from_str(&resolution.description)
        .map_err(|_| "Direct call resolution record is malformed")?;
    if record.get("inherent").is_some() || record.get("trait_selection").is_some() {
        return Ok(unsupported(
            "Associated call requires a separate effect contract",
        ));
    }
    check_dispatch(node, dispatch)
        .map_err(|error| format!("{} at {:?}: {error}", facts.name, node.span))?;
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(dispatch.result_ty.as_str())
    {
        return Ok(unknown("Call result type evidence is unavailable"));
    }
    local_target(facts, &record, &dispatch.op)
}

fn check_dispatch(node: &HirNode, dispatch: &crate::Dispatch) -> Result<(), String> {
    if dispatch.operand_tys.len() != node.children.len() {
        return Err("Call dispatch argument roster contradicts native children".into());
    }
    for (position, (child, declared)) in node.children.iter().zip(&dispatch.operand_tys).enumerate()
    {
        if let Some(actual) = child.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
            && actual != declared
        {
            return Err(format!(
                "Call dispatch argument {position} type contradicts native argument: declared {declared:?}, observed {actual:?}"
            ));
        }
    }

    Ok(())
}

fn local_target(
    facts: &HirFnFacts,
    record: &serde_json::Value,
    path: &str,
) -> Result<Availability<String>, String> {
    let local = record
        .get("local")
        .and_then(serde_json::Value::as_bool)
        .ok_or("Direct callee record has no local ownership observation")?;
    if !local {
        return Ok(unknown(
            "External call has no same-unit scalar leaf summary",
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
