//! Exact original impl/member syntax; lexical trait binding belongs to the caller.
use crate::{ByteSpan, HirFnFacts};
use syn::spanned::Spanned;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Method {
    name: String,
    output: String,
}

/// Proof inputs only: parent must independently resolve trait_path to
/// trait_identity in the original namespace, then call verify_trait on that item.
pub(crate) struct SourceMember {
    pub self_type: syn::TypePath,
    pub trait_path: syn::Path,
    pub trait_identity: syn::Path,
    pub method_name: String,
    roster: Vec<Method>,
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
        (range(bytes, method) == facts.fn_span).then_some(method)
    }) else {
        return Ok(None);
    };
    let mut source = syntax(item, method)?;
    if crate::source_evidence::byte_range(bytes, facts.fn_span)? != facts.source.bytes.as_str() {
        return Err("Local value trait original method differs from captured source".into());
    }

    super::model::original(facts)?;
    captured_signature(facts)?;
    let (trait_identity, method_name) = selected_path(&facts.name)?;
    if method_name != source.method_name {
        return Err("Local value trait selected member differs from original method".into());
    }
    source.trait_identity = trait_identity;
    Ok(Some(source))
}

pub(super) fn syntax(
    item: &syn::ItemImpl,
    method: &syn::ImplItemFn,
) -> Result<SourceMember, String> {
    let Some((None, trait_path, _)) = &item.trait_ else {
        return Err("Local value trait requires a positive original implementation".into());
    };
    let syn::Type::Path(self_type) = item.self_ty.as_ref() else {
        return Err("Local value trait requires its original primitive owner".into());
    };
    if item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.attrs.is_empty()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || self_type.qself.is_some()
        || !self_type.path.is_ident("usize")
        || crate::source_evidence::plain_relative_path(trait_path).is_none()
    {
        return Err(
            "Local value trait requires an ordinary nongeneric usize implementation".into(),
        );
    }

    let roster = impl_roster(item)?;
    let selected = signature(&method.sig)?;
    if selected.output != "u64"
        || !method.attrs.is_empty()
        || method.defaultness.is_some()
        || !matches!(method.vis, syn::Visibility::Inherited)
    {
        return Err(
            "Local value trait selected member requires original usize -> u64 signature".into(),
        );
    }
    Ok(SourceMember {
        self_type: self_type.clone(),
        trait_path: trait_path.clone(),
        trait_identity: trait_path.clone(),
        method_name: selected.name,
        roster,
    })
}

fn impl_roster(item: &syn::ItemImpl) -> Result<Vec<Method>, String> {
    let mut roster = Vec::new();
    for member in &item.items {
        let syn::ImplItem::Fn(method) = member else {
            return Err(
                "Local value trait implementation contains an unsupported associated item".into(),
            );
        };
        if !method.attrs.is_empty()
            || method.defaultness.is_some()
            || !matches!(method.vis, syn::Visibility::Inherited)
        {
            return Err(
                "Local value trait sibling has unsupported attributes or specialization".into(),
            );
        }
        let signature = signature(&method.sig)?;
        if roster
            .iter()
            .any(|previous: &Method| previous.name == signature.name)
        {
            return Err("Local value trait implementation repeats a member".into());
        }
        roster.push(signature);
    }
    if roster.is_empty() || roster.len() > crate::concrete_types::MAX_FIELDS {
        return Err("Local value trait requires its bounded complete member roster".into());
    }
    Ok(roster)
}

pub(crate) fn verify_trait(member: &SourceMember, item: &syn::ItemTrait) -> Result<(), String> {
    if item.unsafety.is_some()
        || item.auto_token.is_some()
        || !item.attrs.is_empty()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !item.supertraits.is_empty()
        || item.restriction.is_some()
        || member
            .trait_identity
            .segments
            .last()
            .is_none_or(|segment| segment.ident != item.ident)
    {
        return Err(
            "Local value trait declaration differs from its original ordinary trait".into(),
        );
    }

    let mut roster = Vec::new();
    for item in &item.items {
        let syn::TraitItem::Fn(method) = item else {
            return Err("Local value trait declaration has an unsupported associated item".into());
        };
        if method.default.is_some() || !method.attrs.is_empty() {
            return Err(
                "Local value trait default bodies and attributed members remain unsupported".into(),
            );
        }
        roster.push(signature(&method.sig)?);
    }

    if roster != member.roster {
        return Err(
            "Local value trait declaration and complete impl member roster disagree".into(),
        );
    }
    Ok(())
}

