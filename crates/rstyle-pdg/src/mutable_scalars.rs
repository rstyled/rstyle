//! Formal scalar referents remain distinct from lexical reference descriptors.
use crate::{HirNode, field_locations::scalar_formals::*, unit_store::EvidenceFacts};

pub(crate) const CAPABILITY: &str = "rust.mutable-scalar-formals";
pub(crate) const LOAD: &str = "data.mutable-scalar-load";
pub(crate) const STORE: &str = "data.mutable-scalar-store";

pub(crate) fn enabled(facts: &dyn EvidenceFacts) -> bool {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return false;
    }
    facts
        .signature
        .params
        .iter()
        .any(|parameter| parameter.ty.observed == "&mut usize")
}

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !enabled(facts) {
        return Ok(());
    }
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    let evidence = fields
        .scalar_formal_accesses
        .as_ref()
        .ok_or("Mutable scalar interface lacks its complete access inventory")?;
    super::field_locations::scalar_formals::verify(facts.facts(), evidence)?;
    if facts.signature.has_self || facts.signature.return_ty.normalized.as_deref() != Some("unit") {
        return Err("Mutable scalar family requires direct formals and unit result".into());
    }

    verify_body(&facts.body, facts)
}

fn verify_body(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    match node.kind.as_str() {
        "expr_block" | "expr_while" | "stmt_expr" | "stmt_semi" | "expr_binary" | "expr_lit"
        | "expr_path" => {}
        "expr_unary" if root(node, facts).is_some() => {}
        "expr_assign_op"
            if node
                .children
                .first()
                .is_some_and(|target| root(target, facts).is_some()) => {}
        "expr_index" => {
            index_root(node, facts)?;
        }
        _ => {
            return Err(
                "Mutable scalar family contains an unsupported operation or storage lifetime"
                    .into(),
            );
        }
    }
    for child in &node.children {
        verify_body(child, facts)?;
    }
    Ok(())
}

/// Lookup only; the complete inventory is verified at the frontend boundary.
pub(crate) fn root(node: &HirNode, facts: &dyn EvidenceFacts) -> Option<u32> {
    if !enabled(facts) || node.kind != "expr_unary" {
        return None;
    }
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()
        .ok()?;
    fields
        .scalar_formal_accesses
        .as_ref()?
        .accesses
        .iter()
        .find(|access| {
            access.hir == node.hir && access.span == node.span && access.kind == AccessKind::Load
        })
        .map(|access| access.parameter)
}

pub(crate) fn index_root(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<u32, String> {
    let checked = crate::data_index_reads::verify(node, facts)?;
    if !checked.shared || checked.base.kind != "expr_path" {
        return Err("Mutable scalar family requires a direct shared byte-slice index base".into());
    }
    let target = checked
        .base
        .res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Missing indexed formal binding")?;
    facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *target && parameter.ty.observed == "&[u8]")
        .map(|parameter| parameter.position)
        .ok_or_else(|| "Mutable scalar family index base is not a shared byte-slice formal".into())
}

pub(crate) fn memory_edge(edge: &crate::PdgEdge, profile: &crate::ProfileId) -> bool {
    crate::extraction_profile::policy_supports(profile, CAPABILITY)
        && edge.location_id.is_some()
        && edge.symbol_id.is_none()
}

pub(crate) fn lexical_reaching(edge: &crate::PdgEdge, profile: &crate::ProfileId) -> bool {
    edge.subkind == "reaching_definition" && !memory_edge(edge, profile)
}
