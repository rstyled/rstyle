//! Resolved concrete inherent ownership and shared receiver correspondence.

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
    /// Exact nominal association; parameter zero carries the full concrete type.
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

pub fn definition(facts: &HirFnFacts) -> Result<Option<Definition>, String> {
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

    let owner =
        crate::nominal_boundary::resolution::inherent_owner(facts, &record.implementation_span)?;
    let owner = TypeToken::from_tree(&owner);
    if !matches!(&owner, TypeToken::Struct { nominal, .. } if nominal == &record.owner_nominal)
        || !facts.signature.has_self
        || receiver_type(facts)? != TypeToken::Shared(Box::new(owner))
    {
        return Err(
            "Inherent owner disagrees with exact source implementation/shared receiver".into(),
        );
    }
    Ok(Some(record))
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
    caller: &HirFnFacts,
    callee: &HirFnFacts,
    resolution: &serde_json::Value,
) -> Result<(), String> {
    if resolution.get("inherent").is_none() {
        return if callee.signature.has_self {
            Err("Associated helper lacks resolved inherent ownership".into())
        } else {
            Ok(())
        };
    }

    let expected = definition(callee)?.ok_or("Missing captured inherent callee ownership")?;
    let observed: CallOwner =
        serde_json::from_value(resolution["inherent"].clone()).map_err(|e| e.to_string())?;
    if observed.identity != IDENTITY
        || observed.implementation_identity != expected.implementation_identity
        || !source_resolved(node, caller)?
        || !ufcs_owner_matches(node, caller, &expected.owner_nominal)?
    {
        return Err("Resolved inherent call/receiver disagrees with verified callee owner".into());
    }

    Ok(())
}

fn ufcs_owner_matches(
    node: &HirNode,
    caller: &HirFnFacts,
    nominal: &crate::concrete_types::Nominal,
) -> Result<bool, String> {
    use crate::concrete_types::source_proof::PathResolution;
    if node.kind != "expr_call" {
        return Ok(true);
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

    let interface = crate::data_frontend::evidence(caller, crate::concrete_types::KEY)?;
    let resolution = crate::nominal_boundary::resolution::verify(caller, &interface)?;
    let owner = resolution.resolve(&syn::TypePath {
        qself: None,
        path: path.path,
    })?;
    Ok(
        matches!(owner, crate::concrete_types::TypeTree::Struct { nominal: ref owner, .. }
        if owner == nominal),
    )
}

/// Checks original invocation/receiver evidence before dependency linking;
/// admission still requires independently verified captured callee documents.
pub(crate) fn source_resolved(node: &HirNode, facts: &HirFnFacts) -> Result<bool, String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.verified-inherent-helper-calls")
        || node.expansion.is_some()
    {
        return Ok(false);
    }
    let Some(resolution) = crate::helper_calls::direct_resolution(node) else {
        return Ok(false);
    };
    if resolution.get("inherent").is_none() {
        return Ok(false);
    }

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
                && shared_receiver(node, facts)?)
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

fn shared_receiver(node: &HirNode, facts: &HirFnFacts) -> Result<bool, String> {
    use crate::{concrete_types::Observation, data_receivers::Kind};
    let capture = crate::data_receivers::verify(facts)?;
    let record = capture
        .methods
        .iter()
        .find(|record| record.call == node.hir)
        .ok_or("Missing exact inherent receiver capture")?;

    let kinds: Vec<_> = record.adjustments.iter().map(|step| &step.kind).collect();
    let signature = record
        .signature_inputs
        .iter()
        .map(|observation| match observation {
            Observation::Captured { ty } => crate::data_types::token(ty),
            _ => Err("Unsupported instantiated inherent signature".into()),
        })
        .collect::<Result<Vec<_>, String>>()?;

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
