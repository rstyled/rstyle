//! Formal field observations for excluded data interfaces. No admission policy.

mod aliases;
pub(crate) mod derived;
mod source;

use crate::{ByteSpan, HirFnFacts, HirNode, HirRef, concrete_types::*};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "formal_field_observations";
pub const IDENTITY: &str = "rstyle.rust.formal-field-observation/1";
pub const MAX_OPERATIONS: usize = 4096;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldResolution {
    pub position: usize,
    pub owner_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Projection {
    SharedReferent,
    /// Complete UTF-8 storage as bytes, preserving length, lifetime and identity.
    /// Introduced only by an independently checked string-to-byte view operation.
    Utf8Bytes,
    /// Element address retained by a checked Option value; usable only under Some.
    CheckedElement,
    IndexedElement {
        operand_position: u64,
    },
    /// A read selector interpreted in this CALL's verified callee graph.
    HelperIndex {
        call_span: [u32; 2],
        operation: String,
        access: usize,
        path: Vec<Projection>,
    },
    ByteWindow {
        operand_position: u64,
        length: u64,
    },
    IteratorPrefix {
        operand_position: u64,
    },
    StructField {
        nominal: Nominal,
        position: usize,
        name: String,
    },
    TupleField {
        position: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", deny_unknown_fields)]
pub enum Location {
    Captured {
        parameter: u32,
        path: Vec<Projection>,
        value: TypeTree,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldOperation {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub snippet: String,
    pub location: Location,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldObservations {
    pub identity: String,
    pub operations: Vec<FieldOperation>,
    /// Every captured shared formal may alias every other captured shared formal.
    pub possibly_aliasing_formals: Vec<[u32; 2]>,
}

pub fn capture(facts: &mut HirFnFacts) -> Result<(), String> {
    if !facts
        .build
        .frontend_options
        .contains_key(super::concrete_types::KEY)
    {
        return Ok(());
    }
    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
    interface.validate()?;
    if interface.parameters.len() != facts.signature.params.len()
        || interface.parameters.len() > MAX_ENTRIES
        || facts
            .signature
            .params
            .iter()
            .enumerate()
            .any(|(index, parameter)| parameter.position as usize != index)
    {
        return Err("Concrete interface/formal inventory or position evidence disagrees".into());
    }
    let nodes = fields(&facts.body);
    if nodes.len() > MAX_OPERATIONS {
        return Err("Formal field observation exceeds operation budget".into());
    }
    let observations = FieldObservations {
        identity: IDENTITY.into(),
        operations: nodes
            .into_iter()
            .map(|node| operation(node, facts, &interface))
            .collect(),
        possibly_aliasing_formals: aliases(&interface),
    };
    let encoded = serde_json::to_vec(&observations).map_err(|error| error.to_string())?;
    if encoded.len() > MAX_BYTES {
        return Err("Formal field observation exceeds evidence byte budget".into());
    }
    facts.build.frontend_options.insert_data(observations);

    Ok(())
}

fn operation(
    node: &HirNode,
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
) -> FieldOperation {
    let location = if crate::data_value_fields::candidate(node, facts) {
        Location::Unsupported {
            reason: "Aggregate value projection has no formal memory location".into(),
        }
    } else if derived::candidate(node, facts) {
        Location::Unsupported {
            reason: "Indirect locations are reconstructed from reference value flow".into(),
        }
    } else {
        match resolve(node, facts, interface) {
            Ok(location) => location,
            Err(reason) => Location::Unsupported { reason },
        }
    };

    FieldOperation {
        hir: node.hir.clone(),
        span: node.span,
        snippet: node.snippet.clone(),
        location,
    }
}

fn resolve(
    node: &HirNode,
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
) -> Result<Location, String> {
    let (parameter, mut path, mut value) = base(node, facts, interface)?;
    while let TypeTree::Shared(referent) = value {
        path.push(Projection::SharedReferent);
        value = *referent;
    }
    let resolution = node
        .res
        .as_ref()
        .filter(|res| res.kind == "resolved_field")
        .ok_or("Missing compiler-resolved field identity")?;
    let resolution: FieldResolution =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    let (projection, result) = project(&value, &resolution)?;
    if let TypeTree::Scalar(token) = &result
        && node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) != Some(token)
    {
        return Err("Resolved field output contradicts concrete field type".into());
    }
    path.push(projection);

    Ok(Location::Captured {
        parameter,
        path,
        value: result,
    })
}

fn base(
    node: &HirNode,
    facts: &HirFnFacts,
    interface: &InterfaceObservations,
) -> Result<(u32, Vec<Projection>, TypeTree), String> {
    let [base] = node.children.as_slice() else {
        return Err("Field operation lacks its exact base expression".into());
    };
    if base.kind == "expr_field" {
        return match resolve(base, facts, interface)? {
            Location::Captured {
                parameter,
                path,
                value,
            } => Ok((parameter, path, value)),
            Location::Unsupported { reason } => Err(reason),
        };
    }
    let target = base
        .res
        .as_ref()
        .filter(|res| base.kind == "expr_path" && res.kind == "local")
        .and_then(|res| res.target.as_ref())
        .ok_or("Field root is not a resolved formal binding")?;
    let candidates: Vec<_> = facts
        .signature
        .params
        .iter()
        .enumerate()
        .filter(|(_, parameter)| parameter.pat.hir == *target)
        .collect();
    let [(index, parameter)] = candidates.as_slice() else {
        return aliases::base(base, facts, interface);
    };
    let Some(Observation::Captured { ty }) = interface.parameters.get(*index) else {
        return Err("Field root lacks supported concrete type evidence".into());
    };

    Ok((parameter.position, Vec::new(), ty.clone()))
}

fn project(
    value: &TypeTree,
    resolution: &FieldResolution,
) -> Result<(Projection, TypeTree), String> {
    match value {
        TypeTree::Struct {
            nominal, fields, ..
        } if resolution.owner_path.as_ref() == Some(&nominal.definition_path) => {
            let field = fields
                .get(resolution.position)
                .ok_or("Resolved struct field position is out of range")?;
            Ok((
                Projection::StructField {
                    nominal: nominal.clone(),
                    position: field.position,
                    name: field.name.clone(),
                },
                field.ty.clone(),
            ))
        }
        TypeTree::Tuple(elements) if resolution.owner_path.is_none() => {
            let value = elements
                .get(resolution.position)
                .ok_or("Resolved tuple field position is out of range")?;
            Ok((
                Projection::TupleField {
                    position: resolution.position,
                },
                value.clone(),
            ))
        }
        _ => Err(
            "Field owner contradicts concrete root; overloaded dereference is unsupported".into(),
        ),
    }
}

pub fn verify(
    facts: &dyn crate::unit_store::EvidenceFacts,
    observations: &FieldObservations,
) -> Result<(), String> {
    if observations.identity != IDENTITY || observations.operations.len() > MAX_OPERATIONS {
        return Err("Unregistered or over-budget field observations".into());
    }
    if serde_json::to_vec(observations)
        .map_err(|error| error.to_string())?
        .len()
        > MAX_BYTES
    {
        return Err("Field observations exceed evidence byte budget".into());
    }
    if !facts
        .build
        .frontend_options
        .contains_key(super::concrete_types::KEY)
    {
        return Err("Missing concrete interface evidence".into());
    }
    let interface = facts
        .build
        .frontend_options
        .data::<crate::concrete_types::InterfaceObservations>()?;
    interface.validate()?;
    if interface.parameters.len() != facts.signature.params.len()
        || interface.parameters.len() > MAX_ENTRIES
        || facts
            .signature
            .params
            .iter()
            .enumerate()
            .any(|(index, parameter)| parameter.position as usize != index)
    {
        return Err("Concrete interface/formal inventory or position evidence disagrees".into());
    }
    if observations.possibly_aliasing_formals != aliases(&interface) {
        return Err("Shared formal alias uncertainty is incomplete or inconsistent".into());
    }
    if facts.source.bytes.identity() != facts.source.digest {
        return Err("Field observations lack exact captured source identity".into());
    }

    source::verify(facts, observations, &interface)
}

pub(crate) fn fields(node: &HirNode) -> Vec<&HirNode> {
    operations(node, &["expr_field"])
}

pub(crate) fn operations<'a>(node: &'a HirNode, kinds: &[&str]) -> Vec<&'a HirNode> {
    let own = kinds.contains(&node.kind.as_str()).then_some(node);

    node.children
        .iter()
        .flat_map(|child| operations(child, kinds))
        .chain(own)
        .collect()
}

fn aliases(interface: &InterfaceObservations) -> Vec<[u32; 2]> {
    let shared: Vec<_> = interface
        .parameters
        .iter()
        .enumerate()
        .filter_map(|(index, observation)| {
            matches!(
                observation,
                Observation::Captured {
                    ty: TypeTree::Shared(_)
                }
            )
            .then_some(index as u32)
        })
        .collect();

    shared
        .iter()
        .enumerate()
        .flat_map(|(index, left)| shared[index..].iter().map(move |right| [*left, *right]))
        .collect()
}

pub(crate) fn selected_operations<'a>(
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    kinds: &[&str],
    predicate: impl Fn(&HirNode, &dyn crate::unit_store::EvidenceFacts) -> bool,
) -> Vec<&'a HirNode> {
    operations(&facts.body, kinds)
        .into_iter()
        .filter(|node| predicate(node, facts))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_roots_retain_possible_aliases_including_self_aliases() {
        let shared = Observation::Captured {
            ty: TypeTree::Shared(Box::new(TypeTree::Scalar("i32".into()))),
        };
        let interface = InterfaceObservations {
            identity: super::super::concrete_types::IDENTITY.into(),
            parameters: vec![
                shared.clone(),
                Observation::Captured {
                    ty: TypeTree::Scalar("bool".into()),
                },
                shared,
            ],
            result: Observation::Captured {
                ty: TypeTree::Scalar("unit".into()),
            },
        };
        assert_eq!(aliases(&interface), [[0, 0], [0, 2], [2, 2]]);
    }

    #[test]
    fn observation_wire_format_rejects_opaque_or_extra_location_fields() {
        assert!(serde_json::from_value::<FieldObservations>(serde_json::json!({
            "identity": IDENTITY, "operations": [], "possibly_aliasing_formals": [], "pure": true,
        })).is_err());
        assert!(serde_json::from_value::<Location>(serde_json::json!({
            "status": "Captured", "parameter": 0, "path": [{"kind": "memory"}], "value": {"kind": "Scalar", "evidence": "i32"},
        })).is_err());
    }
}
