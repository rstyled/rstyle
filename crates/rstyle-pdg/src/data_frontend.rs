//! Independently checked source/type/origin boundary for local data extraction.

use crate::{HirNode, concrete_types::*, data_types, field_locations};
use source_proof::PathResolution;

struct Unresolved;
impl PathResolution for Unresolved {
    fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
        Err("Named data types require selected-unit nominal resolution/effect evidence".into())
    }
}

pub fn verify(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    let interface = interface(facts)?;
    let source: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    verify_lifetimes(&source)?;
    let resolution = nominal_resolution(facts, &interface)?;
    verify_interface(facts, &source, &interface, resolution.as_ref())?;

    for parameter in &facts.signature.params {
        crate::data_origins::parameter(parameter)?;
    }

    verify_fields(facts)?;
    let places = crate::data_places::Places::verify(facts)?;
    verify_nodes(&facts.body, facts, &places)
}

fn nominal_resolution(
    facts: &dyn crate::unit_store::EvidenceFacts,
    interface: &InterfaceObservations,
) -> Result<Option<crate::nominal_boundary::resolution::Resolution>, String> {
    if crate::declaration_origins::declarations(interface)?.is_empty() {
        return Ok(None);
    }
    crate::nominal_boundary::resolution::verify(facts, interface).map(Some)
}

struct Resolver<'a>(Option<&'a crate::nominal_boundary::resolution::Resolution>);
impl PathResolution for Resolver<'_> {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
        self.0.map_or_else(
            || Unresolved.resolve(path),
            |resolution| resolution.resolve(path),
        )
    }
}

fn interface(
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<std::borrow::Cow<'_, InterfaceObservations>, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
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
    facts: &dyn crate::unit_store::EvidenceFacts,
    source: &syn::ItemFn,
    interface: &InterfaceObservations,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<(), String> {
    let receiver = source
        .sig
        .inputs
        .first()
        .is_some_and(|arg| matches!(arg, syn::FnArg::Receiver(_)));
    if receiver != facts.signature.has_self || receiver != facts.signature.receiver.is_some() {
        return Err("Source/serialized receiver ownership disagrees".into());
    }
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
        match source {
            syn::FnArg::Typed(source) => SourceProof {
                bytes: &facts.source.bytes,
                resolution,
            }
            .verify_type(
                &source.ty,
                observation,
                parameter.ty.normalized.as_deref(),
            )?,
            syn::FnArg::Receiver(receiver) => {
                verify_receiver(receiver, observation, parameter, resolution)?
            }
        }
        verify_parameter_origins(parameter)?;
    }

    verify_output(facts, &source.sig.output, &interface.result, resolution)?;
    Ok(())
}

fn verify_receiver(
    receiver: &syn::Receiver,
    observation: &Observation,
    parameter: &crate::ParamFact,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<(), String> {
    let owning = resolution
        .and_then(|r| r.receiver.as_ref())
        .ok_or("Missing exact owning receiver impl")?;
    let expected = TypeTree::Shared(Box::new(owning.clone()));
    if receiver.reference.is_none()
        || receiver.mutability.is_some()
        || receiver.colon_token.is_some()
        || !matches!(observation, Observation::Captured { ty } if ty == &expected)
        || parameter.position != 0
        || parameter.ty.normalized.as_deref() != Some(data_types::token(&expected)?.as_str())
    {
        return Err("Receiver requires exact concrete shared-self correspondence".into());
    }
    Ok(())
}

fn verify_fields(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<(), String> {
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    field_locations::verify(facts, &fields)?;
    let nodes = field_locations::fields(&facts.body);

    for field in &fields.operations {
        let field_locations::Location::Captured { value, .. } = &field.location else {
            return Err("Data field lacks verified formal origin".into());
        };
        let token = data_types::token(value)?;
        let matched: Vec<_> = nodes.iter().filter(|node| node.hir == field.hir).collect();
        let [node] = matched.as_slice() else {
            return Err("Field type proof lacks one exact source/HIR operation".into());
        };
        if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(&token) {
            return Err(
                "Field value type contradicts independently verified source location".into(),
            );
        }
    }
    Ok(())
}

struct SourceProof<'a> {
    bytes: &'a str,
    resolution: Option<&'a crate::nominal_boundary::resolution::Resolution>,
}
impl SourceProof<'_> {
    fn verify_type(
        &self,
        source: &syn::Type,
        observation: &Observation,
        normalized: Option<&str>,
    ) -> Result<(), String> {
        let Observation::Captured { ty } = observation else {
            return Err("Unsupported concrete data type".into());
        };
        source_proof::verify_top_spelling(
            spelling(self.bytes, source)?,
            ty,
            &Resolver(self.resolution),
        )?;

        let token = data_types::token(ty)?;
        data_types::parse(&token)?;
        if normalized != Some(token.as_str()) {
            return Err("Resolved data interface token disagrees with captured tree".into());
        }
        Ok(())
    }
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
    facts: &dyn crate::unit_store::EvidenceFacts,
    source: &syn::ReturnType,
    observation: &Observation,
    resolution: Option<&crate::nominal_boundary::resolution::Resolution>,
) -> Result<(), String> {
    let output = match source {
        syn::ReturnType::Default => syn::parse_str("()").expect("unit type"),
        syn::ReturnType::Type(_, output) => *output.clone(),
    };
    let bytes = if matches!(source, syn::ReturnType::Default) {
        "()"
    } else {
        &facts.source.bytes
    };
    SourceProof { bytes, resolution }.verify_type(
        &output,
        observation,
        facts.signature.return_ty.normalized.as_deref(),
    )
}

fn verify_nodes(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    places: &crate::data_places::Places,
) -> Result<(), String> {
    if let Some(token) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) {
        data_types::parse(token)?;
        if node.kind == "expr_path" {
            verify_path(node, facts, token)?;
        }
    }
    let origins = if places.contains(node) {
        Vec::new()
    } else {
        crate::data_origins::origins(node, facts)?
    };
    if !places.contains(node)
        && node.kind.starts_with("expr_")
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
        verify_nodes(child, facts, places)?;
    }
    Ok(())
}

fn verify_path(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    token: &str,
) -> Result<(), String> {
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
    facts: &dyn crate::unit_store::EvidenceFacts,
    key: &str,
) -> Result<T, String> {
    let value = facts
        .build
        .frontend_options
        .get(key)
        .ok_or_else(|| format!("Missing data evidence {key}"))?;
    serde::Deserialize::deserialize(value).map_err(|error: serde_json::Error| error.to_string())
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
