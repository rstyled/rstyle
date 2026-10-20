//! Reference-valued source correspondence; no purity flags or erased origins.

use crate::{
    HirNode,
    data_types::{self, TypeToken},
    field_locations::{self, Projection},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KEY: &str = "rstyle.data:reference-origins";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub parameter: u32,
    pub path: Vec<Projection>,
}

pub fn shared(token: &TypeToken) -> bool {
    match token {
        TypeToken::Shared(_) => true,
        TypeToken::SliceArrayResult { borrowed: true, .. } => true,
        TypeToken::Tuple(elements) => elements.iter().any(shared),
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, field)| shared(field)),
        TypeToken::Array { element, .. } | TypeToken::Slice(element) => shared(element),
        _ => false,
    }
}

pub fn origins(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    if node.kind == "ty_annotation" {
        return Ok(Vec::new());
    }
    let Some(text) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) else {
        return Ok(Vec::new());
    };

    let ty = data_types::parse(text)?;
    if !shared(&ty) {
        return Ok(Vec::new());
    }

    if crate::data_reference_locals::flow_enabled(facts) {
        return crate::data_reference_locals::selected(node, &[], facts);
    }

    if let Some(origins) =
        crate::data_bytes::source_origins(node, facts, |operand| origins(operand, facts))
    {
        return origins;
    }
    let origins = source_operation(node, facts, text, &ty)?;

    Ok(origins
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

fn source_operation(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    text: &str,
    ty: &TypeToken,
) -> Result<Vec<Origin>, String> {
    Ok(match node.kind.as_str() {
        "expr_path" => formal(node, facts, ty)?,
        "expr_field" => field(node, facts, ty)?,
        "expr_unary" if crate::data_dereferences::candidate(node, facts) => {
            field_locations::derived::selected(node, &[], facts)?
        }
        "expr_tup" | "expr_struct" => constructed(node, facts)?,
        "expr_addr_of" => borrowed(node, facts)?,
        "expr_block" | "expr_ret" => {
            let value = node
                .children
                .last()
                .ok_or("Reference value lacks source operand")?;
            same_type(text, value)?;
            origins(value, facts)?
        }
        "expr_if" => alternatives(node, facts, text)?,
        "expr_call" => crate::helper_references::origins(node, facts)?,
        "expr_method_call" if crate::helper_calls::direct_resolution(node).is_some() => {
            crate::helper_references::origins(node, facts)?
        }
        "pat_binding" => Vec::new(),
        _ => return Err("Unsupported reference-valued source operation".into()),
    })
}

fn borrowed(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    if crate::data_subslices::borrowed(node, facts) {
        crate::data_subslices::verify_borrow(node, facts)?;
        return origins(
            crate::data_subslices::verify(&node.children[0], facts)?.base,
            facts,
        );
    }
    Ok(vec![crate::data_places::borrow_origin(node, facts)?])
}

fn constructed(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    if !crate::data_aggregates::reference_literal(node, facts) {
        return Err("Reference-bearing literals require their registered capability".into());
    }
    if crate::data_aggregates::update(node) {
        return crate::data_component_origins::updated(node, facts);
    }

    let mut roots = Vec::new();
    for child in &node.children {
        roots.extend(origins(child, facts)?);
    }
    Ok(roots)
}

fn same_type(expected: &str, node: &HirNode) -> Result<(), String> {
    if node.ty.as_ref().is_some_and(|ty| ty.observed == "!") {
        return Ok(());
    }
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(expected) {
        return Err("Reference value/source operand types disagree".into());
    }
    Ok(())
}

fn alternatives(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    expected: &str,
) -> Result<Vec<Origin>, String> {
    if node.children.len() != 3 {
        return Err("Reference branch requires both alternatives".into());
    }
    let mut values = Vec::new();
    for alternative in &node.children[1..] {
        same_type(expected, alternative)?;
        values.extend(origins(alternative, facts)?);
    }
    Ok(values)
}

fn formal(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    ty: &TypeToken,
) -> Result<Vec<Origin>, String> {
    let target = node
        .res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Missing reference binding resolution")?;
    let parameter = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *target);
    let Some(parameter) = parameter else {
        let initializer = crate::data_reference_locals::initializer(node, facts, ty)?;
        same_type(crate::data_aggregates::token(node)?, initializer)?;
        return origins(initializer, facts);
    };
    Ok(paths(ty, Vec::new())?
        .into_iter()
        .map(|path| Origin {
            parameter: parameter.position,
            path,
        })
        .collect())
}

