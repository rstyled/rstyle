//! Explicit empty success and conditional error components; no flattened union.
use super::*;
use crate::copy_tagged_values::origins::{Component, FormalRoot, WireContract};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Serialize)]
struct Origin {
    root: FormalRoot,
    route: Vec<Component>,
}

#[derive(Serialize)]
struct Carried {
    role: &'static str,
    origins: Vec<Origin>,
}

#[derive(Clone, Serialize)]
struct Guard {
    scrutinee: crate::HirRef,
    pattern: crate::HirRef,
    selector_pdg_node_id: String,
    outcome: &'static str,
    variant: &'static str,
}

#[derive(Serialize)]
struct Helper<'a> {
    role: &'static str,
    guard: &'a Guard,
    actual: &'a crate::HirRef,
    callee_contract: &'a WireContract,
    origins: Vec<Origin>,
}

#[derive(Serialize)]
struct ErrorBranch<'a> {
    role: &'static str,
    guard: &'a Guard,
    origins: Vec<Origin>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Case<'a> {
    Success {
        variant: &'static str,
        selector_pdg_node_id: &'a str,
        origins: Vec<Origin>,
    },
    Error {
        variant: &'static str,
        guard: &'a Guard,
        origins: Vec<Origin>,
    },
}

#[derive(Serialize)]
struct ResultCases<'a> {
    role: &'static str,
    cases: [Case<'a>; 2],
}

pub(super) fn constructor(
    nodes: [&PdgNode; 5],
    contract: &WireContract,
) -> Result<BTreeMap<String, Value>, String> {
    let leaf = leaf(contract)?;
    let [input, read, named, wrapper, returned] = nodes;
    let mut map = BTreeMap::new();
    for node in [input, read] {
        map.insert(
            node.id.clone(),
            carried("static_input", &leaf.root, &leaf.input)?,
        );
    }

    let route = leaf
        .output
        .get(1..)
        .ok_or("Missing named error component route")?;
    map.insert(
        named.id.clone(),
        carried("named_variant", &leaf.root, route)?,
    );
    for node in [wrapper, returned] {
        map.insert(
            node.id.clone(),
            carried("whole_error", &leaf.root, &leaf.output)?,
        );
    }
    Ok(map)
}

pub(super) fn arithmetic(
    ops: &graph::arithmetic::Operations<'_>,
    root: &source::Root<'_>,
    binding: &crate::HirRef,
) -> Result<BTreeMap<String, Value>, String> {
    let leaf = leaf(&root.contract)?;
    let caller = FormalRoot {
        parameter: 2,
        binding: binding.clone(),
    };
    let mut map = empty_roles(ops)?;
    for node in [ops.inputs[2], ops.actual] {
        map.insert(
            node.id.clone(),
            carried("static_input", &caller, &leaf.input)?,
        );
    }

    let guard = Guard {
        scrutinee: root.option.scrutinee.hir.clone(),
        pattern: root.expression.children[root.option.none_arm + 1].children[0]
            .hir
            .clone(),
        selector_pdg_node_id: ops.none.id.clone(),
        outcome: "true",
        variant: "None",
    };
    let origin = Origin {
        root: caller,
        route: leaf.output.clone(),
    };
    map.extend(error_roles(ops, root, &guard, origin)?);
    Ok(map)
}

fn error_roles(
    ops: &graph::arithmetic::Operations<'_>,
    root: &source::Root<'_>,
    guard: &Guard,
    origin: Origin,
) -> Result<BTreeMap<String, Value>, String> {
    let helper = value(&Helper {
        role: "helper_error",
        guard,
        actual: &root.actual.hir,
        callee_contract: &root.contract,
        origins: vec![origin.clone()],
    })?;
    let error = Origin {
        root: origin.root,
        route: std::iter::once(Component::ResultErr)
            .chain(origin.route)
            .collect(),
    };
    let branch = value(&ErrorBranch {
        role: "error_branch",
        guard,
        origins: vec![error.clone()],
    })?;
    let output = value(&ResultCases {
        role: "result",
        cases: [
            Case::Success {
                variant: "Ok",
                selector_pdg_node_id: &ops.some.id,
                origins: vec![],
            },
            Case::Error {
                variant: "Err",
                guard,
                origins: vec![error],
            },
        ],
    })?;

    Ok(BTreeMap::from([
        (ops.call.id.clone(), helper),
        (ops.err.id.clone(), branch),
        (ops.merge.id.clone(), output.clone()),
        (ops.returned.id.clone(), output),
    ]))
}

