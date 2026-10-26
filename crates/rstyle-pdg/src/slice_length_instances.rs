//! Independently checked bounded slice-length instances; raw observations grant nothing.
pub(crate) mod activation;
mod body;
mod materialize;
mod methods;
mod receipts;
pub mod source;
#[cfg(test)]
mod tests;
mod transport;
mod types;

use crate::{
    HirFnFacts, HirNode, HirRef, generated_sources::capture::Definition, unit_store::EvidenceFacts,
};
use transport::{Envelope, Outcome, Projection};

type Result<T> = std::result::Result<T, String>;
type Site = (u32, u32, u32);
fn site(hir: &HirRef) -> Site {
    (hir.krate, hir.owner, hir.local)
}

pub const CAPABILITY: &str = "rust.named-error-reader-flow";

/// Only construction by the complete independent verifier grants this token.
pub struct CheckedProjection {
    facts: HirFnFacts,
    roles: body::Roles,
}

impl CheckedProjection {
    pub fn facts(&self) -> &HirFnFacts {
        &self.facts
    }
    pub(crate) fn length_call(&self) -> &HirNode {
        self.node(&self.roles.length)
    }
    pub(crate) fn condition(&self) -> &HirNode {
        self.node(&self.roles.condition)
    }
    pub(crate) fn error_call(&self) -> &HirNode {
        self.node(&self.roles.error)
    }
    pub(crate) fn failure(&self) -> &HirNode {
        self.node(&self.roles.failure)
    }
    pub(crate) fn returned(&self) -> &HirNode {
        self.node(&self.roles.returned)
    }
    pub(crate) fn success(&self) -> &HirNode {
        self.node(&self.roles.success)
    }

    fn node(&self, hir: &HirRef) -> &HirNode {
        receipts::find(&self.facts.body, hir).expect("private checked source/HIR site")
    }
}

pub fn verify(owning: &dyn EvidenceFacts, callers: &[HirFnFacts]) -> Result<CheckedProjection> {
    let original = owning.facts();
    let envelope = envelope(original)?.ok_or("Missing slice instance observations")?;
    let projection = match envelope.outcome {
        Outcome::Observed { projection } => projection,
        Outcome::Unsupported { reason } => {
            return Err(format!(
                "Slice instance observation is unsupported: {reason}"
            ));
        }
    };
    let source = source::verify(&original.source.bytes)?;
    metadata(original, &projection, &source)?;
    crate::scalar_instances::verify_lexical_owner(original)?;
    let roles = body::verify(original, &source)?;
    findings(original, &roles.length)?;
    receipts::verify(original, &projection, &envelope.call_receipts, callers)?;
    let failure = receipts::find(&original.body, &roles.failure)
        .ok_or("Missing checked failure constructor")?;
    let types = types::Types::new(owning, &projection, failure)?;
    source_types(owning, &source, &types)?;
    materialize::verify(original, &projection, &roles, &types)?;
    let methods = methods::verify(original, &projection, &roles, &types)?;
    let facts = materialize::facts(original, &roles, &types, methods)?;
    constructors(owning, &facts, &roles)?;
    error_constructor(&facts, &roles, &source, &types)?;

    Ok(CheckedProjection { facts, roles })
}

pub(crate) fn requested_call(
    original: &HirFnFacts,
    caller: &HirFnFacts,
    call: &HirNode,
) -> Result<bool> {
    if !may_request(original, caller, call) {
        return Ok(false);
    }
    let Some(envelope) = envelope(original)? else {
        return Ok(false);
    };
    let Outcome::Observed { projection } = envelope.outcome else {
        return Ok(false);
    };
    receipts::Requests {
        definition: &projection.definition,
        receipts: &envelope.call_receipts,
    }
    .requested(original, caller, call)
}

