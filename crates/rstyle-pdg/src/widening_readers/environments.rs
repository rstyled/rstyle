//! Whole immutable environments and separate lexical call inventories.
use super::{authority::Proof, roles::Roles};
use crate::{
    HirNode, HirRef,
    concrete_types::{Observation, TypeTree},
    id_error_maps::capture::Environment,
    option_callbacks::CaptureMode,
    unit_store::EvidenceFacts,
};
use serde_json::Value;

const KIND: &str = "compiler_usize_reader_callback_environment";
const IDENTITY: &str = "rstyle.rust.usize-reader-callback-environment/1";
const SCOPE: &str =
    "immediate-core-result-reader-callback-owning-body; nested owners separately scoped";

struct Expected<'a> {
    closure: &'a HirNode,
    body: &'a HirNode,
    root: &'a HirRef,
    root_type: TypeTree,
    input: Option<TypeTree>,
    output: TypeTree,
}

pub(super) fn verify(
    roles: &Roles<'_>,
    proof: &Proof,
    facts: &dyn EvidenceFacts,
) -> Result<[TypeTree; 3], String> {
    parameters(roles, facts)?;

    let expected = expectations(roles, proof)?;

    let environments = expected
        .iter()
        .map(environment)
        .collect::<Result<Vec<_>, _>>()?;
    inventories(roles, &environments, proof, facts)?;

    Ok(std::array::from_fn(|index| {
        match &environments[index].stored_type {
            Observation::Captured { ty } => ty.clone(),
            Observation::Unsupported { .. } => unreachable!(),
        }
    }))
}

fn expectations<'a>(roles: &Roles<'a>, proof: &Proof) -> Result<[Expected<'a>; 3], String> {
    let TypeTree::Result { err, .. } = &proof.output else {
        return Err("Widening callback requires its checked final Result".into());
    };

    Ok([
        Expected {
            closure: roles.continuation,
            body: roles
                .continuation
                .children
                .last()
                .ok_or("Missing continuation body")?,
            root: &roles.message.pat.hir,
            root_type: message(),
            input: Some(super::authority::tuple("u32")),
            output: proof.output.clone(),
        },
        Expected {
            closure: roles.success,
            body: roles.tuple,
            root: &roles.count.hir,
            root_type: TypeTree::Scalar("usize".into()),
            input: Some(TypeTree::Scalar("usize".into())),
            output: super::authority::tuple("usize"),
        },
        Expected {
            closure: roles.failure,
            body: roles.constructor,
            root: &roles.message.pat.hir,
            root_type: message(),
            input: None,
            output: (**err).clone(),
        },
    ])
}

fn environment(expected: &Expected<'_>) -> Result<Environment, String> {
    let closure = expected.closure;

    let resolution = closure
        .res
        .as_ref()
        .ok_or("Missing widening callback environment")?;

    if resolution.kind != KIND || resolution.target.is_some() {
        return Err("Widening callback lacks its compiler environment observation".into());
    }

    let environment: Environment =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    if environment.identity != IDENTITY
        || environment.closure != closure.hir
        || environment.body != expected.body.hir
        || environment.definition_identity.is_empty()
        || environment.signature_abi != "RustCall"
        || environment.signature_output != captured(expected.output.clone())
        || closure.kind != "expr_closure"
        || closure.children.len() != 2
        || closure.expansion.is_some()
        || closure.is_unsafe_block
    {
        return Err("Widening callback changes its complete compiler owner/body/interface".into());
    }
    signature_input(&environment, expected.input.as_ref())?;
    capture(&environment, expected)?;

    Ok(environment)
}

fn signature_input(environment: &Environment, input: Option<&TypeTree>) -> Result<(), String> {
    let valid = match input {
        Some(input) => {
            environment.signature_inputs == [captured(TypeTree::Tuple(vec![input.clone()]))]
        }
        // The selected conversion and its pinned core error source independently
        // prove this wildcard's Copy/no-drop input. Preserve unavailable raw data.
        None => matches!(environment.signature_inputs.as_slice(),
            [Observation::Unsupported { reason }]
                if reason == "Struct requires local concrete source, no generics/destructor/interior mutability"),
    };

    if !valid {
        return Err("Widening callback changes its actual tupled RustCall input".into());
    }

    Ok(())
}

fn capture(environment: &Environment, expected: &Expected<'_>) -> Result<(), String> {
    let [capture] = environment.captures.as_slice() else {
        return Err("Widening callback requires its sole original immutable capture".into());
    };

    let storage = TypeTree::Tuple(vec![TypeTree::Shared(Box::new(expected.root_type.clone()))]);

    if &capture.root != expected.root
        || capture.mode != CaptureMode::Shared
        || capture.root_type != captured(expected.root_type.clone())
        || !capture.projections.is_empty()
        || environment.stored_type != captured(storage)
    {
        return Err("Widening callback changes its shared whole-root capture or storage".into());
    }

    Ok(())
}

