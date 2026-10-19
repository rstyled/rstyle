//! Named struct constructors bind source-ordered values to nominal field slots.

use crate::concrete_types::source_proof::PathResolution;
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use std::collections::{BTreeMap, BTreeSet};
use syn::{ext::IdentExt, spanned::Spanned};

pub(crate) fn slots(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Option<usize>>, String> {
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing original struct construction source")?;
    let expression: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::Struct(syntax) = crate::data_aggregates::ungroup(&expression) else {
        return Err("Expected original named struct constructor".into());
    };
    if syntax.qself.is_some() {
        return Err("Qualified struct construction remains unavailable".into());
    }

    let token = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
    let TypeToken::Struct { fields, .. } = &token else {
        return Err("Constructor lacks nominal struct type".into());
    };
    verify_path(syntax, &token, facts)?;
    let context = ComponentContext {
        node,
        source,
        facts,
    };

    context.slots(syntax, fields, &token)
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
    fn slots(
        &self,
        syntax: &syn::ExprStruct,
        fields: &[(String, TypeToken)],
        token: &TypeToken,
    ) -> Result<Vec<Option<usize>>, String> {
        let update = syntax.rest.is_some();
        if update
            && !crate::extraction_profile::ExtractionProfile::from_facts(self.facts)?
                .supports("rust.struct-update-values")
        {
            return Err("Struct update requires its registered capability".into());
        }
        if (!update && syntax.fields.len() != fields.len())
            || self.node.children.len() != syntax.fields.len() + usize::from(update)
        {
            return Err("Struct constructor field/base arity disagrees".into());
        }

        let declarations: BTreeMap<_, _> = fields
            .iter()
            .enumerate()
            .map(|(index, (name, ty))| (name.as_str(), (index, ty)))
            .collect();
        let mut seen = BTreeSet::new();
        let mut slots = Vec::new();
        for (field, child) in syntax.fields.iter().zip(&self.node.children) {
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
            self.verify(&field.expr, child, expected)?;
            slots.push(Some(index));
        }
        if let Some(base) = &syntax.rest {
            self.verify(
                base,
                self.node.children.last().expect("checked base arity"),
                token,
            )?;
            slots.push(None);
        }
        Ok(slots)
    }

    fn verify(
        &self,
        expression: &syn::Expr,
        child: &HirNode,
        expected: &TypeToken,
    ) -> Result<(), String> {
        if expected != &crate::data_types::parse(crate::data_aggregates::token(child)?)?
            || child.ty.as_ref().is_some_and(|ty| ty.observed == "!")
        {
            return Err("Constructed field type disagrees with its nominal slot".into());
        }

        let span = crate::data_aggregates::ungroup(expression).span();
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
