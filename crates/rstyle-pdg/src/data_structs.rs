//! Named struct constructors bind source-ordered values to nominal field slots.

use crate::concrete_types::source_proof::PathResolution;
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use std::collections::{BTreeMap, BTreeSet};
use syn::{ext::IdentExt, spanned::Spanned};

pub(crate) fn slots(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Vec<usize>, String> {
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original struct construction source")?;
    let expression: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::Struct(syntax) = crate::data_aggregates::ungroup(&expression) else {
        return Err("Expected original named struct constructor".into());
    };
    if syntax.rest.is_some() || syntax.qself.is_some() {
        return Err("Struct update and qualified construction remain unavailable".into());
    }
    let token = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
    let TypeToken::Struct { fields, .. } = &token else {
        return Err("Constructor lacks nominal struct type".into());
    };
    verify_path(syntax, &token, facts)?;
    if syntax.fields.len() != fields.len() || node.children.len() != fields.len() {
        return Err("Struct constructor requires every declared field exactly once".into());
    }

    let declarations: BTreeMap<_, _> = fields
        .iter()
        .enumerate()
        .map(|(index, (name, ty))| (name.as_str(), (index, ty)))
        .collect();
    let context = ComponentContext {
        node,
        source,
        facts,
    };
    let mut seen = BTreeSet::new();
    let mut slots = Vec::new();
    for (field, child) in syntax.fields.iter().zip(&node.children) {
        let syn::Member::Named(name) = &field.member else {
            return Err("Only named struct fields are supported".into());
        };
        let name = name.unraw().to_string();
        let &(index, expected) = declarations
            .get(name.as_str())
            .ok_or("Unknown constructed field")?;
        if !seen.insert(index) {
            return Err("Duplicate constructed field".into());
        }
        context.verify(field, child, expected)?;
        slots.push(index);
    }
    Ok(slots)
}

fn verify_path(
    syntax: &syn::ExprStruct,
    expected: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let resolution = facts.nominal_resolution()?;
    let path = syn::TypePath {
        qself: None,
        path: syntax.path.clone(),
    };
    let tree = resolution.resolve(&path)?;

    if &TypeToken::from_tree(&tree) != expected {
        return Err("Struct constructor path disagrees with exact nominal declaration".into());
    }
    Ok(())
}

struct ComponentContext<'a> {
    node: &'a HirNode,
    source: &'a str,
    facts: &'a dyn EvidenceFacts,
}

impl ComponentContext<'_> {
    fn verify(
        &self,
        field: &syn::FieldValue,
        child: &HirNode,
        expected: &TypeToken,
    ) -> Result<(), String> {
        if expected != &crate::data_types::parse(crate::data_aggregates::token(child)?)?
            || child.ty.as_ref().is_some_and(|ty| ty.observed == "!")
        {
            return Err("Constructed field type disagrees with its nominal slot".into());
        }

        let span = crate::data_aggregates::ungroup(&field.expr).span();
        let syntax = crate::ByteSpan {
            start: self.node.span.start
                + crate::source_evidence::byte_offset(self.source, span.start()) as u32,
            end: self.node.span.start
                + crate::source_evidence::byte_offset(self.source, span.end()) as u32,
        };
        if !crate::source_evidence::corresponding_span(&self.facts.source.bytes, syntax, child.span)
        {
            return Err("Constructed field loses original evaluation order/correspondence".into());
        }
        Ok(())
    }
}

pub(crate) fn descriptor(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<String, String> {
    serde_json::to_string(&slots(node, facts)?).map_err(|error| error.to_string())
}
