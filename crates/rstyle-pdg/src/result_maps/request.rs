//! An owned source request, constructed before nominal and helper resolution.
use super::{model::*, shape};
use crate::{
    HirFnFacts, HirRef,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    generated_sources::{capture::Definition, checked::CheckedMember},
    nominal_boundary::census::expansions::FileSpan,
};
use serde::Deserialize;
mod identities;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransportRequest {
    results: [TypeTree; 3],
    constructors: [Definition; 2],
    calls: [Call; 2],
    roles: Vec<Role>,
    leaf: HirRef,
    root: HirRef,
    trait_leaf: Option<crate::tryfrom_calls::model::Call>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Role {
    path: Vec<u32>,
    hir: HirRef,
    kind: String,
    source: FileSpan,
}

impl TransportRequest {
    pub(crate) fn trait_leaf(&self) -> Option<&crate::tryfrom_calls::model::Call> {
        self.trait_leaf.as_ref()
    }

    pub(crate) fn result_types(&self) -> [&TypeTree; 3] {
        [&self.results[0], &self.results[1], &self.results[2]]
    }
    pub(crate) fn copy_requests(&self) -> [&TypeTree; 2] {
        self.payload_requests(false)
    }
    pub(crate) fn cleanup_requests(&self) -> [&TypeTree; 2] {
        self.payload_requests(true)
    }
    fn payload_requests(&self, error: bool) -> [&TypeTree; 2] {
        let TypeTree::Result { ok, err } = &self.results[2] else {
            unreachable!()
        };
        let wrapper = if error { err } else { ok };
        [
            shape::unary(wrapper).expect("checked unary wrapper"),
            wrapper,
        ]
    }
    pub(crate) fn constructors(&self) -> [&Definition; 2] {
        [&self.constructors[0], &self.constructors[1]]
    }
    pub(crate) fn map_hirs(&self) -> [&HirRef; 2] {
        [&self.calls[0].hir, &self.calls[1].hir]
    }
    pub(super) fn calls(&self) -> [&Call; 2] {
        [&self.calls[0], &self.calls[1]]
    }
}

pub(crate) fn verify(
    facts: &HirFnFacts,
    member: &CheckedMember,
) -> Result<Option<TransportRequest>, String> {
    if !source_shape(member) {
        return Ok(None);
    }
    let chain = shape::chain(facts)?;
    let results = results(facts)?;
    let trait_leaf = super::trait_leaf::capture(facts, member, &results[0], chain.leaf)?;
    let calls = calls(
        facts,
        [&chain.map.hir, &chain.root.hir],
        trait_leaf.is_some(),
    )?;
    identities::verify(facts, &calls)?;
    let constructors = substitutions(&results, &calls)?;
    exact_types(&chain, &results)?;
    parameter(facts, &chain)?;
    let roles = roles(member, facts)?;

    Ok(Some(TransportRequest {
        results,
        trait_leaf,
        constructors,
        calls,
        roles,
        leaf: chain.leaf.hir.clone(),
        root: chain.root.hir.clone(),
    }))
}

fn source_shape(member: &CheckedMember) -> bool {
    let [syn::Stmt::Expr(syn::Expr::MethodCall(outer), None)] =
        member.function().block.stmts.as_slice()
    else {
        return false;
    };
    let syn::Expr::MethodCall(inner) = outer.receiver.as_ref() else {
        return false;
    };
    outer.method == "map_err"
        && inner.method == "map"
        && outer.args.len() == 1
        && inner.args.len() == 1
        && matches!(inner.receiver.as_ref(), syn::Expr::Call(_))
}

fn results(facts: &HirFnFacts) -> Result<[TypeTree; 3], String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;
    let Observation::Captured {
        ty: output @ TypeTree::Result { ok, err },
    } = &interface.result
    else {
        return Err("Missing complete transport Result interface".into());
    };
    let success = shape::unary(ok)?;
    let error = shape::unary(err)?;
    if !shape::scalar_product(success)
        || !shape::error_product(error)
        || !matches!(interface.parameters.as_slice(), [Observation::Captured {
            ty }] if crate::checked_index_bytes::parameter(facts, ty))
    {
        return Err(
            "Transport requires finite scalar success and named scalar error products".into(),
        );
    }

