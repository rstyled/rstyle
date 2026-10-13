//! Reference-valued source correspondence; no purity flags or erased origins.

use crate::{
    HirFnFacts, HirNode,
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
        TypeToken::Tuple(elements) => elements.iter().any(shared),
        TypeToken::Struct { fields, .. } => fields.iter().any(|(_, field)| shared(field)),
        TypeToken::Array { element, .. } | TypeToken::Slice(element) => shared(element),
        _ => false,
    }
}

pub fn origins(node: &HirNode, facts: &HirFnFacts) -> Result<Vec<Origin>, String> {
    let Some(text) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) else {
        return Ok(Vec::new());
    };

    let ty = data_types::parse(text)?;
    if !shared(&ty) {
        return Ok(Vec::new());
    }

    let origins = match node.kind.as_str() {
        "expr_path" => formal(node, facts, &ty)?,
        "expr_field" => field(node, facts, &ty)?,
        "expr_addr_of" => vec![crate::data_places::borrow_origin(node, facts)?],
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
        "pat_binding" => Vec::new(),
        _ => return Err("Unsupported reference-valued source operation".into()),
    };
    Ok(origins
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
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

fn alternatives(node: &HirNode, facts: &HirFnFacts, expected: &str) -> Result<Vec<Origin>, String> {
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

fn formal(node: &HirNode, facts: &HirFnFacts, ty: &TypeToken) -> Result<Vec<Origin>, String> {
    let target = node
        .res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Missing reference binding resolution")?;
    let parameter = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *target)
        .ok_or("Reference local origin proof is unavailable")?;
    Ok(paths(ty, Vec::new())?
        .into_iter()
        .map(|path| Origin {
            parameter: parameter.position,
            path,
        })
        .collect())
}

fn field(node: &HirNode, facts: &HirFnFacts, ty: &TypeToken) -> Result<Vec<Origin>, String> {
    let fields: field_locations::FieldObservations =
        serde_json::from_value(facts.build.frontend_options[field_locations::KEY].clone())
            .map_err(|error| error.to_string())?;
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

pub fn aliases(facts: &HirFnFacts) -> Result<Vec<[u32; 2]>, String> {
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
