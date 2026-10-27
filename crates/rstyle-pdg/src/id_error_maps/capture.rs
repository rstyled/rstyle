//! Raw closure observations preserve capture precision and concrete storage.
use super::*;
use crate::{HirRef, concrete_types::Observation, option_callbacks::CaptureMode};
use serde::{Deserialize, Serialize};

pub const KIND: &str = "compiler_id_error_map_environment";
pub const IDENTITY: &str = "rstyle.rust.id-error-map-environment/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub identity: String,
    pub closure: HirRef,
    pub body: HirRef,
    pub definition_identity: String,
    pub captures: Vec<Capture>,
    pub stored_type: Observation,
    pub signature_abi: String,
    pub signature_inputs: Vec<Observation>,
    pub signature_output: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capture {
    pub root: HirRef,
    pub mode: CaptureMode,
    pub root_type: Observation,
    pub projections: Vec<Projection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub kind: String,
    pub before: Observation,
    pub after: Observation,
}

pub(super) fn verify(checked: &mut Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let resolution = checked
        .closure
        .res
        .as_ref()
        .ok_or("Missing ID error callback environment")?;

    if resolution.kind != KIND || resolution.target.is_some() {
        return Err("ID error callback requires compiler closure observations".into());
    }

    let environment: Environment =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    if environment.identity != IDENTITY
        || environment.closure != checked.closure.hir
        || environment.body != checked.constructor.hir
        || environment.definition_identity.is_empty()
        || environment.signature_abi != "RustCall"
        || environment.signature_inputs
            != [captured(TypeTree::Tuple(vec![checked.error().clone()]))]
        || environment.signature_output != captured(checked.transfer_output()?)
    {
        return Err("ID error callback changes compiler body, input or output".into());
    }

    let [capture] = environment.captures.as_slice() else {
        return Err("ID error callback requires exactly one immutable static capture".into());
    };

    let message = static_message();

    if capture.root != checked.message.pat.hir
        || capture.mode != CaptureMode::Shared
        || capture.root_type != captured(message.clone())
        || !capture.projections.is_empty()
    {
        return Err("ID error callback changes its whole static-message shared capture".into());
    }

    let storage = TypeTree::Tuple(vec![TypeTree::Shared(Box::new(message))]);

    if environment.stored_type != captured(storage.clone()) {
        return Err(
            "ID error callback storage differs from its captured whole static reference".into(),
        );
    }
    checked.storage = storage;
    parameter(checked, facts)
}

fn parameter(checked: &Checked<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let binding = checked.binding;

    let scope = format!(
        "scope:source:callback:{}:{}",
        checked.closure.span.start, checked.closure.span.end
    );

    if binding.kind != "pat_binding"
        || binding.binding_mut != Some(false)
        || binding.binding.is_none()
        || binding.param_index.is_some()
        || !binding.children.is_empty()
        || binding.expansion.is_some()
        || !facts.bindings.iter().any(|record| {
            record.hir == binding.hir
                && !record.is_param
                && !record.is_mut
                && record.scope == scope
                && record.decl_span == binding.span
                && Some(&record.symbol) == binding.binding.as_ref()
                && binding
                    .ty
                    .as_ref()
                    .is_some_and(|ty| record.ty.normalized == ty.normalized)
        })
    {
        return Err("ID error callback loses its original immutable owned parameter".into());
    }

    Ok(())
}

fn captured(ty: TypeTree) -> Observation {
    Observation::Captured { ty }
}

pub(super) fn static_message() -> TypeTree {
    TypeTree::Shared(Box::new(TypeTree::Str))
}

impl Checked<'_> {
    fn transfer_output(&self) -> Result<TypeTree, String> {
        let TypeTree::Result { err, .. } = &self.output else {
            return Err("ID error callback has no concrete output Result".into());
        };

        Ok((**err).clone())
    }
}