fn parameters(roles: &Roles<'_>, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let payload = roles.payload;

    if payload.kind != "pat_scalar_tuple"
        || !payload.children.iter().eq([roles.decoded, roles.count])
        || crate::data_aggregates::token(payload)?
            != crate::data_types::token(&super::authority::tuple("u32"))?
        || payload.binding.is_some()
        || payload.binding_mut.is_some()
        || payload.param_index.is_some()
        || payload.expansion.is_some()
        || payload.res.is_some()
        || payload.dispatch.is_some()
        || facts.bindings.len() != 5
    {
        return Err("Widening continuation changes its complete immutable tuple parameter".into());
    }
    parameter(roles.decoded, roles.continuation, "u32", facts)?;
    parameter(roles.count, roles.continuation, "usize", facts)?;
    parameter(roles.converted, roles.success, "usize", facts)?;
    wildcard(roles.wildcard)?;
    crate::source_evidence::verify_binding_read(
        roles.decoded_read,
        roles.decoded,
        "Changed decoded payload",
    )?;
    crate::source_evidence::verify_binding_read(
        roles.value_read,
        roles.converted,
        "Changed converted payload",
    )?;
    crate::source_evidence::verify_binding_read(
        roles.count_read,
        roles.count,
        "Changed saved byte count",
    )?;
    crate::source_evidence::verify_binding_read(
        roles.message_read,
        &roles.message.pat,
        "Changed static message",
    )
}

fn parameter(
    binding: &HirNode,
    closure: &HirNode,
    expected_type: &str,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    crate::source_evidence::plain_immutable_binding(
        binding,
        "Widening callback binding is not immutable",
    )?;

    let scope = format!(
        "scope:source:callback:{}:{}",
        closure.span.start, closure.span.end
    );

    let matching: Vec<_> = facts
        .bindings
        .iter()
        .filter(|record| {
            record.hir == binding.hir || Some(&record.symbol) == binding.binding.as_ref()
        })
        .collect();

    let [record] = matching.as_slice() else {
        return Err("Widening callback parameter binding is absent or ambiguous".into());
    };

    if crate::data_aggregates::token(binding)? != expected_type
        || record.hir != binding.hir
        || record.is_param
        || record.is_mut
        || record.scope != scope
        || record.decl_span != binding.span
        || Some(&record.symbol) != binding.binding.as_ref()
        || binding.param_index.is_some()
        || binding.expansion.is_some()
        || binding.res.is_some()
        || binding.dispatch.is_some()
        || binding.is_unsafe_block
        || binding
            .ty
            .as_ref()
            .is_none_or(|ty| record.ty.normalized != ty.normalized)
    {
        return Err("Widening callback parameter changes its original scope/type/identity".into());
    }

    Ok(())
}

fn wildcard(node: &HirNode) -> Result<(), String> {
    if node.kind != "pat_wild"
        || !node.children.is_empty()
        || node.binding.is_some()
        || node.binding_mut.is_some()
        || node.param_index.is_some()
        || node.expansion.is_some()
        || node.res.is_some()
        || node.dispatch.is_some()
        || node.is_unsafe_block
        || node.ty.as_ref().is_none_or(|ty| {
            ty.normalized.is_some()
                || !matches!(
                    ty.observed.as_str(),
                    "core::num::TryFromIntError" | "std::num::TryFromIntError"
                )
        })
    {
        return Err(
            "Widening error callback must retain its exact unbound conversion-error wildcard"
                .into(),
        );
    }

    Ok(())
}

fn inventories(
    roles: &Roles<'_>,
    environments: &[Environment],
    proof: &Proof,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing widening callback call inventory")?;

    if inventory["identity"] != "rstyle.rust.call-instance-observations/1"
        || inventory["complete"] != true
    {
        return Err("Widening callback inventory is not complete".into());
    }
    calls(inventory, &[roles.source])?;

    let [outer] = crate::source_evidence::call_inventory::callbacks(
        inventory,
        "Malformed widening nested callback roster",
    )?
    else {
        return Err("Widening reader requires exactly its one outer callback owner".into());
    };
    record(outer, roles.continuation, &environments[0], None)?;
    calls(outer, &[roles.map_err, roles.map, roles.conversion])?;
    substitutions(
        roles,
        proof,
        substitution_calls(roles, inventory, outer)?,
        facts,
    )?;

    nested_inventory(outer, roles, environments)
}

