//! Typed shared capture evidence. Structural checks never resolve nominal paths.
use super::{CaptureMode, ENVIRONMENT_KIND, Environment};
use crate::{
    HirNode, HirRef,
    concrete_types::{Observation, TypeTree},
    unit_store::EvidenceFacts,
};
use serde::{Deserialize, Serialize};

pub const CAPABILITY: &str = "rust.shared-helper-option-callbacks";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedCapture {
    pub closure: HirRef,
    pub root_type: Observation,
    pub projections: Vec<SharedProjection>,
    pub uses: Vec<SharedUse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum SharedProjection {
    SharedDeref {
        before: Observation,
        after: Observation,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedUse {
    pub hir: HirRef,
    pub ty: Observation,
}

pub(crate) struct SharedFormal {
    pub root: HirRef,
    pub token: String,
    pub parameter: u32,
}

/// Does not call nominal resolution, data frontend verification or helper proof.
/// The full owner source/effect boundary remains the caller's obligation.
pub(crate) fn verify_structure(
    closure: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<SharedFormal, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY)
        || closure.kind != "expr_closure"
        || closure.expansion.is_some()
        || closure.is_unsafe_block
    {
        return Err("Shared callback capture requires its original registered closure".into());
    }

    let environment = environment(closure)?;
    let [capture] = environment.captures.as_slice() else {
        return Err("Shared callback requires exactly one compiler capture".into());
    };
    let exact = environment
        .shared_formal
        .as_ref()
        .ok_or("Missing exact shared capture evidence")?;
    if capture.mode != CaptureMode::Shared
        || capture.projections != 1
        || exact.closure != closure.hir
        || exact.projections.len() != 1
    {
        return Err("Shared callback loses exact immutable capture ownership/projection".into());
    }

    let formal = formal(closure, &capture.root, facts)?;
    let root = root_type(&exact.root_type, &formal, facts)?;
    projection(exact, root, &environment.stored_type)?;
    uses(closure, exact, &formal)?;
    Ok(formal)
}

fn environment(closure: &HirNode) -> Result<Environment, String> {
    let resolution = closure
        .res
        .as_ref()
        .ok_or("Missing compiler capture environment")?;
    if resolution.kind != ENVIRONMENT_KIND || resolution.target.is_some() {
        return Err("Shared callback requires compiler environment observations".into());
    }
    serde_json::from_str(&resolution.description).map_err(|error| error.to_string())
}

fn formal(
    closure: &HirNode,
    root: &HirRef,
    facts: &dyn EvidenceFacts,
) -> Result<SharedFormal, String> {
    let parameter = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *root)
        .ok_or("Shared callback capture must root at a function formal")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| binding.hir == *root)
        .ok_or("Shared callback formal lacks binding evidence")?;

    if root.krate != 0 || root.owner != closure.hir.owner {
        return Err("Shared callback root must belong to the enclosing function".into());
    }
    let token = formal_binding(parameter, binding)?;
    Ok(SharedFormal {
        root: root.clone(),
        token,
        parameter: parameter.position,
    })
}

fn formal_binding(
    parameter: &crate::ParamFact,
    binding: &crate::BindingFact,
) -> Result<String, String> {
    let token = parameter
        .ty
        .normalized
        .as_ref()
        .ok_or("Missing shared callback formal type")?;
    if binding.is_mut
        || !binding.is_param
        || parameter.pat.kind != "pat_binding"
        || parameter.pat.binding_mut != Some(false)
        || parameter.pat.binding.as_ref() != Some(&parameter.symbol)
        || parameter.symbol != binding.symbol
        || parameter.pat.param_index != Some(parameter.position)
        || parameter.name_span != binding.decl_span
        || parameter.pat.span != binding.decl_span
        || binding.ty.normalized.as_ref() != Some(token)
        || parameter
            .pat
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_ref())
            != Some(token)
    {
        return Err(
            "Shared callback root disagrees with immutable formal/binding/type evidence".into(),
        );
    }
    Ok(token.clone())
}

