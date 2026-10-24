//! Exact generated tuple declarations, distinct from original-byte declarations.
use super::{capture::*, checked::CheckedMember};
use crate::nominal_boundary::census::Census;
use crate::{HirFnFacts, concrete_types::TypeTree, declaration_origins::GeneratedOrigin};
use syn::spanned::Spanned;

pub(crate) struct Input<'a> {
    pub facts: &'a HirFnFacts,
    pub census: &'a Census,
    pub capture: &'a UnitCapture,
    pub member: &'a CheckedMember,
    pub tree: &'a TypeTree,
    pub origin: &'a GeneratedOrigin,
}

/// Constructed only by independent source, field and compiler-definition checks.
pub(crate) struct CheckedGeneratedDeclaration {
    tree: TypeTree,
    item: syn::ItemStruct,
    namespace: Vec<String>,
    constructor: Definition,
}

impl CheckedGeneratedDeclaration {
    pub(crate) fn tree(&self) -> &TypeTree {
        &self.tree
    }
    pub(crate) fn item(&self) -> &syn::ItemStruct {
        &self.item
    }
    pub(crate) fn namespace(&self) -> &[String] {
        &self.namespace
    }
    pub(crate) fn constructor(&self) -> &Definition {
        &self.constructor
    }
}

pub(crate) fn verify(input: Input<'_>) -> Result<CheckedGeneratedDeclaration, String> {
    let (nominal, definition, template, fields) = input
        .tree
        .generated_parts()
        .ok_or("Generated declaration proof requires its explicit observed type")?;
    if &input.origin.nominal != nominal || &input.origin.definition != definition {
        return Err("Generated declaration origin identifies another exact type".into());
    }
    local(input.facts, definition)?;
    original(&input, template)?;
    let item = selected(&input, definition)?;
    field(&input, item, fields)?;
    let constructor = emitted(&input, definition)?;

    Ok(CheckedGeneratedDeclaration {
        tree: input.tree.clone(),
        item: item.clone(),
        namespace: input.member.namespace().to_vec(),
        constructor,
    })
}

fn local(facts: &HirFnFacts, definition: &Definition) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Generated declaration lacks compiled crate identity")?;
    if !definition.local
        || definition.definition_identity.is_empty()
        || compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || compiled["crate_name"].as_str() != Some(&definition.crate_name)
        || compiled["stable_crate_id"].as_str() != Some(&definition.stable_crate_id)
    {
        return Err("Generated declaration belongs to another exact compiled crate".into());
    }
    Ok(())
}

fn original(input: &Input<'_>, template: &crate::SourceText) -> Result<(), String> {
    let raw = &input.origin.origin.raw;
    let file = input
        .census
        .files
        .iter()
        .find(|file| file.path == raw.path)
        .ok_or("Generated declaration lacks its original census file")?;
    if file.bytes != input.origin.file_bytes
        || file.digest != raw.digest
        || !file.bytes.matches_identity(&raw.digest)
        || file
            .bytes
            .get(raw.span.start as usize..raw.span.end as usize)
            != Some(template.as_str())
        || !template.matches_identity(&input.origin.nominal.declaration_digest)
    {
        return Err("Generated declaration template differs from original compiler bytes".into());
    }

    let owner_identity = input
        .facts
        .build
        .frontend_options
        .get("definition_identity")
        .and_then(serde_json::Value::as_str)
        .ok_or("Generated declaration lacks exact owning function identity")?;
    if input.member.owner().definition_identity != owner_identity
        || input.member.defining_expansion() != input.origin.defining_expansion
        || input.origin.origin.marks
            != [Mark {
                expansion: input.origin.defining_expansion.clone(),
                transparency: Transparency::SemiOpaque,
            }]
    {
        return Err(
            "Generated declaration belongs to another exact checked member expansion".into(),
        );
    }
    Ok(())
}

fn selected<'a>(
    input: &'a Input<'_>,
    definition: &Definition,
) -> Result<&'a syn::ItemStruct, String> {
    let expansion = input.member.expansion();
    let mut candidates = expansion.items().iter().filter_map(|item| match item {
        syn::Item::Struct(item)
            if item_span(item)
                .is_ok_and(|span| input.origin.origin.raw == expansion.source_span(span)) =>
        {
            Some(item)
        }
        _ => None,
    });
    let item = candidates
        .next()
        .ok_or("Generated declaration has no exact complete source item")?;
    if candidates.next().is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || definition.path
            != expansion
                .namespace()
                .iter()
                .cloned()
                .chain([item.ident.to_string()])
                .collect::<Vec<_>>()
                .join("::")
    {
        return Err("Generated declaration source inventory or substituted name disagrees".into());
    }

    Ok(item)
}

fn field(
    input: &Input<'_>,
    item: &syn::ItemStruct,
    fields: &[crate::concrete_types::Field],
) -> Result<(), String> {
    let syn::Fields::Unnamed(source) = &item.fields else {
        return Err("Generated cleanup declaration requires a unary tuple".into());
    };
    let source_fields: Vec<_> = source.unnamed.iter().collect();
    let ([source], [field]) = (source_fields.as_slice(), fields) else {
        return Err("Generated tuple field inventory differs".into());
    };
    let syn::Type::Path(path) = &source.ty else {
        return Err("Generated tuple field is not an exact concrete path".into());
    };
    if path.qself.is_some()
        || path
            .path
            .segments
            .iter()
            .any(|part| !part.arguments.is_empty())
        || field.position != 0
        || field.name != "0"
    {
        return Err("Generated tuple field substitution or position is unavailable".into());
    }

    payload_identity(input, path, &field.ty)?;

    let span = input.member.expansion().source_span(source.ty.span());
    let file = input
        .census
        .files
        .iter()
        .find(|file| file.path == span.path)
        .ok_or("Generated tuple field lacks original source")?;
    if file.digest != span.digest
        || file
            .bytes
            .get(span.span.start as usize..span.span.end as usize)
            != Some(field.declaration.as_str())
    {
        return Err("Generated tuple field bytes differ from exact original type syntax".into());
    }
    Ok(())
}

