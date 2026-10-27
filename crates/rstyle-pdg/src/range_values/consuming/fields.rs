use crate::{HirNode, ParamFact, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::{ext::IdentExt, spanned::Spanned};

/// Exact selectors retained from one complete source-checked From body.
pub(crate) struct Fields<'a> {
    formal: &'a ParamFact,
    literal: &'a HirNode,
    selectors: [&'a HirNode; 2],
}

impl Fields<'_> {
    pub(crate) fn position(&self, node: &HirNode) -> Result<Option<usize>, String> {
        let Some(position) = self
            .selectors
            .iter()
            .position(|field| field.hir == node.hir)
        else {
            return Ok(None);
        };
        if self.selectors[position] != node {
            return Err("Range selector differs from its checked complete node".into());
        }
        Ok(Some(position))
    }
}

pub(crate) fn verify_fields(facts: &dyn EvidenceFacts) -> Result<Fields<'_>, String> {
    let formal = super::formal::verify(facts)?;
    if facts.signature.params.len() != 1 || formal.position != 0 || facts.bindings.len() != 1 {
        return Err("Range From requires only its initialized formal and no locals".into());
    }

    let literal = literal(facts)?;
    let [start, end] = literal.children.as_slice() else {
        return Err("Range From requires exactly its two scalar selectors".into());
    };
    let checked = Fields {
        formal,
        literal,
        selectors: [start, end],
    };

    source(&checked, facts)?;
    for (position, field) in checked.selectors.iter().enumerate() {
        selector(field, formal, position, facts)?;
    }

    Ok(checked)
}

fn literal(facts: &dyn EvidenceFacts) -> Result<&HirNode, String> {
    let [literal] = facts.body.children.as_slice() else {
        return Err("Range From requires one complete tail expression".into());
    };
    if facts.body.kind != "expr_block"
        || facts.body.expansion.is_some()
        || literal.kind != "expr_struct"
        || literal.expansion.is_some()
        || crate::data_aggregates::token(literal)?
            != facts
                .signature
                .return_ty
                .normalized
                .as_deref()
                .ok_or("Missing Range From result type")?
    {
        return Err("Range From must return its complete original Span literal".into());
    }

    let TypeToken::Struct { nominal, fields } =
        crate::data_types::parse(crate::data_aggregates::token(literal)?)?
    else {
        return Err("Range From requires its concrete Span result".into());
    };
    if nominal.definition_path != "util::search::Span"
        || fields
            != [
                ("start".into(), TypeToken::Scalar("usize".into())),
                ("end".into(), TypeToken::Scalar("usize".into())),
            ]
    {
        return Err("Range From result loses its exact ordered Span components".into());
    }
    Ok(literal)
}

fn selector(
    node: &HirNode,
    formal: &ParamFact,
    position: usize,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let [base] = node.children.as_slice() else {
        return Err("Range selector requires exactly its original formal base".into());
    };
    if node.kind != "expr_field"
        || !super::formal::read(base, formal)
        || crate::data_aggregates::token(node)? != "usize"
    {
        return Err("Range selector is not an immutable usize field of the consumed formal".into());
    }

    let resolution = node
        .res
        .as_ref()
        .filter(|r| r.kind == "resolved_field" && r.target.is_none())
        .ok_or("Missing Range field resolution")?;
    let field: crate::field_locations::FieldResolution =
        serde_json::from_str(&resolution.description).map_err(|e| e.to_string())?;
    if field.position != position
        || !matches!(
            field.owner_path.as_deref(),
            Some("core::ops::Range" | "std::ops::Range")
        )
    {
        return Err("Range selector disagrees with the pinned ordered field owner".into());
    }

    super::source::node_source(node, facts)?;
    super::source::node_source(base, facts)
}

fn source(checked: &Fields<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let function = super::source::function(facts)?;
    crate::source_evidence::closed_body(&function.block)?;
    let name = super::source::formal_name(&function, checked.formal, facts)?;
    let Some(syn::FnArg::Typed(input)) = function.sig.inputs.first() else {
        return Err("Missing concrete Range source input".into());
    };
    if function.sig.inputs.len() != 1
        || !function.sig.generics.params.is_empty()
        || !super::source::range_type(&input.ty)
    {
        return Err("Range From source requires one concrete formal".into());
    }

    let [syn::Stmt::Expr(syn::Expr::Struct(literal), None)] = function.block.stmts.as_slice()
    else {
        return Err("Range From source requires only its complete tail literal".into());
    };
    if literal.rest.is_some()
        || literal.qself.is_some()
        || literal.fields.len() != 2
        || literal
            .path
            .segments
            .last()
            .is_none_or(|s| s.ident != "Span")
        || !super::source::corresponds(literal.span(), checked.literal.span, facts)
    {
        return Err("Range From source is not the exact complete Span reconstruction".into());
    }

    for (position, (field, member)) in literal.fields.iter().zip(["start", "end"]).enumerate() {
        source_field(field, member, &name)?;
        if !super::source::corresponds(field.expr.span(), checked.selectors[position].span, facts) {
            return Err("Range From selectors lose source order or correspondence".into());
        }
    }
    Ok(())
}

pub(super) fn source_field(
    field: &syn::FieldValue,
    expected: &str,
    formal: &str,
) -> Result<(), String> {
    let syn::Member::Named(member) = &field.member else {
        return Err("Range From requires named Span components".into());
    };
    let syn::Expr::Field(value) = &field.expr else {
        return Err("Range From endpoint must be one exact field read".into());
    };
    let syn::Member::Named(selected) = &value.member else {
        return Err("Range From requires named Range endpoints".into());
    };

    if member.unraw() != expected
        || selected.unraw() != expected
        || field.colon_token.is_none()
        || !crate::source_evidence::plain_value_path(&value.base, formal)
    {
        return Err("Range From endpoints are swapped, duplicated, borrowed or replaced".into());
    }
    Ok(())
}
