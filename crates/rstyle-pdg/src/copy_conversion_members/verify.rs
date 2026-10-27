//! Structural substitutions, selected owner, complete members and associated Error.
use super::{Contract, Direction, Observation, authority::Pins, model::*};
use crate::tryfrom_calls::endpoint;
use crate::{
    HirFnFacts,
    concrete_types::{InterfaceObservations, Nominal, TypeTree},
};

pub(super) struct Owner<'a> {
    pub name: &'a str,
    pub identity: &'a str,
    pub crate_name: &'a str,
    pub stable_crate_id: &'a str,
}

impl<'a> Owner<'a> {
    pub(super) fn from_facts(facts: &'a HirFnFacts) -> Result<Self, String> {
        let options = &facts.build.frontend_options;
        let compiled = options
            .get("rstyle.rust.compiled-crate")
            .ok_or("Copy conversion lacks compiled owner")?;
        if compiled["identity"] != "rstyle.rust.compiled-crate/1" {
            return Err("Copy conversion has another compiled owner schema".into());
        }
        Ok(Self {
            name: &facts.name,
            identity: options
                .get("definition_identity")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or("Copy conversion method identity missing")?,
            crate_name: compiled["crate_name"]
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or("Copy conversion crate name missing")?,
            stable_crate_id: compiled["stable_crate_id"]
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or("Copy conversion stable crate missing")?,
        })
    }

    fn local(&self, definition: &Definition) -> Result<(), String> {
        if !definition.local
            || definition.path.is_empty()
            || definition.definition_identity.is_empty()
            || definition.crate_name != self.crate_name
            || definition.stable_crate_id != self.stable_crate_id
        {
            return Err("Copy conversion definition differs from its selected local owner".into());
        }
        Ok(())
    }
}

pub(super) fn captured(facts: &HirFnFacts, contract: &Contract<'_>) -> Result<(), String> {
    use crate::concrete_types::Observation::Captured;
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;

    if interface.parameters.as_slice()
        != [Captured {
            ty: contract.input.clone(),
        }]
        || interface.result
            != (Captured {
                ty: contract.output.clone(),
            })
        || facts.signature.has_self
        || facts.signature.is_const
        || facts.signature.is_async
        || facts.signature.is_unsafe
        || facts.signature.is_variadic
        || !facts.signature.generics.is_empty()
        || facts.signature.abi.known_value() != Some("Rust")
        || facts.signature.params.len() != 1
        || contract.method_source.span != facts.fn_span
    {
        return Err(
            "Copy conversion differs from complete safe concrete captured interface".into(),
        );
    }

    let input = crate::data_types::TypeToken::from_tree(contract.input).text()?;
    let output = crate::data_types::TypeToken::from_tree(contract.output).text()?;
    if facts.signature.params[0].ty.normalized.as_deref() != Some(input.as_str())
        || facts.signature.return_ty.normalized.as_deref() != Some(output.as_str())
    {
        return Err(
            "Copy conversion normalized interface differs from captured semantic types".into(),
        );
    }
    Ok(())
}

pub(super) fn receipt(
    observed: &Observation,
    contract: &Contract<'_>,
    owner: &Owner<'_>,
    pins: &Pins<'_>,
) -> Result<(), String> {
    selection(observed, contract, owner)?;
    pins.conversion(contract.direction, observed)?;
    let [self_type, input] = observed.trait_arguments.as_slice() else {
        return Err("Copy conversion needs exactly Self and input substitutions".into());
    };
    nominal(input.known()?, contract.input, owner)?;

    let success = match (contract.direction, contract.output) {
        (Direction::Wrap, output) => output,
        (Direction::Unwrap, TypeTree::Result { ok, .. }) => ok.as_ref(),
        _ => return Err("Inverse Copy conversion requires complete Result output".into()),
    };
    nominal(self_type.known()?, success, owner)?;

    substitutions(observed, contract.direction)?;
    signature(&observed.requested_signature, contract, owner, pins)?;
    signature(&observed.selected_signature, contract, owner, pins)?;
    associated(observed, contract, owner, pins)?;
    members(observed)?;
    consistent_definitions(observed)
}

