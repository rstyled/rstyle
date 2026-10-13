//! Independently checked source/type/origin boundary for local data extraction.

use crate::{HirFnFacts, HirNode, concrete_types::*, data_types, field_locations};
use source_proof::PathResolution;

struct Unresolved;
impl PathResolution for Unresolved {
    fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
        Err("Named data types require selected-unit nominal resolution/effect evidence".into())
    }
}

pub fn verify(facts: &HirFnFacts) -> Result<(), String> {
    let interface = interface(facts)?;
    let source: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    verify_lifetimes(&source)?;
    verify_interface(facts, &source, &interface)?;

    verify_fields(facts)?;
    verify_nodes(&facts.body, facts)
}

fn interface(facts: &HirFnFacts) -> Result<InterfaceObservations, String> {
    let interface: InterfaceObservations = evidence(facts, KEY)?;
    interface.validate()?;
    Ok(interface)
}

fn verify_lifetimes(source: &syn::ItemFn) -> Result<(), String> {
    if source
        .sig
        .generics
        .params
        .iter()
        .any(|parameter| !matches!(parameter, syn::GenericParam::Lifetime(_)))
        || source
            .sig
            .generics
            .where_clause
            .as_ref()
            .is_some_and(|clause| {
                clause
                    .predicates
                    .iter()
                    .any(|predicate| !matches!(predicate, syn::WherePredicate::Lifetime(_)))
            })
    {
        return Err("Only lifetime parameters have a concrete data contract".into());
    }
    Ok(())
}

fn verify_interface(
    facts: &HirFnFacts,
    source: &syn::ItemFn,
    interface: &InterfaceObservations,
) -> Result<(), String> {
    if source.sig.inputs.len() != interface.parameters.len() {
        return Err("Data source/formal arity disagrees".into());
    }
    for ((source, observation), parameter) in source
        .sig
        .inputs
        .iter()
        .zip(&interface.parameters)
        .zip(&facts.signature.params)
    {
        let syn::FnArg::Typed(source) = source else {
            return Err("Nominal receiver proof is unavailable".into());
        };
        verify_type(
            &facts.source.bytes,
            &source.ty,
            observation,
            parameter.ty.normalized.as_deref(),
        )?;
        verify_parameter_origins(parameter)?;
    }

    let output = match &source.sig.output {
        syn::ReturnType::Default => syn::parse_str("()").expect("unit type"),
        syn::ReturnType::Type(_, output) => *output.clone(),
    };
    verify_output(facts, &source.sig.output, &output, &interface.result)?;
    Ok(())
}

fn verify_fields(facts: &HirFnFacts) -> Result<(), String> {
    let fields: field_locations::FieldObservations = evidence(facts, field_locations::KEY)?;
    field_locations::verify(facts, &fields)?;
    for field in &fields.operations {
        let field_locations::Location::Captured { value, .. } = &field.location else {
            return Err("Data field lacks verified formal origin".into());
        };
        data_types::parse(&data_types::token(value)?)?;
    }
    Ok(())
}

fn verify_type(
    bytes: &str,
    source: &syn::Type,
    observation: &Observation,
    normalized: Option<&str>,
) -> Result<(), String> {
    let Observation::Captured { ty } = observation else {
        return Err("Unsupported concrete data type".into());
    };
    source_proof::verify_spelling(spelling(bytes, source)?, ty, &Unresolved)?;

    let token = data_types::token(ty)?;
    data_types::parse(&token)?;
    if normalized != Some(token.as_str()) {
        return Err("Resolved data interface token disagrees with captured tree".into());
    }
    Ok(())
}

fn spelling<'a>(bytes: &'a str, source: &syn::Type) -> Result<&'a str, String> {
    use syn::spanned::Spanned;
    let span = source.span();
    let start = crate::source_evidence::byte_offset(bytes, span.start());
    let end = crate::source_evidence::byte_offset(bytes, span.end());

    bytes
        .get(start..end)
        .ok_or_else(|| "Missing exact data type source bytes".into())
}

fn verify_output(
    facts: &HirFnFacts,
    source: &syn::ReturnType,
    output: &syn::Type,
    observation: &Observation,
) -> Result<(), String> {
    let bytes = if matches!(source, syn::ReturnType::Default) {
        "()"
    } else {
        &facts.source.bytes
    };
    verify_type(
        bytes,
        output,
        observation,
        facts.signature.return_ty.normalized.as_deref(),
    )
}

fn verify_nodes(node: &HirNode, facts: &HirFnFacts) -> Result<(), String> {
    if let Some(token) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) {
        data_types::parse(token)?;
        if node.kind == "expr_path" {
            verify_path(node, facts, token)?;
        }
    }
    let origins = crate::data_origins::origins(node, facts)?;
    if node.kind.starts_with("expr_")
        && node
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
            .and_then(|token| data_types::parse(token).ok())
            .is_some_and(|ty| crate::data_origins::shared(&ty))
        && origins.is_empty()
    {
        return Err("Reference-valued expression lacks complete formal origins".into());
    }

    for child in &node.children {
        verify_nodes(child, facts)?;
    }
    Ok(())
}

fn verify_path(node: &HirNode, facts: &HirFnFacts, token: &str) -> Result<(), String> {
    let target = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Unresolved data binding read")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| binding.hir == *target)
        .ok_or("Missing data binding")?;
    if binding.ty.normalized.as_deref() != Some(token) {
        return Err("Data read type disagrees with its binding".into());
    }

    if !scalar(token) && !binding.is_param {
        return Err("Data local origin/value correspondence is not yet available".into());
    }
    Ok(())
}

pub fn accepted(token: &str) -> bool {
    data_types::parse(token).is_ok()
}

pub(crate) fn evidence<T: serde::de::DeserializeOwned>(
    facts: &HirFnFacts,
    key: &str,
) -> Result<T, String> {
    let value = facts
        .build
        .frontend_options
        .get(key)
        .ok_or_else(|| format!("Missing data evidence {key}"))?;
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn verify_parameter_origins(parameter: &crate::ParamFact) -> Result<(), String> {
    let token = data_types::parse(
        parameter
            .ty
            .normalized
            .as_deref()
            .ok_or("Missing parameter type")?,
    )?;
    if crate::data_origins::shared(&token) && crate::data_origins::parameter(parameter)?.is_empty()
    {
        return Err("Reference-bearing formal requires unavailable element origin evidence".into());
    }
    Ok(())
}
