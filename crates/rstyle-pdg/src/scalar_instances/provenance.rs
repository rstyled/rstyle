//! Exact resolved source-bound and assertion identities; no admission authority.
use super::source;
use crate::{ByteSpan, HirNode, HirRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use syn::spanned::Spanned;

#[cfg(test)]
#[path = "provenance/argument_tests.rs"]
mod argument_tests;
#[cfg(test)]
#[path = "provenance/pin_tests.rs"]
mod pin_tests;
mod pins;
#[cfg(test)]
mod tests;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    identity: String,
    complete: bool,
    parameters: Vec<GenericParameter>,
    bounds: Vec<Bound>,
    compiler_bounds: Vec<CompilerBound>,
    assertions: Vec<Assertion>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    path: String,
    definition_identity: String,
    #[serde(rename = "crate")]
    crate_name: String,
    stable_crate_id: String,
    local: bool,
    crate_hash: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Bound {
    status: String,
    parameter_index: u32,
    parameter_hir: HirRef,
    parameter_span: ByteSpan,
    predicate_span: ByteSpan,
    bound_span: ByteSpan,
    path_span: ByteSpan,
    origin: String,
    trait_definition: Definition,
    original_arguments: Vec<Parameter>,
    substituted_arguments: Vec<Argument>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GenericParameter {
    index: u32,
    name: String,
    hir: HirRef,
    span: ByteSpan,
    declaration_span: ByteSpan,
    definition: LocalDefinition,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalDefinition {
    #[serde(rename = "path")]
    _display_path: String,
    #[serde(rename = "definition_identity")]
    _definition_identity: String,
    #[serde(rename = "crate")]
    crate_name: String,
    stable_crate_id: String,
    local: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompilerBound {
    status: String,
    span: ByteSpan,
    trait_definition: Definition,
    original_arguments: Vec<Parameter>,
    substituted_arguments: Vec<Argument>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Parameter {
    kind: String,
    index: u32,
    name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Argument {
    kind: String,
    #[serde(rename = "value")]
    _display_value: String,
    display_only: bool,
    identity_status: String,
    identity: Scalar,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Scalar {
    Builtin { name: String },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Assertion {
    status: String,
    hir: HirRef,
    span: ByteSpan,
    callsite: ByteSpan,
    definition: Definition,
}

pub(super) fn verify(
    observation: &Observation,
    source: &source::Checked<'_>,
    scalar: &str,
) -> Result<(), String> {
    if observation.identity != "rstyle.rust.scalar-instance-provenance/1" || !observation.complete {
        return Err("Scalar source provenance requires its complete typed census".into());
    }
    let facts = source.facts();
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Scalar source provenance requires pinned compiler/target authority")?;

    bound(observation, facts, scalar, registry)?;
    assertions(&observation.assertions, facts, registry)
}

fn bound(
    observation: &Observation,
    facts: &crate::HirFnFacts,
    scalar: &str,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    let [record] = observation.bounds.as_slice() else {
        return Err("Scalar bound provenance requires exactly the original bound roster".into());
    };
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    let Some(syn::GenericParam::Type(parameter)) = syntax.sig.generics.params.first() else {
        return Err("Scalar bound provenance lacks its original type parameter".into());
    };
    let Some(syn::TypeParamBound::Trait(bound)) = parameter.bounds.first() else {
        return Err("Scalar bound provenance lacks its original trait path".into());
    };

    parameters(observation, facts, parameter)?;
    bound_source(record, facts, parameter, bound)?;
    bound_arguments(record, &parameter.ident.to_string(), scalar)?;

    compiler_bounds(observation, &parameter.ident.to_string(), scalar, registry)?;
    let name = bound
        .path
        .segments
        .last()
        .ok_or("Missing original trait name")?;
    pins::resolved(
        &record.trait_definition,
        registry,
        &format!("cmp::{}", name.ident),
    )
}

fn bound_source(
    record: &Bound,
    facts: &crate::HirFnFacts,
    parameter: &syn::TypeParam,
    bound: &syn::TraitBound,
) -> Result<(), String> {
    let span = |syntax| crate::source_evidence::original_span(&facts.source.bytes, syntax);
    if record.status != "resolved"
        || record.origin != "generic_parameter"
        || record.parameter_index != 0
        || record.parameter_hir.krate != 0
        || record.parameter_hir.owner != facts.body.hir.owner
        || record.parameter_span != span(parameter.ident.span())
        || record.predicate_span
            != (ByteSpan {
                start: span(
                    parameter
                        .colon_token
                        .ok_or("Missing source bound colon")?
                        .span(),
                )
                .start,
                end: span(bound.span()).end,
            })
        || record.bound_span != span(bound.span())
        || record.path_span != span(bound.path.span())
    {
        return Err(
            "Resolved trait provenance differs from the exact original generic bound".into(),
        );
    }
    Ok(())
}

struct Arguments<'a> {
    name: &'a str,
    scalar: &'a str,
    count: usize,
}

fn bound_arguments(record: &Bound, name: &str, scalar: &str) -> Result<(), String> {
    arguments(
        &record.original_arguments,
        &record.substituted_arguments,
        Arguments {
            name,
            scalar,
            count: 2,
        },
    )
}

fn arguments(
    original: &[Parameter],
    substituted: &[Argument],
    expected: Arguments<'_>,
) -> Result<(), String> {
    if original.len() != expected.count
        || !original.iter().all(|argument| {
            argument.kind == "parameter" && argument.index == 0 && argument.name == expected.name
        })
        || substituted.len() != expected.count
        || !substituted.iter().all(|argument| {
            let Scalar::Builtin { name } = &argument.identity;
            argument.kind == "type"
                && argument.display_only
                && argument.identity_status == "known"
                && name == expected.scalar
        })
    {
        return Err("Resolved trait provenance loses exact original parameter substitution".into());
    }
    Ok(())
}

fn compiler_bounds(
    observation: &Observation,
    name: &str,
    scalar: &str,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    if observation.compiler_bounds.len() != 2 {
        return Err(
            "Scalar source requires the complete comparison and implicit Sized obligation roster"
                .into(),
        );
    }
    let mut seen = [false; 2];

    for record in &observation.compiler_bounds {
        let expected = compiler_obligation(observation, record);
        if record.status != "resolved" || seen[expected.index] || record.span != expected.span {
            return Err(
                "Scalar compiler obligation differs from its complete original source roster"
                    .into(),
            );
        }
        seen[expected.index] = true;
        pins::verify(&record.trait_definition, registry, expected.path)?;
        arguments(
            &record.original_arguments,
            &record.substituted_arguments,
            Arguments {
                name,
                scalar,
                count: expected.count,
            },
        )?;
    }
    Ok(())
}

struct CompilerObligation<'a> {
    path: &'a str,
    count: usize,
    span: ByteSpan,
    index: usize,
}

fn compiler_obligation<'a>(
    observation: &'a Observation,
    record: &'a CompilerBound,
) -> CompilerObligation<'a> {
    let comparison = &observation.bounds[0];
    if matches!(
        record.trait_definition.path.as_str(),
        "core::marker::Sized" | "std::marker::Sized"
    ) {
        return CompilerObligation {
            path: &record.trait_definition.path,
            count: 1,
            span: observation.parameters[0].span,
            index: 0,
        };
    }

    CompilerObligation {
        path: &comparison.trait_definition.path,
        count: 2,
        span: comparison.bound_span,
        index: 1,
    }
}

fn assertions(
    records: &[Assertion],
    facts: &crate::HirFnFacts,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    let mut nodes = BTreeMap::new();
    assertion_nodes(&facts.body, &mut nodes);
    if records.len() != nodes.len() {
        return Err("Scalar assertion provenance omits or invents original assertion sites".into());
    }

    for record in records {
        let node = nodes
            .remove(&super::site(&record.hir))
            .ok_or("Scalar assertion provenance duplicates or invents an assertion site")?;
        assertion(record, node, registry)?;
    }
    Ok(())
}

fn assertion_nodes<'a>(node: &'a HirNode, nodes: &mut BTreeMap<super::Site, &'a HirNode>) {
    if node.kind == "expr_assert" {
        nodes.insert(super::site(&node.hir), node);
    }
    for child in &node.children {
        assertion_nodes(child, nodes);
    }
}

fn assertion(
    record: &Assertion,
    node: &HirNode,
    registry: &crate::data_library::Registry,
) -> Result<(), String> {
    let expansion = node
        .expansion
        .as_ref()
        .ok_or("Assertion lacks expansion provenance")?;
    let assertion = crate::assertions::Assertion::parse(&node.snippet)?;
    if record.status != "resolved"
        || record.span != node.span
        || record.callsite != expansion.callsite
        || record.callsite != node.span
        || expansion.snippet != node.snippet
    {
        return Err(
            "Resolved assertion provenance differs from its original expansion callsite".into(),
        );
    }
    pins::resolved(&record.definition, registry, assertion.name)
}

fn parameters(
    observation: &Observation,
    facts: &crate::HirFnFacts,
    parameter: &syn::TypeParam,
) -> Result<(), String> {
    let [record] = observation.parameters.as_slice() else {
        return Err(
            "Scalar source provenance requires the complete generic parameter roster".into(),
        );
    };
    let bound = &observation.bounds[0];
    let span = |syntax| crate::source_evidence::original_span(&facts.source.bytes, syntax);
    let owner = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Generic declaration lacks original crate identity")?;

    if record.index != 0
        || parameter.ident != record.name
        || record.hir != bound.parameter_hir
        || record.span != span(parameter.ident.span())
        || record.declaration_span != span(parameter.ident.span())
        || !record.definition.local
        || owner.get("crate_name").and_then(serde_json::Value::as_str)
            != Some(record.definition.crate_name.as_str())
        || owner
            .get("stable_crate_id")
            .and_then(serde_json::Value::as_str)
            != Some(record.definition.stable_crate_id.as_str())
    {
        return Err("Generic parameter provenance differs from the original declaration".into());
    }
    Ok(())
}