fn may_request(original: &HirFnFacts, caller: &HirFnFacts, call: &HirNode) -> bool {
    let target = original.build.frontend_options.get("definition_identity");
    if let Some(resolution) = crate::helper_calls::direct_resolution(call) {
        return resolution["local"] == true && target == resolution.get("definition_identity");
    }
    let Some(value) = original.build.frontend_options.get(transport::KEY) else {
        return false;
    };
    let Some(receipts) = value["call_receipts"].as_array() else {
        return false;
    };

    let identity = caller.build.frontend_options.get("definition_identity");
    receipts.iter().any(|receipt| {
        identity.is_some()
            && identity == receipt.get("caller")
            && receipt["call"]["hir"]["krate"].as_u64() == Some(u64::from(call.hir.krate))
            && receipt["call"]["hir"]["owner"].as_u64() == Some(u64::from(call.hir.owner))
            && receipt["call"]["hir"]["local"].as_u64() == Some(u64::from(call.hir.local))
    })
}

fn envelope(original: &HirFnFacts) -> Result<Option<Envelope>> {
    let Some(value) = original.build.frontend_options.get(transport::KEY) else {
        return Ok(None);
    };
    let envelope: Envelope = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if envelope.identity != "rstyle.rust.shared-slice-instance-projections/1"
        || envelope.scope != transport::SCOPE
        || envelope.obligations_verified
    {
        return Err("Unregistered slice instance observation envelope".into());
    }
    Ok(Some(envelope))
}

fn metadata(
    original: &HirFnFacts,
    projection: &Projection,
    source: &source::Checked,
) -> Result<()> {
    if crate::extraction_profile::ExtractionProfile::from_facts(original)?
        != crate::extraction_profile::ExtractionProfile::DataV3
        || projection.source_digest != original.source.digest
        || projection.fn_span != original.fn_span
        || projection.original_source_scan_complete != original.source_scan_complete
        || projection.owner.krate != 0
        || projection.owner.local != 0
        || projection.owner.owner != original.body.hir.owner
    {
        return Err("Slice projection changes its original DataV3 owner/source evidence".into());
    }

    definition(original, &projection.definition)?;
    let parameter = &projection.type_parameter;
    if !parameter.local
        || parameter.path != format!("{}::{}", original.name, source.generic)
        || parameter.crate_name != projection.definition.crate_name
        || parameter.stable_crate_id != projection.definition.stable_crate_id
        || parameter.definition_identity.is_empty()
        || parameter.definition_identity == projection.definition.definition_identity
    {
        return Err(
            "Slice projection generic parameter differs from its original source owner".into(),
        );
    }
    signature(original, source)
}

fn signature(original: &HirFnFacts, source: &source::Checked) -> Result<()> {
    let signature = &original.signature;
    if signature.has_self
        || signature.receiver.is_some()
        || signature.is_unsafe
        || signature.is_async
        || signature.is_const
        || signature.is_variadic
        || signature.abi.known_value() != Some("Rust")
    {
        return Err("Slice instance original signature is not an ordinary free function".into());
    }

    let expected: Vec<_> = signature
        .generics
        .iter()
        .filter(|generic| generic.name != "'_")
        .collect();
    if signature.generics.len() != 2
        || !matches!(expected.as_slice(), [generic] if generic.name == source.generic && generic.span == source.generic_span)
        || signature.generics.iter().any(|generic| {
            generic.name == "'_" && generic.span.start != source.parameters[0].ty_span.start
        })
    {
        return Err(
            "Slice instance changes its bare source type parameter/implicit Sized contract".into(),
        );
    }
    Ok(())
}