fn payload_identity(
    input: &Input<'_>,
    source: &syn::TypePath,
    observed: &TypeTree,
) -> Result<(), String> {
    let name = source
        .path
        .get_ident()
        .ok_or("Generated tuple payload requires a plain local type name")?;
    let TypeTree::Struct { nominal, .. } = observed else {
        return Err(
            "Generated tuple cleanup payload requires an original scalar error struct".into(),
        );
    };
    let expected = input
        .member
        .namespace()
        .iter()
        .cloned()
        .chain([name.to_string()])
        .collect::<Vec<_>>()
        .join("::");

    if nominal.definition_path != expected {
        return Err(
            "Generated tuple payload normalized identity differs from original source name".into(),
        );
    }
    Ok(())
}

fn item_span(item: &syn::ItemStruct) -> Result<proc_macro2::Span, String> {
    let start = match &item.vis {
        syn::Visibility::Inherited => item.struct_token.span,
        visibility => visibility.span(),
    };
    let end = item
        .semi_token
        .as_ref()
        .ok_or("Generated tuple declaration lacks its semicolon")?
        .span;
    start
        .join(end)
        .ok_or_else(|| "Generated declaration has noncontiguous original coordinates".into())
}

fn emitted(input: &Input<'_>, definition: &Definition) -> Result<Definition, String> {
    let mut matching = input
        .capture
        .emitted
        .iter()
        .filter(|entry| entry.definition.definition_identity == definition.definition_identity);
    let structure = matching.next().ok_or("Missing emitted generated struct")?;
    if matching.next().is_some()
        || &structure.definition != definition
        || structure.kind != "Struct"
        || structure.origin != input.origin.origin
        || structure.parent.path != input.member.namespace().join("::")
    {
        return Err("Generated struct emitted identity, source or parent disagrees".into());
    }
    local(input.facts, &structure.parent)?;
    if &structure.parent != module_owner(input)? {
        return Err(
            "Generated struct and checked callable belong to different exact modules".into(),
        );
    }

    let children: Vec<_> = input
        .capture
        .emitted
        .iter()
        .filter(|entry| &entry.parent == definition)
        .collect();
    if children.len() != 2 {
        return Err(
            "Generated unary struct direct-definition roster is incomplete or has surplus items"
                .into(),
        );
    }
    let constructor = child(input, &children, "Ctor(Struct, Fn)")?;
    let field = child(input, &children, "Field")?;
    if constructor.definition.path != definition.path
        || constructor.origin != structure.origin
        || field.definition.path != format!("{}::0", definition.path)
    {
        return Err(
            "Generated tuple constructor or field differs from its exact emitted owner".into(),
        );
    }
    field_origin(input, field)?;
    Ok(constructor.definition.clone())
}

fn module_owner<'a>(input: &'a Input<'_>) -> Result<&'a Definition, String> {
    let mut observations = input
        .capture
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            Outcome::Observed { observation } if &observation.owner == input.member.owner() => {
                Some(observation)
            }
            _ => None,
        });
    let observation = observations
        .next()
        .ok_or("Missing checked member's exact captured observation")?;
    if observations.next().is_some() {
        return Err("Duplicate checked member observation".into());
    }

    let mut implementations = input
        .capture
        .emitted
        .iter()
        .filter(|entry| entry.definition == observation.implementation);
    let implementation = implementations
        .next()
        .ok_or("Missing checked member implementation")?;
    if implementations.next().is_some() {
        return Err("Duplicate checked member implementation".into());
    }
    Ok(&implementation.parent)
}

fn child<'a>(
    input: &Input<'_>,
    children: &[&'a Emitted],
    kind: &str,
) -> Result<&'a Emitted, String> {
    let mut matches = children.iter().copied().filter(|entry| entry.kind == kind);
    let child = matches
        .next()
        .ok_or("Missing generated tuple direct definition")?;
    if matches.next().is_some() {
        return Err("Duplicate generated tuple direct definition".into());
    }

    local(input.facts, &child.definition)?;
    if child.definition.definition_identity == child.parent.definition_identity {
        return Err("Generated direct child reuses its owner identity".into());
    }
    Ok(child)
}

fn field_origin(input: &Input<'_>, emitted: &Emitted) -> Result<(), String> {
    let item = selected(input, &input.origin.definition)?;
    let syn::Fields::Unnamed(fields) = &item.fields else {
        return Err("Generated field owner is not a tuple".into());
    };
    let field = fields
        .unnamed
        .first()
        .ok_or("Missing generated tuple field")?;

    if emitted.origin.raw != input.member.expansion().source_span(field.span())
        || emitted.origin.marks != input.origin.origin.marks
    {
        return Err("Generated tuple field lacks its exact emitted original source anchor".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
