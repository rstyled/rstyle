//! Bind an original concrete Default member; its ordinary body proves behavior.
use super::{model, result};
use crate::{ByteSpan, HirFnFacts, concrete_types::Nominal};
use syn::spanned::Spanned;

pub(crate) struct SourceMember {
    pub self_type: syn::Type,
    pub trait_path: syn::Path,
    pub nominal: Nominal,
}

pub(crate) fn member(
    facts: &HirFnFacts,
    item: &syn::ItemImpl,
    bytes: &str,
) -> Result<Option<SourceMember>, String> {
    let Some(method) = item.items.iter().find_map(|member| {
        let syn::ImplItem::Fn(method) = member else {
            return None;
        };
        (method_range(bytes, method) == facts.fn_span).then_some(method)
    }) else {
        return Ok(None);
    };
    let Some(observation) = super::observation(facts)? else {
        return Ok(None);
    };

    let trait_path = syntax(item, method)?;
    let tree = result(facts)?;
    let (nominal, _) = tree
        .nominal_parts()
        .ok_or("Default result lacks its nominal owner")?;
    let checker = Checker {
        facts,
        bytes,
        observation: &observation,
        nominal,
        source_checksum: crate::source_identity::checksum(bytes.as_bytes()),
    };

    checker.signatures()?;
    checker.source(item, method)?;
    Ok(Some(SourceMember {
        self_type: (*item.self_ty).clone(),
        trait_path,
        nominal: nominal.clone(),
    }))
}

pub(super) fn syntax(item: &syn::ItemImpl, method: &syn::ImplItemFn) -> Result<syn::Path, String> {
    let Some((None, path, _)) = &item.trait_ else {
        return Err("Default requires a positive original trait implementation".into());
    };
    if item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || item.items.len() != 1
        || method.defaultness.is_some()
        || !matches!(method.vis, syn::Visibility::Inherited)
    {
        return Err("Default requires one ordinary concrete original member".into());
    }
    signature(&method.sig)?;
    Ok(path.clone())
}

fn signature(signature: &syn::Signature) -> Result<(), String> {
    if signature.ident != "default"
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.inputs.is_empty()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || !matches!(&signature.output, syn::ReturnType::Type(_, ty)
            if matches!(ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self")))
    {
        return Err("Default requires original safe nongeneric fn default() -> Self".into());
    }
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
    nominal: &'a Nominal,
    source_checksum: String,
}

impl Checker<'_> {
    fn signatures(&self) -> Result<(), String> {
        let observation = self.observation;
        let owner = model::nominal(&observation.trait_arguments, self.nominal)?;
        local(self.facts, &owner)?;
        local(self.facts, &observation.implementation)?;
        definition(self.facts, &observation.method)?;

        if observation.method != observation.selected
            || observation.associated_error.is_some()
            || !observation.selected_arguments.is_empty()
            || observation.method_header.is_const
            || observation.method_header.is_async
            || observation.requested_signature.verify(self.nominal)? != owner
            || observation.selected_signature.verify(self.nominal)? != owner
            || !self.facts.signature.params.is_empty()
            || self.facts.signature.has_self
            || self.facts.signature.is_const
            || self.facts.signature.is_async
            || self.facts.signature.is_unsafe
            || self.facts.signature.is_variadic
        {
            return Err("Default selected member/signatures contradict its original owner".into());
        }
        Ok(())
    }

    fn source(&self, item: &syn::ItemImpl, method: &syn::ImplItemFn) -> Result<(), String> {
        let observation = self.observation;
        let range = method_range(self.bytes, method);
        if crate::source_evidence::byte_range(self.bytes, range)?
            != self.facts.source.bytes.as_str()
        {
            return Err("Default original member source differs from captured function".into());
        }

        let implementation = ByteSpan {
            start: crate::source_evidence::original_span(self.bytes, item.impl_token.span).start,
            end: crate::source_evidence::original_span(self.bytes, item.brace_token.span.close())
                .end,
        };
        self.anchor(&observation.implementation_source, implementation)?;
        self.anchor(&observation.method_source, range)?;
        let [member] = observation.members.as_slice() else {
            return Err("Default requires its complete single-member implementation roster".into());
        };
        if member.definition != observation.method
            || member.trait_member.as_ref() != Some(&observation.trait_method)
        {
            return Err("Default member roster changes selected method or trait owner".into());
        }
        // Rustc ImplItem spans omit outer attributes; attributes are checked by
        // the enclosing source route against the complete original AST.
        self.anchor(&member.source, range)
    }

    fn anchor(
        &self,
        anchor: &crate::nominal_boundary::census::expansions::FileSpan,
        span: ByteSpan,
    ) -> Result<(), String> {
        if anchor.span != span
            || self
                .facts
                .build
                .frontend_options
                .get("source_file")
                .and_then(serde_json::Value::as_str)
                != anchor.path.to_str()
            || anchor.digest != self.source_checksum
        {
            return Err("Default original source anchor differs from complete source file".into());
        }
        Ok(())
    }
}

pub(super) fn local(facts: &HirFnFacts, definition: &model::Definition) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Default lacks its original compiled-crate identity")?;
    if !definition.local
        || definition.definition_identity.is_empty()
        || definition.path.is_empty()
        || definition.stable_crate_id.is_empty()
        || compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || compiled["crate_name"].as_str() != Some(definition.crate_name.as_str())
        || compiled["stable_crate_id"].as_str() != Some(definition.stable_crate_id.as_str())
    {
        return Err("Default local owner differs from original compiled crate".into());
    }
    Ok(())
}

pub(super) fn definition(facts: &HirFnFacts, definition: &model::Definition) -> Result<(), String> {
    local(facts, definition)?;
    crate::source_evidence::require_definition(
        facts,
        &definition.path,
        &definition.definition_identity,
    )
}