fn nested_inventory(
    outer: &Value,
    roles: &Roles<'_>,
    environments: &[Environment],
) -> Result<(), String> {
    let nested = crate::source_evidence::call_inventory::callbacks(
        outer,
        "Malformed widening nested callback roster",
    )?;

    if nested.len() != 2 {
        return Err("Widening continuation requires both separately owned nested callbacks".into());
    }

    for (closure, environment) in [roles.success, roles.failure]
        .into_iter()
        .zip(&environments[1..])
    {
        let child = exact(nested, closure)?;
        record(child, closure, environment, Some(outer))?;
        calls(child, &[])?;
        if !crate::source_evidence::call_inventory::callbacks(
            child,
            "Malformed widening nested callback roster",
        )?
        .is_empty()
        {
            return Err("Widening inner callback invents additional nested ownership".into());
        }
    }
    distinct(environments)
}

fn substitution_calls<'a>(
    roles: &Roles<'_>,
    inventory: &'a Value,
    outer: &'a Value,
) -> Result<[&'a Value; 3], String> {
    let root = inventory["calls"].as_array().ok_or("Missing root calls")?;

    let direct = outer["calls"]
        .as_array()
        .ok_or("Missing continuation calls")?;

    Ok([
        exact(root, roles.source)?,
        exact(direct, roles.map)?,
        exact(direct, roles.map_err)?,
    ])
}

fn record(
    record: &Value,
    closure: &HirNode,
    environment: &Environment,
    parent: Option<&Value>,
) -> Result<(), String> {
    if !crate::source_evidence::call_inventory::Site::read(record).matches(closure)
        || record["scope"] != SCOPE
        || record["complete"] != true
        || record["body"]
            != serde_json::to_value(&environment.body).map_err(|error| error.to_string())?
        || record["capture_count"] != 1
        || record["definition"]["definition_identity"] != environment.definition_identity
        || record["definition"]["local"] != true
        || ["path", "crate", "stable_crate_id"]
            .iter()
            .any(|key| record["definition"][key].as_str().is_none_or(str::is_empty))
    {
        return Err(
            "Widening callback inventory changes its exact observed body/definition".into(),
        );
    }

    match parent {
        Some(parent)
            if record["parent_callback"] == parent["hir"]
                && record["definition"]["crate"] == parent["definition"]["crate"]
                && record["definition"]["stable_crate_id"]
                    == parent["definition"]["stable_crate_id"] => {}
        None if record.get("parent_callback").is_none() => {}
        _ => return Err("Widening callback changes its lexical parent or compiled crate".into()),
    }

    Ok(())
}

fn calls(record: &Value, nodes: &[&HirNode]) -> Result<(), String> {
    let calls = record["calls"]
        .as_array()
        .ok_or("Missing widening scoped call roster")?;

    if calls.len() != nodes.len() {
        return Err("Widening scoped call roster omits or invents eligible calls".into());
    }

    for node in nodes {
        let call = exact(calls, node)?;
        if call["expanded"] != false
            || call
                .get("compiled_dependency")
                .is_some_and(|value| !value.is_null())
        {
            return Err("Widening scoped call changes its original source ownership".into());
        }
        if node.kind == "expr_method_call" {
            method_call(call, node)?;
        }
    }

    Ok(())
}

fn method_call(call: &Value, node: &HirNode) -> Result<(), String> {
    let callee = crate::data_library::callee(node)?;

    if call["status"] != "unsupported"
        || call["reason"] != "instance argument structural identity is incomplete"
        || call["requested"] != call["selected"]
        || call["requested"]["path"] != callee.path
        || call["requested"]["definition_identity"] != callee.definition_hash
        || call["requested"]["crate"] != "core"
        || call["requested"]["local"] != false
        || call["requested_arguments"] != call["selected_arguments"]
    {
        return Err("Widening method inventory changes its selected core method".into());
    }

    Ok(())
}

fn substitutions(
    roles: &Roles<'_>,
    proof: &Proof,
    calls: [&Value; 3],
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let outer = arguments(calls[0], roles.continuation)?;

    let map = arguments(calls[1], roles.success)?;

    let error_map = arguments(calls[2], roles.failure)?;

    let decoded = tuple_identity("u32");

    let converted = tuple_identity("usize");

    if outer[0]["identity"] != decoded
        || outer[2]["identity"] != converted
        || map[0]["identity"] != serde_json::json!({"kind":"builtin", "name":"usize"})
        || map[2]["identity"] != converted
        || error_map[0]["identity"] != converted
        || outer[1]["identity"] != error_map[2]["identity"]
        || map[1]["identity"] != error_map[1]["identity"]
    {
        return Err(
            "Widening method changes its independently observed generic substitutions".into(),
        );
    }

    let inventory = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing widening call inventory")?;

    let [owner] = crate::source_evidence::call_inventory::callbacks(
        inventory,
        "Malformed widening nested callback roster",
    )?
    else {
        return Err("Missing sole widening callback owner".into());
    };
    local_error(&outer[1], &proof.output, &owner["definition"])?;
    core_error(&map[1], facts)
}