fn error_constructor(
    facts: &HirFnFacts,
    roles: &body::Roles,
    source: &source::Checked,
    types: &types::Types,
) -> Result<()> {
    let node = receipts::find(&facts.body, &roles.error).ok_or("Missing checked error call")?;
    let resolution = crate::helper_calls::direct_resolution(node)
        .ok_or("Slice error lacks exact direct constructor resolution")?;
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Slice error constructor lacks dispatch")?;
    let crate::concrete_types::TypeTree::Struct { nominal, .. } = &types.error else {
        return Err("Missing checked error nominal".into());
    };

    let method = source
        .constructor_path
        .segments
        .last()
        .ok_or("Missing source constructor name")?
        .ident
        .to_string();
    let expected = format!("{}::{method}", nominal.definition_path);
    let error_token = crate::data_types::token(&types.error)?;
    let text_token = crate::data_types::token(&crate::concrete_types::TypeTree::Shared(Box::new(
        crate::concrete_types::TypeTree::Str,
    )))?;

    if resolution["local"] != true
        || resolution["path"] != expected
        || resolution["definition_identity"]
            .as_str()
            .is_none_or(str::is_empty)
        || resolution["crate"]
            != facts
                .build
                .frontend_options
                .get("rstyle.rust.compiled-crate")
                .ok_or("Missing compiled error owner")?["crate_name"]
        || dispatch.kind != "resolved_direct"
        || dispatch.op != expected
        || dispatch.operand_tys != [text_token]
        || dispatch.result_ty != error_token
    {
        return Err(
            "Slice error constructor differs from exact local source/type/dispatch identity".into(),
        );
    }
    Ok(())
}

fn definition(original: &HirFnFacts, definition: &Definition) -> Result<()> {
    let options = &original.build.frontend_options;
    let compiled = options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Missing original compiled crate")?;
    if !definition.local
        || definition.path != original.name
        || definition.definition_identity.is_empty()
        || options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            != Some(definition.definition_identity.as_str())
        || compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || compiled["crate_name"] != definition.crate_name
        || compiled["stable_crate_id"] != definition.stable_crate_id
    {
        return Err("Slice projection differs from original compiler definition/build".into());
    }
    Ok(())
}

pub(super) fn findings(facts: &HirFnFacts, length: &HirRef) -> Result<()> {
    if facts.source_scan_complete && facts.source_diagnostics.is_empty() {
        return Ok(());
    }
    let length =
        receipts::find(&facts.body, length).ok_or("Missing checked deferred length call")?;
    crate::source_evidence::checked_activation::exact_deferred_call(facts, length).map_err(
        |error| {
            format!(
                "{error}; source_scan_complete={}, length={:?} {:?}, diagnostics={:?}",
                facts.source_scan_complete, length.span, length.snippet, facts.source_diagnostics
            )
        },
    )
}

fn source_types(
    owning: &dyn EvidenceFacts,
    source: &source::Checked,
    types: &types::Types,
) -> Result<()> {
    let resolution = owning.nominal_resolution()?;
    let crate::concrete_types::TypeTree::Struct { nominal, .. } = &types.error else {
        return Err("Missing checked error nominal".into());
    };
    resolution.matches_nominal(
        &syn::TypePath {
            qself: None,
            path: source.error_path.clone(),
        },
        nominal,
    )?;
    resolution.verify_result_type_source(
        owning.facts(),
        &syn::TypePath {
            qself: None,
            path: source.result_path.clone(),
        },
    )
}

fn constructors(owning: &dyn EvidenceFacts, facts: &HirFnFacts, roles: &body::Roles) -> Result<()> {
    for (hir, variant) in [(&roles.failure, "Err"), (&roles.success, "Ok")] {
        let node = receipts::find(&facts.body, hir).ok_or("Missing checked Result constructor")?;
        let resolution = node
            .res
            .as_ref()
            .ok_or("Missing exact Result constructor resolution")?;
        if resolution.kind != crate::concrete_results::RESOLUTION
            || resolution.target.is_some()
            || node.dispatch.is_some()
        {
            return Err("Slice result constructor changes its exact core variant".into());
        }
        let callee: crate::data_library::Callee =
            serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
        let registry =
            crate::data_library::registry_for(facts).ok_or("Unpinned Result constructor")?;
        let key = format!("std::prelude::v1::{variant}");
        if ![
            key.clone(),
            format!("core::prelude::v1::{variant}"),
            format!("core::result::Result::{variant}"),
            format!("std::result::Result::{variant}"),
        ]
        .contains(&callee.path)
            || !registry.exact_core_definition(&callee, &key)
        {
            return Err("Slice Result constructor lacks its pinned definition".into());
        }
        crate::concrete_results::source::constructor(node, owning, variant)?;
    }
    Ok(())
}
