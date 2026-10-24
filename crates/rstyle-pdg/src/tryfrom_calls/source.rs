//! Original member receipt; no nominal resolution, semantic permission or replay hashing.
use super::{model, *};
use crate::ByteSpan;
use syn::spanned::Spanned;

#[derive(Clone)]
pub(crate) struct SourceMember {
    pub self_type: syn::TypePath,
    pub trait_path: syn::Path,
    pub error_type: syn::TypePath,
    pub result_type: syn::TypePath,
    pub success: TypeTree,
    pub error: TypeTree,
    pub implementation_span: ByteSpan,
    pub function_span: ByteSpan,
    observation: model::Observation,
}

pub(crate) fn source_member(
    facts: &HirFnFacts,
    item: &syn::ItemImpl,
    bytes: &str,
    authority: &dyn CoreAuthority,
) -> Result<Option<SourceMember>, String> {
    let Some(method) = item.items.iter().find_map(|member| match member {
        syn::ImplItem::Fn(method) if method_range(bytes, method) == facts.fn_span => Some(method),
        _ => None,
    }) else {
        return Ok(None);
    };
    let Some(observation) = super::observation(facts)? else {
        return Ok(None);
    };
    let paths = syntax::implementation(item, method)?;
    owner::verify(facts, &observation, authority)?;
    let (success, error) = super::result(facts)?;
    let implementation_span = ByteSpan {
        start: crate::source_evidence::original_span(bytes, item.impl_token.span).start,
        end: crate::source_evidence::original_span(bytes, item.brace_token.span.close()).end,
    };
    let checker = Checker {
        facts,
        bytes,
        observation: &observation,
        anchors: crate::source_evidence::trait_members::SourceAnchors::new(
            facts,
            bytes,
            "TryFrom source anchor differs from complete original file",
        ),
    };
    checker.source(item, implementation_span)?;

    Ok(Some(SourceMember {
        self_type: paths.self_type,
        trait_path: paths.trait_path,
        error_type: paths.error_type,
        result_type: paths.result_type,
        success,
        error,
        implementation_span,
        function_span: facts.fn_span,
        observation,
    }))
}

/// The parent retains immutable original source/census binding separately.
/// Replay compares typed evidence and interface; it neither reparses nor hashes source.
pub(crate) fn verify_owner(
    facts: &HirFnFacts,
    member: &SourceMember,
    authority: &dyn CoreAuthority,
) -> Result<(), String> {
    let observation = super::observation(facts)?.ok_or("Missing retained TryFrom owner")?;
    let (success, error) = super::result(facts)?;
    if observation != member.observation
        || success != member.success
        || error != member.error
        || facts.fn_span != member.function_span
        || observation.implementation_source.span != member.implementation_span
    {
        return Err("TryFrom receipt differs from original typed owner/interface".into());
    }
    owner::verify(facts, &observation, authority)?;

    Ok(())
}

fn method_range(bytes: &str, method: &syn::ImplItemFn) -> ByteSpan {
    ByteSpan {
        start: crate::source_evidence::original_span(bytes, method.sig.span()).start,
        end: crate::source_evidence::original_span(bytes, method.block.span()).end,
    }
}

struct Checker<'a> {
    facts: &'a HirFnFacts,
    bytes: &'a str,
    observation: &'a model::Observation,
    anchors: crate::source_evidence::trait_members::SourceAnchors<'a>,
}

impl Checker<'_> {
    fn source(&self, item: &syn::ItemImpl, implementation: ByteSpan) -> Result<(), String> {
        let observed = self.observation;
        if crate::source_evidence::byte_range(self.bytes, self.facts.fn_span)?
            != self.facts.source.bytes.as_str()
        {
            return Err("TryFrom captured function differs from original file slice".into());
        }
        self.anchors
            .verify(&observed.implementation_source, implementation)?;
        self.anchors
            .verify(&observed.method_source, self.facts.fn_span)?;
        if observed.members.len() != item.items.len() {
            return Err("TryFrom member roster is incomplete".into());
        }
        let associated = observed
            .associated_error
            .as_ref()
            .ok_or("Missing TryFrom Error")?;
        for (syntax, captured) in item.items.iter().zip(&observed.members) {
            let (definition, trait_member, span) = match syntax {
                syn::ImplItem::Fn(method) => (
                    &observed.method,
                    &observed.trait_method,
                    method_range(self.bytes, method),
                ),
                syn::ImplItem::Type(error) => (
                    &associated.implementation_member,
                    &associated.trait_member,
                    crate::source_evidence::original_span(self.bytes, error.span()),
                ),
                _ => return Err("TryFrom roster contains another original member".into()),
            };
            if &captured.definition != definition
                || captured.trait_member.as_ref() != Some(trait_member)
            {
                return Err("TryFrom roster changes exact implementation or trait member".into());
            }
            self.anchors.verify(&captured.source, span)?;
            if matches!(syntax, syn::ImplItem::Type(_)) {
                self.anchors.verify(&associated.source, span)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