fn selection(
    observed: &Observation,
    contract: &Contract<'_>,
    owner: &Owner<'_>,
) -> Result<(), String> {
    if observed.identity != "rstyle.rust.concrete-conversion-member-observation/1"
        || observed.admission_authority
        || observed.method != observed.selected
        || !observed.selected_arguments.is_empty()
        || observed.method_header.is_const
        || observed.method_header.is_async
        || observed.method.path != owner.name
        || observed.method.definition_identity != owner.identity
        || observed.method_source != *contract.method_source
        || observed.implementation_source != *contract.implementation_source
    {
        return Err("Copy conversion selection, substitution, source or header differs".into());
    }

    owner.local(&observed.method)?;
    owner.local(&observed.implementation)?;
    let member = match contract.direction {
        Direction::Wrap => "from",
        Direction::Unwrap => "try_from",
    };
    if observed.method.path != format!("{}::{member}", observed.implementation.path) {
        return Err("Copy conversion method is not a member of its exact original impl".into());
    }
    Ok(())
}

fn nominal(actual: &Structural, expected: &TypeTree, owner: &Owner<'_>) -> Result<(), String> {
    let Structural::Adt {
        definition,
        arguments,
    } = actual
    else {
        return Err("Copy conversion endpoint lacks exact nominal identity".into());
    };
    let (nominal, _) = expected
        .nominal_parts()
        .ok_or("Copy conversion endpoint is not nominal")?;

    nominal_path(definition, nominal, owner)?;
    if !arguments.is_empty() {
        return Err("Copy conversion nominal is not concrete nongeneric".into());
    }
    Ok(())
}

fn nominal_path(
    definition: &Definition,
    nominal: &Nominal,
    owner: &Owner<'_>,
) -> Result<(), String> {
    owner.local(definition)?;
    let path = definition
        .path
        .strip_prefix(&format!("{}::", definition.crate_name))
        .unwrap_or(&definition.path);
    if path != nominal.definition_path {
        return Err(
            "Copy conversion structural nominal differs from captured semantic type".into(),
        );
    }
    Ok(())
}

fn signature(
    actual: &Signature,
    contract: &Contract<'_>,
    owner: &Owner<'_>,
    pins: &Pins<'_>,
) -> Result<(), String> {
    let [input] = actual.inputs.as_slice() else {
        return Err("Copy conversion requires one input".into());
    };
    if actual.bound_variables != 0
        || actual.abi != "\"Rust\""
        || actual.safety != "Safe"
        || actual.variadic
    {
        return Err("Copy conversion loses concrete safe Rust ABI".into());
    }

    nominal(endpoint(input)?, contract.input, owner)?;
    let output = endpoint(&actual.output)?;
    match (contract.direction, contract.output, output) {
        (Direction::Wrap, expected, actual) => nominal(actual, expected, owner),
        (
            Direction::Unwrap,
            TypeTree::Result { ok, err },
            Structural::Adt {
                definition,
                arguments,
            },
        ) => {
            pins.role(definition, "result")?;
            let [success, error] = arguments.as_slice() else {
                return Err("Copy conversion Result payload roster differs".into());
            };
            nominal(success, ok, owner)?;
            nominal(error, err, owner)
        }
        _ => Err("Copy conversion signature changes complete return shape".into()),
    }
}

fn associated(
    observed: &Observation,
    contract: &Contract<'_>,
    owner: &Owner<'_>,
    pins: &Pins<'_>,
) -> Result<(), String> {
    match (
        contract.direction,
        &observed.associated_error,
        contract.error_source,
    ) {
        (Direction::Wrap, None, None) => Ok(()),
        (Direction::Unwrap, Some(error), Some(source)) => {
            let TypeTree::Result { err, .. } = contract.output else {
                return Err("Missing inverse Result output".into());
            };
            owner.local(&error.implementation_member)?;
            pins.role(&error.trait_member, "error")?;
            if error.source != *source
                || error.implementation_member.path
                    != format!("{}::Error", observed.implementation.path)
                || error.declared_type != error.normalized_projection
            {
                return Err("Copy conversion associated Error source/projection differs".into());
            }
            nominal(endpoint(&error.declared_type)?, err, owner)
        }
        _ => Err("Copy conversion associated Error roster differs from direction".into()),
    }
}

fn members(observed: &Observation) -> Result<(), String> {
    let mut expected = Vec::new();
    if let Some(error) = &observed.associated_error {
        expected.push((
            &error.implementation_member,
            &error.trait_member,
            &error.source,
        ));
    }
    expected.push((
        &observed.method,
        &observed.trait_method,
        &observed.method_source,
    ));

    if observed.members.len() != expected.len() {
        return Err("Copy conversion member roster incomplete".into());
    }
    for (actual, (definition, trait_member, source)) in observed.members.iter().zip(expected) {
        if &actual.definition != definition
            || actual.trait_member.as_ref() != Some(trait_member)
            || &actual.source != source
        {
            return Err("Copy conversion exact member identity/source roster differs".into());
        }
    }
    Ok(())
}

