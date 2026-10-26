//! Static formal source and binding identities.
use super::*;

/// Explicit source lifetime plus exact immutable binding; normalized &str alone
/// cannot distinguish a static input from a shorter borrow.
pub(crate) fn static_formal(
    facts: &dyn EvidenceFacts,
    position: u32,
) -> Result<Formal<'_>, String> {
    formal(facts, position, TypeToken::Shared(Box::new(TypeToken::Str)))
}

pub(super) fn constructor_formals(facts: &dyn EvidenceFacts) -> Result<Vec<Formal<'_>>, String> {
    if facts.bindings.len() != facts.signature.params.len() {
        return Err("Named error constructor cannot contain additional bindings".into());
    }
    if let Some(transfer) = owned::checked(facts)? {
        return Ok(vec![
            formal(facts, 0, TypeToken::from_tree(&transfer.input))?,
            static_formal(facts, 1)?,
        ]);
    }
    match facts.signature.params.len() {
        1 => Ok(vec![static_formal(facts, 0)?]),
        2 if crate::extraction_profile::facts_support(facts, SCALAR_CAPABILITY) => (0..2)
            .map(|position| formal(facts, position, TypeToken::Scalar("u32".into())))
            .collect(),
        _ => Err("Named error constructor requires one static or two scalar formals".into()),
    }
}

fn formal(
    facts: &dyn EvidenceFacts,
    position: u32,
    token: TypeToken,
) -> Result<Formal<'_>, String> {
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    source::signature(&syntax)?;
    let argument = argument(&syntax, position)?;
    source::payload_type_in(&argument.ty, &token, facts)?;
    let parameter = facts
        .signature
        .params
        .get(position as usize)
        .filter(|parameter| parameter.position == position)
        .ok_or("Named error formal lacks exact source position")?;
    parameter_type(parameter, &token)?;
    source::span(argument.pat.as_ref(), parameter.name_span, facts)?;
    source::span(argument.ty.as_ref(), parameter.ty_span, facts)?;
    binding(facts, parameter, &argument.pat)?;

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    if !matches!(interface.parameters.get(position as usize), Some(Observation::Captured { ty }) if TypeToken::from_tree(ty) == token)
    {
        return Err("Named error formal lacks complete captured interface evidence".into());
    }
    Ok(Formal {
        position,
        pattern: &parameter.pat,
        token,
    })
}

pub(super) fn formal_operand<'a>(
    node: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Formal<'a>, String> {
    if node.kind != "expr_path"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.dispatch.is_some()
    {
        return Err("Named error payload requires an unchanged static formal".into());
    }

    let target = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Named error field lacks local formal identity")?;

    let parameter = facts
        .signature
        .params
        .iter()
        .find(|parameter| &parameter.pat.hir == target)
        .ok_or("Named error field is not an original formal")?;

    let token = crate::data_aggregates::parsed_type(node)?;
    let formal = formal(facts, parameter.position, token)?;

    if crate::data_aggregates::parsed_type(node)? != formal.token {
        return Err("Named error operand changes its formal type".into());
    }

    Ok(formal)
}

fn argument(syntax: &syn::ItemFn, position: u32) -> Result<&syn::PatType, String> {
    let argument = syntax
        .sig
        .inputs
        .iter()
        .nth(position as usize)
        .ok_or("Static string formal is outside original signature")?;

    let syn::FnArg::Typed(argument) = argument else {
        return Err("Static string formal cannot be a receiver".into());
    };

    if !argument.attrs.is_empty() {
        return Err("Static string formal cannot hide conditional source attributes".into());
    }

    let syn::Pat::Ident(ident) = argument.pat.as_ref() else {
        return Err("Static string formal requires a plain identifier".into());
    };

    if ident.by_ref.is_some() || ident.mutability.is_some() || ident.subpat.is_some() {
        return Err("Static string formal must be immutable and unprojected".into());
    }

    Ok(argument)
}

fn parameter_type(parameter: &crate::frontend::ParamFact, token: &TypeToken) -> Result<(), String> {
    if parameter.pat.kind != "pat_binding"
        || parameter.pat.binding_mut != Some(false)
        || parameter.pat.binding.as_ref() != Some(&parameter.symbol)
        || !parameter.pat.children.is_empty()
        || parameter.pat.expansion.is_some()
        || parameter
            .ty
            .normalized
            .as_deref()
            .map(crate::data_types::parse)
            .transpose()?
            != Some(token.clone())
        || crate::data_aggregates::parsed_type(&parameter.pat)? != *token
    {
        return Err("Static string formal differs from its captured immutable binding".into());
    }

    Ok(())
}

fn binding(
    facts: &dyn EvidenceFacts,
    parameter: &crate::frontend::ParamFact,
    syntax: &syn::Pat,
) -> Result<(), String> {
    let syn::Pat::Ident(ident) = syntax else {
        return Err("Static string binding requires an identifier".into());
    };

    let bindings: Vec<_> = facts
        .bindings
        .iter()
        .filter(|binding| binding.hir == parameter.pat.hir)
        .collect();

    let [binding] = bindings.as_slice() else {
        return Err("Static string formal requires one unique binding declaration".into());
    };

    if !binding.is_param
        || binding.is_mut
        || binding.symbol != parameter.symbol
        || binding.decl_span != parameter.name_span
        || ident.ident != binding.name
        || binding.ty.normalized != parameter.ty.normalized
    {
        return Err("Static string formal binding inventory disagrees".into());
    }

    Ok(())
}