    Ok([
        TypeTree::Result {
            ok: Box::new(success.clone()),
            err: Box::new(error.clone()),
        },
        TypeTree::Result {
            ok: ok.clone(),
            err: Box::new(error.clone()),
        },
        output.clone(),
    ])
}

fn calls(facts: &HirFnFacts, hirs: [&HirRef; 2], conversion: bool) -> Result<[Call; 2], String> {
    let raw = facts
        .build
        .frontend_options
        .get("rstyle.rust.call-instances")
        .ok_or("Missing constructor transport call-instance observations")?;
    let inventory = Inventory::deserialize(raw).map_err(|e| e.to_string())?;
    if inventory.identity != "rstyle.rust.call-instance-observations/1"
        || !inventory.complete
        || inventory.scope.is_empty()
        || inventory.calls.len() != if conversion { 3 } else { 2 }
    {
        return Err("Incomplete constructor transport call-instance roster".into());
    }
    let select = |hir: &HirRef| {
        let found: Vec<_> = inventory
            .calls
            .iter()
            .filter(|call| &call.hir == hir)
            .collect();
        match found.as_slice() {
            [call] => Ok((*call).clone()),
            _ => Err("Missing or duplicated transport method call observation".to_string()),
        }
    };

    Ok([select(hirs[0])?, select(hirs[1])?])
}

fn substitutions(results: &[TypeTree; 3], calls: &[Call; 2]) -> Result<[Definition; 2], String> {
    let TypeTree::Result {
        ok: base,
        err: error,
    } = &results[0]
    else {
        unreachable!()
    };
    let TypeTree::Result {
        ok: wrapped,
        err: wrapped_error,
    } = &results[2]
    else {
        unreachable!()
    };
    Ok([
        substitution(&calls[0], [base, error, wrapped], "map")?,
        substitution(&calls[1], [wrapped, error, wrapped_error], "map_err")?,
    ])
}

fn substitution(call: &Call, trees: [&TypeTree; 3], name: &str) -> Result<Definition, String> {
    if call.status != "resolved"
        || call.requested != call.selected
        || call.requested_arguments != call.selected_arguments
        || call.selected.local
        || call.selected.crate_name != "core"
        || call.selected.path != format!("core::result::Result::<T, E>::{name}")
        || !call.signature.display_only
        || call.signature.inputs.len() != 2
        || call.signature.output.is_empty()
    {
        return Err("Transport method lacks its exact inherent core substitution".into());
    }
    let [first, second, third, callback] = call.selected_arguments.as_slice() else {
        return Err("Transport method requires four complete structural substitutions".into());
    };
    for (argument, tree) in [first, second, third].into_iter().zip(trees) {
        argument_type(argument)?;
        structural(&argument.identity, tree)?;
    }
    argument_type(callback)?;
    let input = if name == "map" { trees[0] } else { trees[1] };

    constructor(&callback.identity, input, trees[2])
}

fn argument_type(argument: &Argument) -> Result<(), String> {
    if argument.kind != "type"
        || argument.value.is_empty()
        || !argument.display_only
        || argument.identity_status != "known"
    {
        return Err("Unknown or non-type constructor transport substitution".into());
    }
    Ok(())
}

fn structural(value: &Structural, tree: &TypeTree) -> Result<(), String> {
    let Structural::Adt {
        definition,
        arguments,
    } = value
    else {
        return Err("Transport payload substitution must identify its nominal ADT".into());
    };
    let nominal = tree
        .nominal_identity()
        .ok_or("Transport payload lacks nominal identity")?;
    let path = definition
        .path
        .strip_prefix(&format!("{}::", definition.crate_name))
        .unwrap_or(&definition.path);
    if !arguments.is_empty()
        || !definition.local
        || definition.crate_name.is_empty()
        || definition.stable_crate_id.is_empty()
        || definition.definition_identity.is_empty()
        || path != nominal.definition_path
        || tree
            .generated_parts()
            .is_some_and(|(_, observed, ..)| observed != definition)
    {
        return Err("Transport structural ADT differs from its captured nominal".into());
    }

    Ok(())
}

