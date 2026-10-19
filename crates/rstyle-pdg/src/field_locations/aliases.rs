//! Capture a shared local's referent without allocating a local storage location.

use super::*;
use crate::data_types::TypeToken;

type Referent = (u32, Vec<Projection>, TypeTree);

pub(super) fn base(
    node: &HirNode,
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
) -> Result<Referent, String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.shared-local-field-reads")
    {
        return Err("Shared local field reads require their registered capability".into());
    }

    let ty = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
    let initializer = crate::data_reference_locals::initializer(node, facts, &ty)?;
    let referent = reference(initializer, facts, interface)?;
    check_type(node, &referent.2)?;
    Ok(referent)
}

fn reference(
    node: &HirNode,
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
) -> Result<Referent, String> {
    let result = match node.kind.as_str() {
        "expr_path" => path(node, facts, interface)?,
        "expr_field" => loaded(super::resolve(node, facts, interface)?)?,
        "expr_addr_of" => {
            let [place] = node.children.as_slice() else {
                return Err("Shared field borrow lacks one place".into());
            };
            if place.kind != "expr_field" {
                return Err("Shared alias borrow requires a formal field place".into());
            }
            captured(super::resolve(place, facts, interface)?)?
        }
        "expr_block" => reference(
            node.children.last().ok_or("Missing shared block value")?,
            facts,
            interface,
        )?,
        "expr_if" => {
            let [_, left, right] = node.children.as_slice() else {
                return Err("Shared field alias requires both alternatives".into());
            };
            let left = reference(left, facts, interface)?;
            if left != reference(right, facts, interface)? {
                return Err("Shared field alias has alternative memory locations".into());
            }
            left
        }
        _ => return Err("Unsupported shared field alias initializer".into()),
    };
    check_type(node, &result.2)?;
    Ok(result)
}

fn path(
    node: &HirNode,
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
) -> Result<Referent, String> {
    let target = node
        .res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Unresolved shared alias path")?;
    let Some(parameter) = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *target)
    else {
        return base(node, facts, interface);
    };
    let Some(Observation::Captured { ty }) = interface.parameters.get(parameter.position as usize)
    else {
        return Err("Shared alias formal lacks its concrete type".into());
    };
    loaded(Location::Captured {
        parameter: parameter.position,
        path: Vec::new(),
        value: ty.clone(),
    })
}

fn captured(location: Location) -> Result<Referent, String> {
    match location {
        Location::Captured {
            parameter,
            path,
            value,
        } => Ok((parameter, path, value)),
        Location::Unsupported { reason } => Err(reason),
    }
}

fn loaded(location: Location) -> Result<Referent, String> {
    let (parameter, mut path, value) = captured(location)?;
    let TypeTree::Shared(referent) = value else {
        return Err("Shared alias initializer is not a reference value".into());
    };
    path.push(Projection::SharedReferent);
    Ok((parameter, path, *referent))
}

fn check_type(node: &HirNode, referent: &TypeTree) -> Result<(), String> {
    let expected = TypeToken::Shared(Box::new(TypeToken::from_tree(referent)));
    if crate::data_types::parse(crate::data_aggregates::token(node)?)? != expected {
        return Err("Shared alias value disagrees with its formal referent type".into());
    }
    Ok(())
}
