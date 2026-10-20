//! Bounded shared-slice views. Range syntax is a structural carrier, not a
//! fabricated unit-valued operation or a generally supported Range value.

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub const PREFIX_CAPABILITY: &str = "rust.native-prefix-readers";
pub const ARRAY_CAPABILITY: &str = "rust.borrowed-array-views";

pub(crate) const BOUNDS_KEY: &str = "rstyle.data:subslice-bounds";

pub(crate) struct Checked<'a> {
    pub base: &'a HirNode,
    pub start: Option<&'a HirNode>,
    pub end: &'a HirNode,
    pub requires_prefix_policy: bool,
    pub array_length: Option<u64>,
}

impl<'a> Checked<'a> {
    pub fn operands(&self) -> impl Iterator<Item = &'a HirNode> {
        std::iter::once(self.base)
            .chain(self.start)
            .chain(std::iter::once(self.end))
    }

    pub fn bounds(&self) -> crate::data_bounds::Bounds {
        if self.start.is_some() {
            crate::data_bounds::Bounds::subslice(self.array_length)
        } else {
            crate::data_bounds::Bounds::prefix(self.array_length)
        }
    }
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    let [_, range] = node.children.as_slice() else {
        return false;
    };
    node.kind == "expr_index"
        && range.kind == "expr_struct"
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.kind == "overloaded" && dispatch.op == "[]")
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|profile| profile.supports("rust.shared-subslice-views"))
}

pub(crate) fn borrowed(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    match node.children.as_slice() {
        [place] if node.kind == "expr_addr_of" => candidate(place, facts),
        _ => false,
    }
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts) {
        return Err("Subslice requires registered overloaded range indexing".into());
    }
    let [base, range] = node.children.as_slice() else {
        return Err("Subslice lacks exact base and range operands".into());
    };
    let (start, end) = match range.children.as_slice() {
        [start, end] => (Some(start), end),
        [end] if crate::extraction_profile::facts_support(facts, PREFIX_CAPABILITY) => (None, end),
        _ => return Err("Subslice requires registered exclusive range endpoints".into()),
    };
    let base_ty = crate::data_types::parse(crate::data_aggregates::token(base)?)?;
    let (referent, array_length) = base_type(base, base_ty, facts)?;
    if !matches!(&referent, TypeToken::Slice(element) if matches!(element.as_ref(), TypeToken::Scalar(_)))
        || crate::data_types::parse(crate::data_aggregates::token(node)?)? != referent
        || start.is_some_and(|start| crate::data_aggregates::token(start) != Ok("usize"))
        || crate::data_aggregates::token(end)? != "usize"
    {
        return Err("Subslice requires a scalar slice and exact usize endpoints".into());
    }

    verify_dispatch(node, base, range)?;

    let core_index = authority(node, "resolved_callee", "std::ops::Index::index", facts)?;
    let range_path = if start.is_some() {
        "std::ops::Range"
    } else {
        "std::ops::RangeTo"
    };
    let core_range = authority(range, "resolved_range_type", range_path, facts)?;
    source(node, base, range, facts)?;
    Ok(Checked {
        base,
        start,
        end,
        requires_prefix_policy: start.is_none() || core_index || core_range,
        array_length,
    })
}