fn field(
    node: &HirNode,
    facts: &dyn crate::unit_store::EvidenceFacts,
    ty: &TypeToken,
) -> Result<Vec<Origin>, String> {
    if crate::field_locations::derived::candidate(node, facts) {
        return crate::field_locations::derived::selected(node, &[], facts);
    }
    if crate::data_value_fields::candidate(node, facts) {
        return crate::data_component_origins::source(
            &node.children[0],
            &[crate::data_value_fields::index(node, facts)?],
            facts,
        );
    }
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    let field = fields
        .operations
        .iter()
        .find(|field| field.hir == node.hir)
        .ok_or("Missing reference field")?;
    let field_locations::Location::Captured {
        parameter,
        path,
        value,
    } = &field.location
    else {
        return Err("Unsupported reference field origin".into());
    };
    if data_types::TypeToken::from_tree(value) != *ty {
        return Err("Reference field type disagrees with origin".into());
    }

    Ok(paths(ty, path.clone())?
        .into_iter()
        .map(|path| Origin {
            parameter: *parameter,
            path,
        })
        .collect())
}

pub(crate) fn paths(
    ty: &TypeToken,
    prefix: Vec<Projection>,
) -> Result<Vec<Vec<Projection>>, String> {
    Ok(match ty {
        TypeToken::Shared(_) => vec![
            prefix
                .into_iter()
                .chain([Projection::SharedReferent])
                .collect(),
        ],
        TypeToken::Tuple(elements) => member_paths(
            elements
                .iter()
                .enumerate()
                .map(|(position, element)| (element, Projection::TupleField { position })),
            &prefix,
        )?,
        TypeToken::Struct { nominal, fields } => struct_paths(nominal, fields, &prefix)?,
        // Reference-bearing by-value arrays need element origin evidence.
        TypeToken::Array { element, .. } | TypeToken::Slice(element) if shared(element) => {
            return Err(
                "Reference-bearing elements require unavailable element-origin evidence".into(),
            );
        }
        _ => Vec::new(),
    })
}

fn struct_paths(
    nominal: &crate::concrete_types::Nominal,
    fields: &[(String, TypeToken)],
    prefix: &[Projection],
) -> Result<Vec<Vec<Projection>>, String> {
    member_paths(
        fields.iter().enumerate().map(|(position, (name, field))| {
            (
                field,
                Projection::StructField {
                    nominal: nominal.clone(),
                    position,
                    name: name.clone(),
                },
            )
        }),
        prefix,
    )
}

fn member_paths<'a>(
    members: impl Iterator<Item = (&'a TypeToken, Projection)>,
    prefix: &[Projection],
) -> Result<Vec<Vec<Projection>>, String> {
    members
        .map(|(member, projection)| {
            paths(member, prefix.iter().cloned().chain([projection]).collect())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|paths| paths.into_iter().flatten().collect())
}

pub fn aliases(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Vec<[u32; 2]>, String> {
    let mut roots = Vec::new();
    for parameter in &facts.signature.params {
        let ty = data_types::parse(
            parameter
                .ty
                .normalized
                .as_deref()
                .ok_or("Missing formal type")?,
        )?;
        if shared(&ty) {
            roots.push(parameter.position);
        }
    }
    Ok(roots
        .iter()
        .enumerate()
        .flat_map(|(index, left)| roots[index..].iter().map(move |right| [*left, *right]))
        .collect())
}

pub fn parameter(parameter: &crate::ParamFact) -> Result<Vec<Origin>, String> {
    let token = data_types::parse(
        parameter
            .ty
            .normalized
            .as_deref()
            .ok_or("Missing reference formal type")?,
    )?;
    Ok(paths(&token, Vec::new())?
        .into_iter()
        .map(|path| Origin {
            parameter: parameter.position,
            path,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_evidence_gaps_never_return_partial_origins() {
        let reference = TypeToken::Shared(Box::new(TypeToken::Scalar("i32".into())));
        for storage in [
            TypeToken::Array {
                element: Box::new(reference.clone()),
                length: 2,
            },
            TypeToken::Slice(Box::new(reference.clone())),
        ] {
            let mixed = TypeToken::Tuple(vec![reference.clone(), storage.clone()]);
            assert!(paths(&mixed, Vec::new()).is_err());
            let indirect = TypeToken::Shared(Box::new(storage));
            assert_eq!(
                paths(&indirect, Vec::new()).unwrap(),
                vec![vec![Projection::SharedReferent]]
            );
        }
    }
}
