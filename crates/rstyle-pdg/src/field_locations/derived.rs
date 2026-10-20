//! Reconstruct indirect field locations from existing reference-value origins.
//! These are temporary typed projections, not additional capture records.

use super::*;
use crate::{data_origins::Origin, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::{ext::IdentExt, spanned::Spanned};

pub(crate) struct Located {
    pub parameter: u32,
    pub path: Vec<Projection>,
    pub value: TypeTree,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    if crate::data_dereferences::candidate(node, facts) {
        return true;
    }
    if node.kind != "expr_field"
        || !crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|profile| profile.supports("rust.indirect-field-reads"))
    {
        return false;
    }

    let [base] = node.children.as_slice() else {
        return false;
    };
    if matches!(base.kind.as_str(), "expr_field" | "expr_unary") && candidate(base, facts) {
        return true;
    }
    if !crate::data_aggregates::token(base)
        .ok()
        .and_then(|token| crate::data_types::parse(token).ok())
        .is_some_and(|ty| matches!(ty, TypeToken::Shared(_)))
    {
        return false;
    }
    match base.kind.as_str() {
        "expr_path" => local(base, facts),
        _ => true,
    }
}

fn local(base: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    let target = base
        .res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref());
    let Some(binding) = facts
        .bindings
        .iter()
        .find(|binding| Some(&binding.hir) == target)
    else {
        return false;
    };

    if binding.is_mut {
        return true;
    }
    if binding.is_param {
        return false;
    }
    if crate::data_reference_locals::flow_enabled(facts) {
        return true;
    }
    let Ok(interface) = facts.build.frontend_options.data::<InterfaceObservations>() else {
        return false;
    };
    aliases::base(base, facts, &interface).is_err()
}

pub(crate) fn locations(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Vec<Located>, String> {
    locations_with(node, facts, &mut |base| {
        crate::data_origins::origins(base, facts)
    })
}

pub(crate) fn locations_with(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    origins: &mut impl FnMut(&HirNode) -> Result<Vec<Origin>, String>,
) -> Result<Vec<Located>, String> {
    let [base] = node.children.as_slice() else {
        return Err("Indirect field lacks one evaluated base".into());
    };
    if crate::data_dereferences::candidate(node, facts) {
        crate::data_dereferences::verify(node, facts)?;
        return referents(base, facts, origins(base)?);
    }
    let bases = if crate::data_places::is_place_base(base)? {
        if candidate(base, facts) {
            locations_with(base, facts, origins)?
        } else {
            let interface = facts
                .build
                .frontend_options
                .data::<InterfaceObservations>()?;
            captured(resolve(base, facts, &interface)?)?
        }
    } else {
        referents(base, facts, origins(base)?)?
    };

    let resolution = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "resolved_field")
        .ok_or("Indirect field lacks resolved member identity")?;
    let resolution: FieldResolution =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let mut result = Vec::new();
    for base in bases {
        let (projection, value) = project(&base.value, &resolution)?;
        verify_member(node, &projection, &value, facts)?;
        result.push(Located {
            parameter: base.parameter,
            path: base.path.into_iter().chain([projection]).collect(),
            value,
        });
    }
    if result.is_empty() {
        return Err("Indirect field has no verified formal referent".into());
    }
    Ok(result)
}

fn captured(location: Location) -> Result<Vec<Located>, String> {
    match location {
        Location::Captured {
            parameter,
            path,
            value,
        } => Ok(vec![Located {
            parameter,
            path,
            value,
        }]),
        Location::Unsupported { reason } => Err(reason),
    }
}

pub(crate) fn referents(
    base: &HirNode,
    facts: &dyn EvidenceFacts,
    origins: Vec<Origin>,
) -> Result<Vec<Located>, String> {
    let TypeToken::Shared(expected) =
        crate::data_types::parse(crate::data_aggregates::token(base)?)?
    else {
        return Err("Indirect field requires a shared reference value".into());
    };
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    origins
        .into_iter()
        .map(|origin| {
            let Some(Observation::Captured { ty }) =
                interface.parameters.get(origin.parameter as usize)
            else {
                return Err("Indirect field has no concrete formal root".into());
            };
            let value = source::origin_value(ty, &origin.path)?;
            if TypeToken::from_tree(&value) != *expected {
                return Err("Indirect field referent contradicts its source value type".into());
            }
            Ok(Located {
                parameter: origin.parameter,
                path: origin.path,
                value: value.into_owned(),
            })
        })
        .collect()
}

fn verify_member(
    node: &HirNode,
    projection: &Projection,
    value: &TypeTree,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing indirect field source")?;
    let expression: syn::Expr = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let syn::Expr::Field(field) = crate::data_aggregates::ungroup(&expression) else {
        return Err("Expected original indirect field expression".into());
    };
    let matches = match (projection, &field.member) {
        (Projection::StructField { name, .. }, syn::Member::Named(member)) => {
            member.unraw() == name.as_str()
        }
        (Projection::StructField { name, .. }, syn::Member::Unnamed(member)) => {
            member.index.to_string() == *name
        }
        (Projection::TupleField { position }, syn::Member::Unnamed(member)) => {
            *position == member.index as usize
        }
        _ => false,
    };

    let span = crate::data_aggregates::ungroup(&field.base).span();
    let syntax = ByteSpan {
        start: node.span.start + crate::source_evidence::byte_offset(bytes, span.start()) as u32,
        end: node.span.start + crate::source_evidence::byte_offset(bytes, span.end()) as u32,
    };
    if !matches
        || crate::data_aggregates::token(node)? != crate::data_types::token(value)?
        || !crate::source_evidence::corresponding_span(
            &facts.source.bytes,
            syntax,
            node.children[0].span,
        )
    {
        return Err("Indirect field loses its original member, type or evaluated base".into());
    }
    Ok(())
}

pub(crate) fn selected(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    selected_with(node, selectors, facts, &mut |base| {
        crate::data_origins::origins(base, facts)
    })
}

pub(crate) fn selected_with(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
    origins: &mut impl FnMut(&HirNode) -> Result<Vec<Origin>, String>,
) -> Result<Vec<Origin>, String> {
    let mut result = std::collections::BTreeSet::new();
    for location in locations_with(node, facts, origins)? {
        result.extend(crate::data_component_origins::selected(
            &TypeToken::from_tree(&location.value),
            selectors,
            location.parameter,
            location.path,
        )?);
    }
    Ok(result.into_iter().collect())
}