fn root_type<'a>(
    observation: &'a Observation,
    formal: &SharedFormal,
    facts: &dyn EvidenceFacts,
) -> Result<&'a TypeTree, String> {
    let Observation::Captured { ty: root } = observation else {
        return Err("Shared capture root type is unknown".into());
    };
    let TypeTree::Shared(referent) = root else {
        return Err("Shared callback formal must have a builtin shared reference type".into());
    };
    if !matches!(referent.as_ref(), TypeTree::Struct { .. }) {
        return Err("Shared callback referent must be a complete concrete struct".into());
    }

    let interface: crate::concrete_types::InterfaceObservations =
        crate::data_frontend::evidence(facts, crate::concrete_types::KEY)?;
    interface.validate()?;
    if interface.parameters.get(formal.parameter as usize) != Some(observation)
        || crate::data_types::token(root)? != formal.token
    {
        return Err("Shared capture root differs from concrete formal observation".into());
    }
    Ok(root)
}

fn projection(exact: &SharedCapture, root: &TypeTree, stored: &Observation) -> Result<(), String> {
    let TypeTree::Shared(referent) = root else {
        return Err("Shared capture lacks its builtin reference root".into());
    };
    let [SharedProjection::SharedDeref { before, after }] = exact.projections.as_slice() else {
        return Err("Shared capture requires exactly one typed builtin shared dereference".into());
    };

    let captured = |ty| Observation::Captured { ty };
    let expected = TypeTree::Tuple(vec![TypeTree::Shared(referent.clone())]);
    if *before != captured(root.clone())
        || *after != captured(*referent.clone())
        || *stored != captured(expected)
    {
        return Err("Shared capture projection/storage disagrees with exact builtin types".into());
    }
    Ok(())
}

fn uses(closure: &HirNode, exact: &SharedCapture, formal: &SharedFormal) -> Result<(), String> {
    let mut paths = Vec::new();
    crate::frontend::collect_kind(closure, "expr_path", &mut paths);
    let paths: Vec<_> = paths
        .into_iter()
        .filter(|node| {
            node.res.as_ref().is_some_and(|resolution| {
                resolution.kind == "local" && resolution.target.as_ref() == Some(&formal.root)
            })
        })
        .collect();
    if paths.is_empty() || paths.len() != exact.uses.len() {
        return Err("Shared capture loses its complete resolved source-use inventory".into());
    }

    let mut seen = std::collections::BTreeSet::new();
    for (node, observed) in paths.into_iter().zip(&exact.uses) {
        if node.hir != observed.hir
            || !seen.insert((node.hir.owner, node.hir.local))
            || observed.ty != exact.root_type
            || node.hir.krate != 0
            || node.span.start < closure.span.start
            || node.span.end > closure.span.end
            || node.expansion.is_some()
            || !node.children.is_empty()
            || node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) != Some(&formal.token)
        {
            return Err("Shared capture use disagrees with its exact root/type/site".into());
        }
    }
    Ok(())
}

pub(crate) fn verify_shared_formal(
    closure: &HirNode,
    receiver: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let formal = verify_structure(closure, facts)?;
    let environment = environment(closure)?;
    let exact = environment
        .shared_formal
        .as_ref()
        .ok_or("Missing exact shared capture")?;
    if !exact.uses.iter().any(|usage| usage.hir == receiver.hir)
        || receiver.kind != "expr_path"
        || !receiver.res.as_ref().is_some_and(|resolution| {
            resolution.kind == "local" && resolution.target.as_ref() == Some(&formal.root)
        })
    {
        return Err("Callback helper receiver differs from its actual captured formal use".into());
    }

    let expected = vec![crate::data_origins::Origin {
        parameter: formal.parameter,
        path: vec![crate::field_locations::Projection::SharedReferent],
    }];
    if crate::data_origins::origins(receiver, facts)? != expected {
        return Err("Callback shared receiver loses its exact formal referent origin".into());
    }
    Ok(())
}
