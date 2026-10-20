//! Bounded shared-slice views. Range syntax is a structural carrier, not a
//! fabricated unit-valued operation or a generally supported Range value.

use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) const BOUNDS_KEY: &str = "rstyle.data:subslice-bounds";
const INDEX: &str = "DefPathHash(Fingerprint(5614226571623188065, 15938615241105906548))";
const RANGE: &str = "DefPathHash(Fingerprint(5614226571623188065, 860015315731983206))";

pub(crate) struct Checked<'a> {
    pub base: &'a HirNode,
    pub start: &'a HirNode,
    pub end: &'a HirNode,
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
    let [start, end] = range.children.as_slice() else {
        return Err("Subslice requires both exclusive range endpoints".into());
    };
    let base_ty = crate::data_types::parse(crate::data_aggregates::token(base)?)?;
    let TypeToken::Shared(referent) = base_ty else {
        return Err("Subslice base requires a shared slice reference".into());
    };
    if !matches!(referent.as_ref(), TypeToken::Slice(element) if matches!(element.as_ref(), TypeToken::Scalar(_)))
        || crate::data_types::parse(crate::data_aggregates::token(node)?)? != *referent
        || crate::data_aggregates::token(start)? != "usize"
        || crate::data_aggregates::token(end)? != "usize"
    {
        return Err("Subslice requires a scalar slice and exact usize endpoints".into());
    }

    verify_dispatch(node, base, range)?;

    authority(node, "resolved_callee", INDEX, facts)?;
    authority(range, "resolved_range_type", RANGE, facts)?;
    source(node, base, range, facts)?;
    Ok(Checked { base, start, end })
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
    identity: &str,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned subslice compiler/target")?;
    // This implementation's independently inspected core authority is aarch64.
    if registry.target != "aarch64-unknown-linux-gnu" {
        return Err("Subslice authority is not registered for this target".into());
    }

    let resolution = node
        .res
        .as_ref()
        .filter(|res| res.kind == kind)
        .ok_or("Missing resolved subslice authority")?;
    let callee: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || callee.definition_hash != identity
    {
        return Err("Subslice disagrees with pinned core index/range authority".into());
    }
    Ok(())
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
    let start = bounds
        .start
        .as_ref()
        .ok_or("Missing subslice lower endpoint")?;
    let end = bounds
        .end
        .as_ref()
        .ok_or("Missing subslice upper endpoint")?;

    for (syntax, hir) in [
        (&*index.expr, base),
        (&*index.index, range),
        (&**start, &range.children[0]),
        (&**end, &range.children[1]),
    ] {
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

pub(crate) fn verify_borrow(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let [place] = node.children.as_slice() else {
        return Err("Subslice borrow lacks one place".into());
    };

    verify(place, facts)?;
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
    Ok(())
}
