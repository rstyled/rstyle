//! Exact primitive source spelling joins compiler reference/formal observations.
use super::*;

pub(super) fn verify(
    facts: &HirFnFacts,
    evidence: &ScalarFormalAccesses,
    source: &syn::ItemFn,
) -> Result<(), Failure> {
    if source.sig.unsafety.is_some()
        || source.sig.asyncness.is_some()
        || facts.signature.has_self
        || !source.sig.generics.params.is_empty()
        || source.sig.inputs.len() != evidence.formals.len()
        || facts.signature.params.len() != evidence.formals.len()
    {
        return Err("Mutable scalar source signature/formal roster is unavailable".into());
    }
    for (index, (source, formal)) in source.sig.inputs.iter().zip(&evidence.formals).enumerate() {
        let syn::FnArg::Typed(source) = source else {
            return Err("Mutable scalar receiver is unsupported".into());
        };
        let parameter = &facts.signature.params[index];
        if formal.parameter as usize != index
            || parameter.position as usize != index
            || formal.binding != parameter.pat.hir
        {
            return Err("Mutable scalar compiler formal position/binding differs".into());
        }
        binding(facts, parameter, source)?;
        reference(&source.ty, &formal.observation)?;
        compiled_type(facts, parameter, &formal.observation)?;
    }

    Ok(())
}

fn binding(
    facts: &HirFnFacts,
    parameter: &crate::ParamFact,
    source: &syn::PatType,
) -> Result<(), Failure> {
    let syn::Pat::Ident(pattern) = source.pat.as_ref() else {
        return Err("Mutable scalar formal requires one direct binding pattern".into());
    };
    let bindings: Vec<_> = facts
        .bindings
        .iter()
        .filter(|binding| binding.hir == parameter.pat.hir)
        .collect();
    let [binding] = bindings.as_slice() else {
        return Err(Failure::Unknown(
            "Mutable scalar formal binding is missing or ambiguous".into(),
        ));
    };
    spans(facts, parameter, binding, source)?;
    if pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
        || parameter.pat.kind != "pat_binding"
        || pattern.ident != parameter.pat.snippet
        || parameter.pat.param_index != Some(parameter.position)
        || parameter.pat.binding.as_ref() != Some(&parameter.symbol)
        || !binding.is_param
        || binding.is_mut
        || binding.symbol != parameter.symbol
        || pattern.ident != binding.name
        || binding.ty.observed != parameter.ty.observed
    {
        return Err("Mutable scalar formal source/pattern/binding inventory differs".into());
    }

    Ok(())
}

fn spans(
    facts: &HirFnFacts,
    parameter: &crate::ParamFact,
    binding: &crate::BindingFact,
    source: &syn::PatType,
) -> Result<(), Failure> {
    let syn::Pat::Ident(pattern) = source.pat.as_ref() else {
        return Err("Mutable scalar formal requires one direct binding pattern".into());
    };
    let name_span = source_evidence::original_span(&facts.source.bytes, pattern.ident.span());
    let type_span = source_evidence::original_span(&facts.source.bytes, source.ty.span());
    if parameter.name_span != name_span
        || parameter.pat.span != name_span
        || binding.decl_span != name_span
        || parameter.ty_span != type_span
        || facts
            .source
            .bytes
            .get(type_span.start as usize..type_span.end as usize)
            != Some(parameter.ty.spelling.as_str())
        || parameter.pat.ty.as_ref().is_none_or(|ty| {
            ty.observed != parameter.ty.observed || ty.normalized != parameter.ty.normalized
        })
        || binding.ty.normalized != parameter.ty.normalized
    {
        return Err(Failure::Unknown(
            "Mutable scalar formal source/type spans or compiler type records differ".into(),
        ));
    }

    Ok(())
}

fn reference(source: &syn::Type, observation: &FormalObservation) -> Result<(), Failure> {
    match (source, observation) {
        (syn::Type::Reference(source), FormalObservation::Shared { referent })
            if source.mutability.is_none() =>
        {
            scalar(&source.elem, referent)
        }
        (syn::Type::Reference(source), FormalObservation::Mutable { referent })
            if source.mutability.is_some() =>
        {
            scalar(&source.elem, referent)
        }
        (syn::Type::Path(source), FormalObservation::NonReference)
            if source.qself.is_none() && source.path.is_ident("usize") =>
        {
            Ok(())
        }
        (_, FormalObservation::Unknown { .. }) => Err(Failure::Unknown(
            "Mutable scalar formal type remains unknown".into(),
        )),
        _ => Err("Source reference kind contradicts compiler formal observation".into()),
    }
}

fn scalar(source: &syn::Type, observation: &ScalarObservation) -> Result<(), Failure> {
    let usize = matches!(source, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("usize"));
    match observation {
        ScalarObservation::Captured { scalar } if scalar == "usize" && usize => Ok(()),
        ScalarObservation::Unsupported { reason } if !reason.is_empty() && !usize => Ok(()),
        ScalarObservation::Unknown { .. } => Err(Failure::Unknown(
            "Reference scalar referent remains unknown".into(),
        )),
        _ => Err("Source primitive referent contradicts compiled scalar observation".into()),
    }
}

fn compiled_type(
    facts: &HirFnFacts,
    parameter: &crate::ParamFact,
    observation: &FormalObservation,
) -> Result<(), Failure> {
    let observed: syn::Type = syn::parse_str(&parameter.ty.observed)
        .map_err(|_| Failure::Unknown("Compiler formal type is not available as a type".into()))?;
    reference(&observed, observation)?;
    let expected = expected_type(observation);
    if let Some((observed, normalized)) = expected
        && (parameter.ty.observed != observed || parameter.ty.normalized.as_deref() != normalized)
    {
        return Err(Failure::Unknown(
            "Compiler formal type contradicts scalar reference observation".into(),
        ));
    }
    let source = facts
        .source
        .bytes
        .get(parameter.ty_span.start as usize..parameter.ty_span.end as usize)
        .ok_or_else(|| Failure::Unknown("Formal type source interval is missing".into()))?;
    if source.chars().filter(|c| !c.is_whitespace()).ne(parameter
        .ty
        .observed
        .chars()
        .filter(|c| !c.is_whitespace()))
    {
        return Err("Compiler formal type differs from bounded source type spelling".into());
    }

    Ok(())
}

fn expected_type(observation: &FormalObservation) -> Option<(&'static str, Option<&'static str>)> {
    match observation {
        FormalObservation::Mutable {
            referent: ScalarObservation::Captured { .. },
        } => Some(("&mut usize", None)),
        FormalObservation::Shared {
            referent: ScalarObservation::Captured { .. },
        } => Some(("&usize", Some("&usize"))),
        FormalObservation::NonReference => Some(("usize", Some("usize"))),
        _ => None,
    }
}