fn substitutions(observed: &Observation, direction: Direction) -> Result<(), String> {
    let [self_type, input] = observed.trait_arguments.as_slice() else {
        return Err("Copy conversion trait substitution roster differs".into());
    };
    for signature in [&observed.requested_signature, &observed.selected_signature] {
        let [actual_input] = signature.inputs.as_slice() else {
            return Err("Copy conversion signature input roster differs".into());
        };
        if endpoint(actual_input)? != input.known()? {
            return Err("Copy conversion signature input changes selected nominal identity".into());
        }
        let output = endpoint(&signature.output)?;
        match (direction, output) {
            (Direction::Wrap, actual) if actual == self_type.known()? => {}
            (Direction::Unwrap, Structural::Adt { arguments, .. }) => {
                let error = observed
                    .associated_error
                    .as_ref()
                    .ok_or("Missing inverse Error")?;
                if arguments.as_slice()
                    != [
                        self_type.known()?.clone(),
                        endpoint(&error.declared_type)?.clone(),
                    ]
                {
                    return Err("Copy conversion Result changes selected payload identity".into());
                }
            }
            _ => return Err("Copy conversion result changes selected Self identity".into()),
        }
    }
    Ok(())
}

/// One compiler identity may recur, but must always describe the same definition.
#[derive(Default)]
struct Definitions<'a> {
    records: std::collections::BTreeMap<(&'a str, &'a str), &'a Definition>,
}

impl<'a> Definitions<'a> {
    fn insert(&mut self, definition: &'a Definition) -> Result<(), String> {
        let key = (
            definition.stable_crate_id.as_str(),
            definition.definition_identity.as_str(),
        );
        if self
            .records
            .insert(key, definition)
            .is_some_and(|old| old != definition)
        {
            return Err(
                "Copy conversion compiler identity describes contradictory definitions".into(),
            );
        }
        Ok(())
    }

    fn arguments(&mut self, arguments: &'a [Argument]) -> Result<(), String> {
        if arguments.len() > 256 {
            return Err("Copy conversion structural identity inventory exceeds its bound".into());
        }
        let mut pending = arguments
            .iter()
            .map(Argument::known)
            .collect::<Result<Vec<_>, _>>()?;

        let mut visited = 0;
        while let Some(identity) = pending.pop() {
            visited += 1;
            if visited > 256 || pending.len() > 256 {
                return Err(
                    "Copy conversion structural identity inventory exceeds its bound".into(),
                );
            }
            if let Structural::Adt {
                definition,
                arguments,
            } = identity
            {
                self.insert(definition)?;
                if arguments.len() > 256 - pending.len() {
                    return Err(
                        "Copy conversion structural identity inventory exceeds its bound".into(),
                    );
                }
                pending.extend(arguments);
            }
        }
        Ok(())
    }

    fn signature(&mut self, signature: &'a Signature) -> Result<(), String> {
        for arguments in signature.inputs.iter().chain([&signature.output]) {
            self.arguments(arguments)?;
        }
        Ok(())
    }
}

fn consistent_definitions(observed: &Observation) -> Result<(), String> {
    let mut definitions = Definitions::default();
    let members = observed
        .members
        .iter()
        .flat_map(|member| std::iter::once(&member.definition).chain(member.trait_member.as_ref()));
    let error_members = observed
        .associated_error
        .iter()
        .flat_map(|error| [&error.implementation_member, &error.trait_member]);
    for definition in [
        &observed.method,
        &observed.implementation,
        &observed.selected,
        &observed.trait_definition,
        &observed.trait_method,
    ]
    .into_iter()
    .chain(members)
    .chain(error_members)
    {
        definitions.insert(definition)?;
    }

    let error_arguments = observed
        .associated_error
        .iter()
        .flat_map(|error| [&error.declared_type, &error.normalized_projection]);
    for arguments in [&observed.trait_arguments, &observed.selected_arguments]
        .into_iter()
        .chain(error_arguments)
    {
        definitions.arguments(arguments)?;
    }
    for signature in [&observed.requested_signature, &observed.selected_signature] {
        definitions.signature(signature)?;
    }
    Ok(())
}