fn constructor(
    value: &Structural,
    input: &TypeTree,
    output: &TypeTree,
) -> Result<Definition, String> {
    let Structural::TupleConstructor {
        definition,
        owner,
        arguments,
        signature,
    } = value
    else {
        return Err("Transport callback is not an actual unary struct constructor FnDef".into());
    };
    structural(
        &Structural::Adt {
            definition: owner.clone(),
            arguments: vec![],
        },
        output,
    )?;
    structural(&signature.input, input)?;
    structural(&signature.output, output)?;
    if !arguments.is_empty()
        || signature.bound_variables != 0
        || signature.abi != "\"Rust\""
        || signature.safety != "Safe"
        || signature.variadic
        || !definition.local
        || definition.crate_name != owner.crate_name
        || definition.stable_crate_id != owner.stable_crate_id
        || definition.definition_identity.is_empty()
        || definition == owner
    {
        return Err("Transport constructor has unsupported owner, ABI or generic signature".into());
    }

    Ok(definition.clone())
}

fn exact_types(chain: &shape::Chain<'_>, results: &[TypeTree; 3]) -> Result<(), String> {
    for (node, tree) in [chain.leaf, chain.map, chain.root].into_iter().zip(results) {
        let expected = crate::data_types::TypeToken::from_tree(tree);
        crate::data_aggregates::verify_node_type(
            node,
            &expected,
            "Transport Result type disagrees",
        )?;
    }
    Ok(())
}

fn parameter(facts: &HirFnFacts, chain: &shape::Chain<'_>) -> Result<(), String> {
    let pattern = &facts.signature.params[0].pat;
    let [binding] = facts.bindings.as_slice() else {
        return Err("Transport requires exactly its single scalar parameter binding".into());
    };
    let resolution = chain
        .argument
        .res
        .as_ref()
        .ok_or("Missing transport scalar parameter use")?;
    if resolution.kind != "local"
        || resolution.target.as_ref() != Some(&pattern.hir)
        || pattern.binding_mut == Some(true)
        || pattern.binding.is_none()
        || chain.argument.dispatch.is_some()
        || binding.hir != pattern.hir
        || !binding.is_param
        || binding.is_mut
        || pattern.binding.as_ref() != Some(&binding.symbol)
        || facts.signature.params[0].symbol != binding.symbol
    {
        return Err("Transport leaf does not consume its exact immutable scalar parameter".into());
    }

    Ok(())
}

fn roles(member: &CheckedMember, facts: &HirFnFacts) -> Result<Vec<Role>, String> {
    if member.roles().len() != 8 || facts.body.is_unsafe_block {
        return Err("Transport source role roster is not its complete bounded chain".into());
    }
    for role in member.roles() {
        let node = role_node(facts, role.path())?;
        if node.hir != *role.hir() || node.kind != role.kind() {
            return Err("Transport HIR no longer matches its checked exact source role".into());
        }
    }
    Ok(member
        .roles()
        .iter()
        .map(|role| Role {
            path: role.path().to_vec(),
            hir: role.hir().clone(),
            kind: role.kind().into(),
            source: role.source().clone(),
        })
        .collect())
}

fn role_node<'a>(facts: &'a HirFnFacts, path: &[u32]) -> Result<&'a crate::HirNode, String> {
    let (mut node, rest) = match path {
        [0, parameter, rest @ ..] => (
            &facts
                .signature
                .params
                .get(*parameter as usize)
                .ok_or("Transport role has an unknown parameter")?
                .pat,
            rest,
        ),
        [1, rest @ ..] => (&facts.body, rest),
        _ => return Err("Transport source role has an unknown root".into()),
    };
    for index in rest {
        node = node
            .children
            .get(*index as usize)
            .ok_or("Transport role has an unknown child")?;
    }
    Ok(node)
}
