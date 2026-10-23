//! Scalar instance call observations supplement checked dependency links; they never create one.
use crate::{HirNode, scalar_instances, unit_store::EvidenceFacts};
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

const KEY: &str = "rstyle.rust.call-instances";
const COMPILED: &str = "rstyle.rust.compiled-crate";

pub(super) fn signature(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    if !scalar_instances::activation::verify(callee)?
        || !crate::extraction_profile::facts_support(caller, scalar_instances::CAPABILITY)
    {
        return Err("Scalar helper call lacks its private checked instance authority".into());
    }
    let argument = callee
        .scalar_instance()
        .ok_or("Missing checked scalar instance")?
        .scalar_argument();
    let observation = call_observation(node, caller)?;
    let observed: Observation =
        serde_json::from_value(observation.clone()).map_err(|e| e.to_string())?;

    validate(node, &observed, callee.facts(), argument)
}

pub(crate) fn call_observation<'a>(
    node: &HirNode,
    caller: &'a dyn EvidenceFacts,
) -> Result<&'a Value, String> {
    let inventory = caller
        .build
        .frontend_options
        .get(KEY)
        .ok_or("Missing scalar call inventory")?;
    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Incomplete scalar call inventory".into());
    }
    let calls = inventory["calls"]
        .as_array()
        .ok_or("Missing scalar call roster")?;

    let mut matching = calls.iter().filter(|call| {
        serde_json::from_value::<crate::HirRef>(call["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
    });
    let matched = matching
        .next()
        .ok_or("Missing original scalar call observation")?;
    if matching.next().is_some() {
        return Err(
            "Scalar call is missing, duplicated or differs from its original inventory".into(),
        );
    }
    Ok(matched)
}

pub(crate) fn call_argument(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
) -> Result<scalar_instances::ScalarArgument, String> {
    let observation: Observation = serde_json::from_value(call_observation(node, caller)?.clone())
        .map_err(|e| e.to_string())?;
    site(node, &observation)?;
    let requested = scalar_argument(&observation.requested_arguments)?;

    if scalar_argument(&observation.selected_arguments)? != requested {
        return Err("Scalar call requested and selected substitutions disagree".into());
    }
    Ok(requested)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    hir: crate::HirRef,
    span: crate::ByteSpan,
    expanded: bool,
    requested: Definition,
    requested_arguments: Vec<Argument>,
    compiled_dependency: Value,
    status: String,
    selected: Definition,
    selected_arguments: Vec<Argument>,
    signature: Signature,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Definition {
    path: String,
    definition_identity: String,
    #[serde(rename = "crate")]
    crate_name: String,
    stable_crate_id: String,
    local: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Argument {
    kind: String,
    #[serde(rename = "value")]
    _display: String,
    display_only: bool,
    identity_status: String,
    identity: Builtin,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Builtin {
    Builtin { name: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Signature {
    display_only: bool,
    inputs: Vec<String>,
    output: String,
}

fn validate(
    node: &HirNode,
    observation: &Observation,
    callee: &crate::HirFnFacts,
    argument: scalar_instances::ScalarArgument,
) -> Result<(), String> {
    scalar_instances::activation::interface(callee, argument)?;
    site(node, observation)?;
    definition(
        &observation.selected,
        &observation.compiled_dependency,
        callee,
    )?;

    let scalar = scalar_instances::activation::scalar(argument);
    arguments(&observation.requested_arguments, scalar)?;
    arguments(&observation.selected_arguments, scalar)?;
    concrete_signature(node, observation, callee, scalar)
}

fn site(node: &HirNode, observation: &Observation) -> Result<(), String> {
    if node.kind != "expr_call"
        || observation.hir != node.hir
        || observation.span != node.span
        || observation.expanded
        || node.expansion.is_some()
        || observation.status != "resolved"
        || observation.requested != observation.selected
    {
        return Err("Scalar call site or selected original definition disagrees".into());
    }
    Ok(())
}

fn definition(
    definition: &Definition,
    compiled: &Value,
    callee: &crate::HirFnFacts,
) -> Result<(), String> {
    let options = &callee.build.frontend_options;
    let owner = options
        .get(COMPILED)
        .ok_or("Missing original scalar compiler owner")?;
    let path = definition
        .path
        .strip_prefix(&definition.crate_name)
        .and_then(|tail| tail.strip_prefix("::"))
        .unwrap_or(&definition.path);

    if definition.local
        || path != callee.name
        || options.get("definition_identity").and_then(Value::as_str)
            != Some(&definition.definition_identity)
        || owner["identity"] != "rstyle.rust.compiled-crate/1"
        || owner["crate_name"].as_str() != Some(&definition.crate_name)
        || owner["stable_crate_id"].as_str() != Some(&definition.stable_crate_id)
        || ["identity", "crate_name", "stable_crate_id"]
            .iter()
            .any(|key| compiled[*key] != owner[*key])
        || compiled["rmeta"].as_str().is_none_or(str::is_empty)
        || compiled["crate_hash"].as_str().is_none_or(str::is_empty)
        || ["type_arguments", "const_arguments"].iter().any(|key| {
            compiled["instance"][*key]
                .as_array()
                .is_none_or(|args| !args.is_empty())
        })
    {
        return Err("Scalar call differs from original compiled definition owner".into());
    }
    Ok(())
}

fn arguments(arguments: &[Argument], scalar: &str) -> Result<(), String> {
    if scalar_instances::activation::scalar(scalar_argument(arguments)?) != scalar {
        return Err("Scalar call structural substitution disagrees with checked instance".into());
    }
    Ok(())
}

fn scalar_argument(arguments: &[Argument]) -> Result<scalar_instances::ScalarArgument, String> {
    let [argument] = arguments else {
        return Err("Scalar call requires exactly one structural type argument".into());
    };
    let Builtin::Builtin { name } = &argument.identity;
    if argument.kind != "type" || !argument.display_only || argument.identity_status != "known" {
        return Err("Scalar call structural substitution disagrees with checked instance".into());
    }
    match name.as_str() {
        "i32" => Ok(scalar_instances::ScalarArgument::I32),
        "u32" => Ok(scalar_instances::ScalarArgument::U32),
        _ => Err("Scalar call structural argument is outside i32/u32".into()),
    }
}

fn concrete_signature(
    node: &HirNode,
    observation: &Observation,
    callee: &crate::HirFnFacts,
    scalar: &str,
) -> Result<(), String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing scalar call dispatch")?;
    let arity = callee.signature.params.len();
    if node.kind != "expr_call"
        || dispatch.kind != "resolved_scalar_instance"
        || dispatch.op != observation.selected.path
        || dispatch.operand_tys.len() != arity
        || dispatch.operand_tys.iter().any(|ty| ty != scalar)
        || dispatch.result_ty != scalar
        || node.children.len() != arity
        || node
            .children
            .iter()
            .any(|child| child.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(scalar))
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(scalar)
        || !observation.signature.display_only
        || observation.signature.inputs != dispatch.operand_tys
        || observation.signature.output != scalar
    {
        return Err(
            "Scalar call concrete operands or result disagree with checked instance".into(),
        );
    }
    Ok(())
}