fn base_type(
    base: &HirNode,
    ty: TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(TypeToken, Option<u64>), String> {
    match ty {
        TypeToken::Shared(referent) => Ok((*referent, None)),
        TypeToken::Array { element, length }
            if crate::extraction_profile::facts_support(facts, ARRAY_CAPABILITY) =>
        {
            crate::data_places::field_origin(base, facts)?;
            Ok((TypeToken::Slice(element), Some(length)))
        }
        _ => Err("Subslice base requires a shared slice or checked formal array place".into()),
    }
}

fn verify_dispatch(node: &HirNode, base: &HirNode, range: &HirNode) -> Result<(), String> {
    let dispatch = node.dispatch.as_ref().ok_or("Missing subslice dispatch")?;
    let range_ty = range.ty.as_ref().ok_or("Missing range type observation")?;
    if dispatch.op != "[]"
        || range.kind != "expr_struct"
        || range.expansion.is_none()
        || range_ty.normalized.is_some()
        || dispatch.operand_tys
            != [
                base.ty
                    .as_ref()
                    .ok_or("Missing subslice base observation")?
                    .observed
                    .as_str(),
                range_ty.observed.as_str(),
            ]
        || dispatch.result_ty
            != node
                .ty
                .as_ref()
                .ok_or("Missing subslice result observation")?
                .observed
    {
        return Err("Subslice dispatch/type correspondence disagrees".into());
    }

    Ok(())
}

fn authority(
    node: &HirNode,
    kind: &str,
    path: &str,
    facts: &dyn EvidenceFacts,
) -> Result<bool, String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned subslice compiler/target")?;
    let resolution = node
        .res
        .as_ref()
        .filter(|res| res.kind == kind)
        .ok_or("Missing resolved subslice authority")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    let alternate = crate::extraction_profile::facts_support(facts, PREFIX_CAPABILITY)
        && callee.path.strip_prefix("core::") == path.strip_prefix("std::");
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || (callee.path != path && !alternate)
        || registry.definitions.get(path) != Some(&callee.definition_hash)
        || callee.implementation.is_some()
    {
        return Err("Subslice disagrees with pinned core index/range authority".into());
    }
    Ok(alternate)
}

fn source(
    node: &HirNode,
    base: &HirNode,
    range: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original subslice source")?;
    let syntax: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let syn::Expr::Index(index) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Subslice lacks original indexing syntax".into());
    };
    let syn::Expr::Range(bounds) = crate::data_aggregates::ungroup(&index.index) else {
        return Err("Subslice lacks original range syntax".into());
    };
    if !matches!(bounds.limits, syn::RangeLimits::HalfOpen(_)) {
        return Err("Only exclusive subslice endpoints are registered".into());
    }
    if bounds.end.is_none()
        || bounds.start.is_none()
            && !crate::extraction_profile::facts_support(facts, PREFIX_CAPABILITY)
        || range.children.len() != 1 + usize::from(bounds.start.is_some())
    {
        return Err("Subslice source lacks its registered exclusive endpoints".into());
    }
    let syntax_operands = std::iter::once(index.expr.as_ref())
        .chain(std::iter::once(index.index.as_ref()))
        .chain(bounds.start.as_deref())
        .chain(bounds.end.as_deref());
    let hir_operands = std::iter::once(base)
        .chain(std::iter::once(range))
        .chain(range.children.iter());

    for (syntax, hir) in syntax_operands.zip(hir_operands) {
        let span = crate::data_aggregates::ungroup(syntax).span();
        let span = crate::ByteSpan {
            start: node.span.start
                + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
            end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
        };
        if !crate::source_evidence::corresponding_span(&facts.source.bytes, span, hir.span) {
            return Err("Subslice loses original base/endpoint correspondence".into());
        }
    }
    Ok(())
}

pub(crate) fn verify_borrow<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    let [place] = node.children.as_slice() else {
        return Err("Subslice borrow lacks one place".into());
    };

    let checked = verify(place, facts)?;
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original subslice borrow")?;
    let syntax: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let syn::Expr::Reference(borrow) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Subslice borrow lacks original reference syntax".into());
    };

    let expression = crate::data_aggregates::ungroup(&borrow.expr).span();
    let expression = crate::ByteSpan {
        start: node.span.start
            + crate::source_evidence::byte_offset(bytes, expression.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(bytes, expression.end()) as u32,
    };
    if !crate::source_evidence::corresponding_span(&facts.source.bytes, expression, place.span) {
        return Err("Subslice borrow loses exact place correspondence".into());
    }

    let expected = TypeToken::Shared(Box::new(crate::data_types::parse(
        crate::data_aggregates::token(place)?,
    )?));
    if borrow.mutability.is_some()
        || crate::data_types::parse(crate::data_aggregates::token(node)?)? != expected
    {
        return Err("Subslice borrow loses immutable result type".into());
    }
    Ok(checked)
}
