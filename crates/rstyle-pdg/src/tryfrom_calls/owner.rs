//! Join concrete owning observation to original interface without nominal replay.
use super::{model::*, *};
use crate::{concrete_types::Nominal, data_types::TypeToken};

pub(super) struct Identity {
    pub success: Definition,
    pub error: Definition,
}

pub(super) fn local(facts: &HirFnFacts, definition: &Definition) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("TryFrom lacks original compiled-crate identity")?;
    if !definition.local
        || definition.path.is_empty()
        || definition.definition_identity.is_empty()
        || definition.stable_crate_id.is_empty()
        || compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || compiled["crate_name"].as_str() != Some(&definition.crate_name)
        || compiled["stable_crate_id"].as_str() != Some(&definition.stable_crate_id)
    {
        return Err("TryFrom selected local owner differs from compiled crate".into());
    }

    Ok(())
}

pub(super) fn nominal(identity: &Structural, nominal: &Nominal) -> Result<Definition, String> {
    let Structural::Adt {
        definition,
        arguments,
    } = identity
    else {
        return Err("TryFrom requires an exact local nominal structural identity".into());
    };
    let path = definition
        .path
        .strip_prefix(&format!("{}::", definition.crate_name))
        .unwrap_or(&definition.path);
    if !arguments.is_empty()
        || !definition.local
        || definition.crate_name.is_empty()
        || definition.definition_identity.is_empty()
        || definition.stable_crate_id.is_empty()
        || path != nominal.definition_path
    {
        return Err("TryFrom structural nominal differs from complete captured owner".into());
    }

    Ok(definition.clone())
}

pub(super) fn endpoint(arguments: &[Argument]) -> Result<&Structural, String> {
    let [argument] = arguments else {
        return Err("TryFrom requires one complete structural endpoint".into());
    };
    argument.known()
}

pub(super) fn usize_type(identity: &Structural) -> Result<(), String> {
    if !matches!(identity, Structural::Builtin { name } if name == "usize") {
        return Err("TryFrom input differs from exact builtin usize".into());
    }

    Ok(())
}

pub(super) fn verify(
    facts: &HirFnFacts,
    observation: &Observation,
    authority: &dyn CoreAuthority,
) -> Result<Identity, String> {
    let (success, error) = super::result(facts)?;
    let [self_argument, input] = observation.trait_arguments.as_slice() else {
        return Err("TryFrom requires complete Self and input substitutions".into());
    };
    let success = nominal(
        self_argument.known()?,
        success
            .nominal_parts()
            .ok_or("TryFrom success lacks a concrete nominal owner")?
            .0,
    )?;
    usize_type(input.known()?)?;
    let associated = observation
        .associated_error
        .as_ref()
        .ok_or("Missing actual TryFrom Error observation")?;
    let error = nominal(
        endpoint(&associated.declared_type)?,
        error
            .nominal_parts()
            .ok_or("TryFrom Error lacks a concrete nominal owner")?
            .0,
    )?;
    if endpoint(&associated.declared_type)? != endpoint(&associated.normalized_projection)? {
        return Err("TryFrom declared Error differs from exact normalized projection".into());
    }

    let identity = Identity { success, error };
    definitions(facts, observation, &identity, authority)?;
    signature(
        facts,
        &observation.requested_signature,
        &identity,
        authority,
    )?;
    signature(facts, &observation.selected_signature, &identity, authority)?;
    captured_signature(facts)?;

    Ok(identity)
}

fn definitions(
    facts: &HirFnFacts,
    observed: &Observation,
    identity: &Identity,
    authority: &dyn CoreAuthority,
) -> Result<(), String> {
    let associated = observed
        .associated_error
        .as_ref()
        .ok_or("Missing TryFrom Error member")?;
    for definition in [
        &observed.method,
        &observed.implementation,
        &associated.implementation_member,
        &identity.success,
        &identity.error,
    ] {
        local(facts, definition)?;
    }
    crate::source_evidence::require_definition(
        facts,
        &observed.method.path,
        &observed.method.definition_identity,
    )?;
    for (role, definition) in [
        (CoreRole::Trait, &observed.trait_definition),
        (CoreRole::Method, &observed.trait_method),
        (CoreRole::ErrorMember, &associated.trait_member),
    ] {
        core(facts, role, definition, authority)?;
    }

    if observed.method != observed.selected
        || !observed.selected_arguments.is_empty()
        || observed.method_header.is_const
        || observed.method_header.is_async
        || observed.method_source.span != facts.fn_span
    {
        return Err("TryFrom owning selection/header/span contradicts original method".into());
    }

    Ok(())
}

pub(super) fn core(
    facts: &HirFnFacts,
    role: CoreRole,
    definition: &Definition,
    authority: &dyn CoreAuthority,
) -> Result<(), String> {
    let path = match role {
        CoreRole::Trait => "core::convert::TryFrom",
        CoreRole::Method => "core::convert::TryFrom::try_from",
        CoreRole::ErrorMember => "core::convert::TryFrom::Error",
        CoreRole::Result => "core::result::Result",
    };
    if definition.local
        || definition.crate_name != "core"
        || definition.definition_identity.is_empty()
        || definition.stable_crate_id.is_empty()
        || definition.path.replace("std::", "core::") != path
    {
        return Err("TryFrom core definition has an unsupported structural role".into());
    }
    authority.verify(facts, role, definition)
}

fn signature(
    facts: &HirFnFacts,
    signature: &Signature,
    identity: &Identity,
    authority: &dyn CoreAuthority,
) -> Result<(), String> {
    let [input] = signature.inputs.as_slice() else {
        return Err("TryFrom signature requires its single usize input".into());
    };
    usize_type(endpoint(input)?)?;
    let Structural::Adt {
        definition,
        arguments,
    } = endpoint(&signature.output)?
    else {
        return Err("TryFrom signature result lacks structural core Result identity".into());
    };
    core(facts, CoreRole::Result, definition, authority)?;
    let expected = |definition: &Definition| Structural::Adt {
        definition: definition.clone(),
        arguments: vec![],
    };
    if arguments != &[expected(&identity.success), expected(&identity.error)]
        || signature.bound_variables != 0
        || signature.abi != "\"Rust\""
        || signature.safety != "Safe"
        || signature.variadic
    {
        return Err(
            "TryFrom instantiated signature changes payload identity or safe Rust ABI".into(),
        );
    }

    Ok(())
}

fn captured_signature(facts: &HirFnFacts) -> Result<(), String> {
    if facts.signature.has_self
        || facts.signature.is_const
        || facts.signature.is_async
        || facts.signature.is_unsafe
        || facts.signature.is_variadic
        || facts.signature.abi.known_value() != Some("Rust")
        || !facts.signature.generics.is_empty()
        || facts.signature.params.len() != 1
        || facts.signature.params[0].ty.normalized.as_deref() != Some("usize")
    {
        return Err("TryFrom capture requires original safe nongeneric fn(usize)".into());
    }
    let (ok, err) = super::result(facts)?;
    let result = TypeToken::Result {
        ok: Box::new(TypeToken::from_tree(&ok)),
        err: Box::new(TypeToken::from_tree(&err)),
    };
    if facts.signature.return_ty.normalized.as_deref() != Some(result.text()?.as_str()) {
        return Err("TryFrom captured signature differs from concrete Result interface".into());
    }

    Ok(())
}