fn arguments<'a>(call: &'a Value, closure: &HirNode) -> Result<&'a [Value], String> {
    let arguments = call["requested_arguments"]
        .as_array()
        .filter(|arguments| arguments.len() == 4)
        .ok_or("Widening method requires all four actual type substitutions")?;

    let callback = &arguments[3];

    if callback["kind"] != "type"
        || callback["display_only"] != true
        || callback["identity_status"] != "unsupported"
        || callback["reason"] != "capturing closure is outside structural callable observation"
        || callback.get("identity").is_some()
        || callback["value"].as_str() != closure.ty.as_ref().map(|ty| ty.observed.as_str())
        || arguments[..3].iter().any(|argument| {
            argument["kind"] != "type"
                || argument["display_only"] != true
                || argument["identity_status"] != "known"
                || argument["value"].as_str().is_none_or(str::is_empty)
                || !argument["identity"].is_object()
        })
    {
        return Err(
            "Widening substitutions lose known types or their explicitly unsupported closure"
                .into(),
        );
    }

    Ok(arguments)
}

fn local_error(argument: &Value, output: &TypeTree, owner: &Value) -> Result<(), String> {
    let TypeTree::Result { err, .. } = output else {
        unreachable!()
    };

    let TypeTree::Struct { nominal, .. } = err.as_ref() else {
        unreachable!()
    };

    let identity = &argument["identity"];

    let definition = &identity["definition"];

    if identity["kind"] != "adt"
        || identity["arguments"] != serde_json::json!([])
        || definition["local"] != true
        || definition["path"] != nominal.definition_path
        || definition["crate"] != owner["crate"]
        || definition["stable_crate_id"] != owner["stable_crate_id"]
        || definition["definition_identity"]
            .as_str()
            .is_none_or(str::is_empty)
        || argument["value"] != nominal.definition_path
    {
        return Err(
            "Widening substitution changes its independently checked local error owner".into(),
        );
    }

    Ok(())
}

fn core_error(argument: &Value, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned widening core error")?;

    let identity = &argument["identity"];

    let definition = &identity["definition"];

    let stable = registry
        .definitions
        .get("rstyle.lossless-integer.core-stable-crate-id")
        .ok_or("Missing independently pinned core stable identity")?;

    let error = registry
        .definitions
        .get("std::num::TryFromIntError")
        .ok_or("Missing independently pinned conversion error definition")?;

    if identity["kind"] != "adt"
        || identity["arguments"] != serde_json::json!([])
        || definition["local"] != false
        || definition["crate"] != "core"
        || definition["stable_crate_id"].as_str() != Some(stable.as_str())
        || definition["definition_identity"].as_str() != Some(error.as_str())
        || !matches!(
            definition["path"].as_str(),
            Some("std::num::TryFromIntError" | "core::num::TryFromIntError")
        )
    {
        return Err("Widening substitution changes its exact pinned conversion error".into());
    }

    Ok(())
}

fn tuple_identity(first: &str) -> Value {
    serde_json::json!({"kind":"tuple", "elements":[
        builtin_identity(first), builtin_identity("usize")
    ]})
}

fn distinct(environments: &[Environment]) -> Result<(), String> {
    for (index, environment) in environments.iter().enumerate() {
        if environments[..index].iter().any(|prior| {
            prior.closure == environment.closure
                || prior.body == environment.body
                || prior.definition_identity == environment.definition_identity
        }) {
            return Err("Widening callbacks repeat a compiler owner, body or definition".into());
        }
    }

    Ok(())
}

fn exact<'a>(records: &'a [Value], node: &HirNode) -> Result<&'a Value, String> {
    let mut matches = records.iter().filter(|record| {
        serde_json::from_value::<HirRef>(record["hir"].clone())
            .ok()
            .as_ref()
            == Some(&node.hir)
            || serde_json::from_value::<crate::ByteSpan>(record["span"].clone()).ok()
                == Some(node.span)
    });

    let record = matches
        .next()
        .ok_or("Missing widening owned callback/call site")?;

    if matches.next().is_some()
        || !crate::source_evidence::call_inventory::Site::read(record).matches(node)
    {
        return Err("Widening owned callback/call site is ambiguous or mismatched".into());
    }

    Ok(record)
}

fn captured(ty: TypeTree) -> Observation {
    Observation::Captured { ty }
}

fn message() -> TypeTree {
    TypeTree::Shared(Box::new(TypeTree::Str))
}

fn builtin_identity(name: &str) -> Value {
    serde_json::json!({"kind":"builtin", "name":name})
}