fn empty_roles(ops: &graph::arithmetic::Operations<'_>) -> Result<BTreeMap<String, Value>, String> {
    let empty = value(&Carried {
        role: "scalar",
        origins: vec![],
    })?;
    let mut map: BTreeMap<_, _> = ops
        .inputs
        .into_iter()
        .chain([
            ops.left,
            ops.right,
            ops.checked,
            ops.some,
            ops.none,
            ops.payload,
            ops.binding,
            ops.read,
            ops.ok,
            ops.actual,
            ops.call,
            ops.err,
            ops.merge,
            ops.returned,
        ])
        .map(|node| (node.id.clone(), empty.clone()))
        .collect();
    map.insert(
        ops.ok.id.clone(),
        value(&Carried {
            role: "ok_branch",
            origins: vec![],
        })?,
    );
    Ok(map)
}

fn leaf(contract: &WireContract) -> Result<&crate::copy_tagged_values::origins::Leaf, String> {
    match contract.alternatives.as_slice() {
        [alternative] => match alternative.leaves.as_slice() {
            [leaf] => Ok(leaf),
            _ => Err("Fresh named error requires its single original static component".into()),
        },
        _ => Err("Fresh named error requires its unconditional complete constructor route".into()),
    }
}

fn carried(role: &'static str, root: &FormalRoot, route: &[Component]) -> Result<Value, String> {
    value(&Carried {
        role,
        origins: vec![Origin {
            root: root.clone(),
            route: route.to_vec(),
        }],
    })
}

fn value(source: &impl Serialize) -> Result<Value, String> {
    serde_json::to_value(source).map_err(|error| error.to_string())
}

pub(super) fn scalar_constructor(
    inputs: &[&PdgNode],
    reads: &[&PdgNode],
    outputs: [&PdgNode; 3],
    contract: &WireContract,
) -> Result<BTreeMap<String, Value>, String> {
    if !crate::helper_calls::named_errors::empty_scalar_contract(contract) {
        return Err("Scalar constructor cannot publish incomplete reference cases".into());
    }
    let empty = value(&Carried {
        role: "scalar",
        origins: vec![],
    })?;
    Ok(inputs
        .iter()
        .copied()
        .chain(reads.iter().copied())
        .chain(outputs)
        .map(|node| (node.id.clone(), empty.clone()))
        .collect())
}

pub(super) struct OwnedNodes<'a> {
    pub error: [&'a PdgNode; 2],
    pub message: [&'a PdgNode; 2],
    pub outputs: [&'a PdgNode; 3],
}

pub(super) fn owned_constructor(
    nodes: OwnedNodes<'_>,
    contract: &WireContract,
    transfer: &crate::named_error_payloads::owned::Transfer,
) -> Result<BTreeMap<String, Value>, String> {
    let [named, wrapper, returned] = nodes.outputs;

    let mut roles = constructor(
        [nodes.message[0], nodes.message[1], named, wrapper, returned],
        contract,
    )?;
    if transfer.parameter != 0 {
        return Err("Owned error constructor changes transferred input position".into());
    }
    for (node, role) in nodes
        .error
        .into_iter()
        .zip(["whole_owned_input", "whole_owned_move"])
    {
        super::super::guarded_sources::insert_role(
            &mut roles,
            node,
            Carried {
                role,
                origins: vec![],
            },
        )?;
    }
    Ok(roles)
}
