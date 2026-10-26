//! Resolved concrete inherent ownership and shared receiver correspondence.
mod digit_forwarders;
mod id_reader_decoders;
pub(crate) mod lifetimes;
mod result_maps;
pub(crate) use result_maps::{call_owner as generated_call_owner, leaf as generated_leaf_source};

use crate::{ByteSpan, HirFnFacts, HirNode, data_types::TypeToken};
use serde::{Deserialize, Serialize};
mod unit;
pub use unit::{Association, UNIT_IDENTITY, UNIT_KEY, UnitDefinitions};

pub const KEY: &str = "inherent_definition";
pub const IDENTITY: &str = "rstyle.helper.inherent-resolution/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub identity: String,
    pub definition_identity: String,
    pub implementation_identity: String,
    pub implementation_span: ByteSpan,
    /// Exact nominal association; shared methods also carry the type in parameter zero.
    pub owner_nominal: crate::concrete_types::Nominal,
    pub source_file: String,
    pub function_span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallOwner {
    pub identity: String,
    pub implementation_identity: String,
}

pub fn definition(
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<std::borrow::Cow<'_, Definition>>, String> {
    facts.inherent_definition()
}

pub(crate) fn check_definition(
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Option<Definition>, String> {
    let Some(value) = facts.build.frontend_options.get(KEY) else {
        return Ok(None);
    };
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-inherent-helper-calls")
    {
        return Err("Inherent ownership requires the registered Data-v3 boundary".into());
    }
    let record: Definition = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    verify_declared_name(&facts.source.bytes, &facts.name)?;
    unit::verify(facts, &record)?;
    if record.identity != IDENTITY
        || record.definition_identity.is_empty()
        || record.implementation_identity.is_empty()
        || facts.build.frontend_options.get("definition_identity")
            != Some(&serde_json::json!(record.definition_identity))
    {
        return Err("Inherent definition identity contradicts captured callee ownership".into());
    }

    if !facts.signature.has_self {
        crate::nominal_boundary::resolution::static_owner(
            facts,
            &record.implementation_span,
            &record.owner_nominal,
        )?;
        return Ok(Some(record));
    }
    check_receiver_owner(facts, &record)?;
    Ok(Some(record))
}

fn check_receiver_owner(
    facts: &dyn crate::unit_store::EvidenceFacts,
    record: &Definition,
) -> Result<(), String> {
    let owner =
        crate::nominal_boundary::resolution::inherent_owner(facts, &record.implementation_span)?;
    let receiver = receiver_type(facts)?;
    let owned = matches!(
        &receiver,
        TypeToken::Struct { .. } | TypeToken::FieldlessEnum { .. }
    ) && crate::owned_receiver::value_self(facts, &owner)?;
    let owner = TypeToken::from_tree(&owner);

    if !matches!(&owner, TypeToken::Struct { nominal, .. } | TypeToken::FieldlessEnum { nominal, .. } if nominal == &record.owner_nominal)
        || !facts.signature.has_self
        || receiver
            != if owned {
                owner
            } else {
                TypeToken::Shared(Box::new(owner))
            }
    {
        return Err("Inherent owner disagrees with exact source implementation/receiver".into());
    }
    Ok(())
}

fn verify_declared_name(source: &str, resolved: &str) -> Result<(), String> {
    use syn::ext::IdentExt;
    let method: syn::ImplItemFn = syn::parse_str(source).map_err(|e| e.to_string())?;
    if resolved.rsplit("::").next() != Some(method.sig.ident.unraw().to_string().as_str()) {
        return Err("Resolved inherent name disagrees with original method declaration".into());
    }
    Ok(())
}

fn receiver_type(facts: &HirFnFacts) -> Result<TypeToken, String> {
    crate::data_types::parse(
        facts
            .signature
            .params
            .first()
            .and_then(|parameter| parameter.ty.normalized.as_deref())
            .ok_or("Missing concrete inherent receiver type")?,
    )
}

pub(crate) fn verify(
    node: &HirNode,
    caller: &dyn crate::unit_store::EvidenceFacts,
    callee: &dyn crate::unit_store::EvidenceFacts,
    resolution: &serde_json::Value,
) -> Result<(), String> {
    crate::range_values::verify_selection(node, caller, callee, resolution)?;
    if resolution.get("value_trait_requested").is_some() {
        return crate::local_value_trait_calls::verify_selection(node, caller, callee, resolution);
    }
    if resolution.get("inherent").is_none() {
        return if callee.signature.has_self {
            Err("Associated helper lacks resolved inherent ownership".into())
        } else {
            Ok(())
        };
    }

    if id_reader_decoders::verify(node, caller, callee, resolution)? {
        return Ok(());
    }

    let expected = definition(callee)?.ok_or("Missing captured inherent callee ownership")?;
    let observed: CallOwner =
        serde_json::from_value(resolution["inherent"].clone()).map_err(|e| e.to_string())?;
    if observed.identity != IDENTITY
        || observed.implementation_identity != expected.implementation_identity
        || !source_resolved_in(node, caller, Some(callee))?
        || !ufcs_owner_matches(node, caller, &expected.owner_nominal)?
    {
        return Err("Resolved inherent call/receiver disagrees with verified callee owner".into());
    }

    Ok(())
}

fn ufcs_owner_matches(
    node: &HirNode,
    caller: &dyn crate::unit_store::EvidenceFacts,
    nominal: &crate::concrete_types::Nominal,
) -> Result<bool, String> {
    if node.kind != "expr_call" {
        return Ok(true);
    }
    if let Some(matched) = digit_forwarders::verify(node, caller, None, Some(nominal))? {
        return Ok(matched);
    }
    if let Some(matched) = result_maps::verify(node, caller, Some(nominal))? {
        return Ok(matched);
    }

    let source = caller
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original UFCS invocation")?;
    let mut expression: syn::Expr = syn::parse_str(source).map_err(|e| e.to_string())?;
    while let syn::Expr::Paren(inner) = expression {
        expression = *inner.expr;
    }

    let syn::Expr::Call(call) = expression else {
        return Ok(false);
    };
    let syn::Expr::Path(mut path) = *call.func else {
        return Ok(false);
    };
    path.path.segments.pop();
    path.path.segments.pop_punct();
    if path.path.segments.is_empty() || path.qself.is_some() {
        return Ok(false);
    }

    let resolution = caller.nominal_resolution()?;
    resolution.matches_nominal(
        &syn::TypePath {
            qself: None,
            path: path.path,
        },
        nominal,
    )?;
    Ok(true)
}

/// Checks original invocation/receiver evidence before dependency linking;
/// admission still requires independently verified captured callee documents.
pub(crate) fn source_resolved(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<bool, String> {
    source_resolved_in(node, facts, None)
}

fn source_resolved_in(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    callee: Option<&dyn crate::unit_store::EvidenceFacts>,
) -> Result<bool, String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-inherent-helper-calls")
    {
        return Ok(false);
    }
    let Some(resolution) = crate::helper_calls::direct_resolution(node) else {
        return Ok(false);
    };
    if resolution.get("inherent").is_none() {
        return Ok(false);
    }
    if let Some(matched) = digit_forwarders::verify(node, facts, callee, None)? {
        return Ok(matched);
    }
    if let Some(matched) = result_maps::verify(node, facts, None)? {
        return Ok(matched);
    }
    if node.expansion.is_some() {
        return Ok(false);
    }

    resolved_expression(node, facts, callee, &resolution)
}

fn resolved_expression(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    callee: Option<&dyn crate::unit_store::EvidenceFacts>,
    resolution: &serde_json::Value,
) -> Result<bool, String> {
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original inherent call source")?;
    let mut expression: syn::Expr = syn::parse_str(source).map_err(|e| e.to_string())?;
    while let syn::Expr::Paren(inner) = expression {
        expression = *inner.expr;
    }

    match expression {
        syn::Expr::MethodCall(call) if node.kind == "expr_method_call" => {
            Ok(call.turbofish.is_none()
                && resolution["path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with(&format!("::{}", call.method)))
                && shared_receiver(node, facts, callee)?)
        }
        syn::Expr::Call(call) if node.kind == "expr_call" => {
            Ok(matches!(call.func.as_ref(), syn::Expr::Path(path)
            if path.qself.is_none()
                && path.path.segments.iter().all(|segment| matches!(segment.arguments, syn::PathArguments::None))
                && path.path.segments.last().is_some_and(|segment|
                    resolution["path"].as_str().is_some_and(|name|
                        name.ends_with(&format!("::{}", segment.ident))))))
        }
        _ => Ok(false),
    }
}

fn shared_receiver(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    callee: Option<&dyn crate::unit_store::EvidenceFacts>,
) -> Result<bool, String> {
    use crate::{concrete_types::Observation, data_receivers::Kind};
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing exact inherent receiver capture")?;

    let mut signature = receiver_signature(record)?;

    if callee.is_some_and(|callee| {
        !signature.iter().map(|ty| Some(ty.as_str())).eq(callee
            .signature
            .params
            .iter()
            .map(|param| param.ty.normalized.as_deref()))
    }) {
        return Ok(false);
    }

    if crate::owned_receiver::call_signature(record, facts, &mut signature)? {
        return Ok(node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.operand_tys == signature));
    }

    let kinds: Vec<_> = record.adjustments.iter().map(|step| &step.kind).collect();
    Ok(matches!(
        &record.receiver_type,
        Observation::Captured {
            ty: crate::concrete_types::TypeTree::Shared(_)
        }
    ) && kinds == [&Kind::BuiltinDereference, &Kind::SharedBorrow]
        && record.adjusted_type == record.receiver_type
        && record.signature_inputs.first() == Some(&record.receiver_type)
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.operand_tys == signature))
}

fn receiver_signature(record: &crate::data_receivers::Method) -> Result<Vec<String>, String> {
    use crate::concrete_types::Observation;
    record
        .signature_inputs
        .iter()
        .map(|observation| match observation {
            Observation::Captured { ty } => crate::data_types::token(ty),
            _ => Err("Unsupported instantiated inherent signature".into()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_method_name_must_match_resolved_identity() {
        let source = "pub fn get(&self) -> u32 { self.value }";
        assert!(super::verify_declared_name(source, "model::Data::get").is_ok());
        assert!(super::verify_declared_name(source, "model::Data::wrap").is_err());
        assert!(super::verify_declared_name("fn r#type(&self) {}", "Data::type").is_ok());
    }
}