fn signature(signature: &syn::Signature) -> Result<Method, String> {
    let mut inputs = signature.inputs.iter();
    let Some(syn::FnArg::Receiver(receiver)) = inputs.next() else {
        return Err("Local value trait requires exactly one by-value self argument".into());
    };
    if inputs.next().is_some()
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || receiver.reference.is_some()
        || receiver.mutability.is_some()
        || receiver.colon_token.is_some()
        || !receiver.attrs.is_empty()
    {
        return Err("Local value trait requires an ordinary safe by-value self method".into());
    }

    let syn::ReturnType::Type(_, output) = &signature.output else {
        return Err("Local value trait requires an explicit unsigned scalar result".into());
    };
    let syn::Type::Path(output) = output.as_ref() else {
        return Err("Local value trait requires a direct unsigned scalar result".into());
    };
    let name = output
        .path
        .get_ident()
        .ok_or("Local value trait result is not a primitive name")?
        .to_string();
    if output.qself.is_some() || !matches!(name.as_str(), "u8" | "u16" | "u32" | "u64") {
        return Err("Local value trait result is outside its primitive contract".into());
    }
    Ok(Method {
        name: signature.ident.to_string(),
        output: name,
    })
}

pub(super) fn selected_path(name: &str) -> Result<(syn::Path, String), String> {
    let mut path: syn::ExprPath = syn::parse_str(name).map_err(|error| error.to_string())?;
    let qself = path
        .qself
        .ok_or("Local value trait selected path lacks its concrete owner")?;
    if qself.as_token.is_none()
        || qself.position + 1 != path.path.segments.len()
        || !matches!(qself.ty.as_ref(), syn::Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("usize"))
        || crate::source_evidence::plain_relative_path(&path.path).is_none()
    {
        return Err(
            "Local value trait selected path is not its concrete usize implementation".into(),
        );
    }

    let method = path
        .path
        .segments
        .pop()
        .ok_or("Missing selected local trait member")?
        .into_value()
        .ident
        .to_string();
    path.path.segments.pop_punct();
    Ok((path.path, method))
}

pub(super) fn captured_signature(facts: &HirFnFacts) -> Result<(), String> {
    let signature = &facts.signature;
    let [parameter] = signature.params.as_slice() else {
        return Err("Local value trait captured signature lacks its single self operand".into());
    };
    if !signature.has_self
        || signature.is_const
        || signature.is_async
        || signature.is_unsafe
        || signature.is_variadic
        || !signature.generics.is_empty()
        || signature.abi.status != crate::FactStatus::Known
        || signature.abi.value.as_deref() != Some("Rust")
        || signature
            .receiver
            .as_ref()
            .is_none_or(|receiver| receiver.snippet != "self")
        || parameter.position != 0
        || parameter.ty.normalized.as_deref() != Some("usize")
        || parameter.pat.kind != "pat_binding"
        || parameter.pat.binding_mut != Some(false)
        || parameter.pat.snippet != "self"
        || parameter.pat.binding.as_ref() != Some(&parameter.symbol)
        || signature.return_ty.normalized.as_deref() != Some("u64")
    {
        return Err(
            "Local value trait captured signature contradicts original primitive self".into(),
        );
    }
    Ok(())
}

fn range(bytes: &str, method: &syn::ImplItemFn) -> ByteSpan {
    ByteSpan {
        start: crate::source_evidence::original_span(bytes, method.sig.span()).start,
        end: crate::source_evidence::original_span(bytes, method.block.span()).end,
    }
}
