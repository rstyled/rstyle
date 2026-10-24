//! Bind one complete guarded source contract to independently replayed value roles.
use super::*;
use crate::copy_tagged_values::origins::{self as source, WireContract};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub(crate) const KEY: &str = "rstyle.data:copy-tagged-value-contract";
pub(crate) const ROLE_KEY: &str = "rstyle.data:copy-tagged-value-origin";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Role {
    Input,
    SelectedPayload,
    Ok,
    Err,
    Output,
}

#[derive(Debug, PartialEq, Eq)]
struct Roles {
    values: BTreeMap<String, Role>,
    /// Exact source scrutinee/pattern HirRefs bind to these independently found IDs.
    selector: Option<(String, String)>,
}

struct Types<'a> {
    input: &'a crate::concrete_types::TypeTree,
    product: &'a crate::concrete_types::TypeTree,
    output: &'a crate::concrete_types::TypeTree,
}

impl<'a> Types<'a> {
    fn checked(receipt: &'a CheckedCopyConversion) -> Self {
        Self {
            input: receipt.input(),
            product: receipt.product(),
            output: receipt.output(),
        }
    }
}

pub(crate) fn install(
    pair: &mut Pair,
    facts: &dyn EvidenceFacts,
    receipt: &CheckedCopyConversion,
) -> Result<(), String> {
    let contract = source::derive_source_contract(facts, receipt)?;
    let expected = source_roles(pair.as_view(), facts, receipt)?;
    let graph = graph_roles(pair.as_view(), facts, receipt)?;
    if expected != graph {
        return Err("Copy value source and graph roles disagree".into());
    }

    inventory(pair.as_view(), &expected, &Types::checked(receipt))?;
    pair.pdg.extensions.insert(
        KEY.into(),
        serde_json::to_value(WireContract::from_contract(&contract))
            .map_err(|error| error.to_string())?,
    );
    for node in &mut pair.pdg.nodes {
        if let Some(role) = expected.values.get(&node.id) {
            node.annotations.insert(
                ROLE_KEY.into(),
                serde_json::to_value(role).map_err(|error| error.to_string())?,
            );
        }
    }
    Ok(())
}

pub(crate) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    receipt: &CheckedCopyConversion,
) -> Result<(), String> {
    let wire: WireContract = serde_json::from_value(
        pair.pdg
            .extensions
            .get(KEY)
            .ok_or("Missing complete Copy value origin contract")?
            .clone(),
    )
    .map_err(|error| error.to_string())?;
    source::validate_wire_source_contract(&wire, facts, receipt)?;

    let expected = source_roles(pair, facts, receipt)?;
    let graph = graph_roles(pair, facts, receipt)?;
    if graph != expected {
        return Err("Copy value origins disagree with saved graph values".into());
    }
    inventory(pair, &expected, &Types::checked(receipt))?;
    annotations(pair, &expected)
}

fn annotations(pair: PairView<'_>, expected: &Roles) -> Result<(), String> {
    for node in &pair.pdg.nodes {
        let actual: Option<Role> = node
            .annotations
            .get(ROLE_KEY)
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|error| error.to_string())?;
        if actual.as_ref() != expected.values.get(&node.id) {
            return Err("Copy value node loses its exact complete-contract origin role".into());
        }
    }
    Ok(())
}

fn source_roles(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    receipt: &CheckedCopyConversion,
) -> Result<Roles, String> {
    let node = crate::copy_tagged_values::expression(facts)?;
    let formal = facts
        .signature
        .params
        .first()
        .ok_or("Missing Copy origin formal")?;
    let input = serialized_operation(pair, &formal.pat.span, |node| {
        node.category == NodeCategory::FormalInput
    })?;
    let returned = serialized_operation(pair, &node.span, |node| {
        node.category == NodeCategory::Return
    })?;

    let mut roles = Roles {
        values: BTreeMap::from([
            (input.id.clone(), Role::Input),
            (returned.id.clone(), Role::Output),
        ]),
        selector: None,
    };
    if receipt.error().is_none() {
        let tag = crate::copy_tagged_values::constructor(node, facts, receipt)?;
        source_wrapping(pair, node, &tag, &mut roles)?;
    } else {
        let checked = crate::copy_tagged_values::matched(node, facts, receipt)?;
        source_inverse(pair, &checked, &mut roles)?;
        roles
            .values
            .insert(value_operation(pair, node)?.id.clone(), Role::Output);
    }
    Ok(roles)
}

fn source_wrapping(
    pair: PairView<'_>,
    node: &HirNode,
    tag: &str,
    roles: &mut Roles,
) -> Result<(), String> {
    roles.values.extend([
        (
            value_operation(pair, &node.children[0])?.id.clone(),
            Role::Input,
        ),
        (
            operation(pair, node, "data.enum-value", Some(tag))?
                .id
                .clone(),
            Role::Output,
        ),
    ]);
    Ok(())
}

fn source_inverse(
    pair: PairView<'_>,
    checked: &crate::copy_tagged_values::Match<'_>,
    roles: &mut Roles,
) -> Result<(), String> {
    let saved = value_operation(pair, checked.scrutinee)?;
    let selector = operation(pair, checked.pattern, CONSTRUCTS[1], Some(&checked.tag))?;
    let payload = operation(pair, checked.binding, CONSTRUCTS[2], Some(&checked.tag))?;

    super::super::conditional_payloads::binding(pair, checked.binding, payload)?;
    roles.values.extend([
        (saved.id.clone(), Role::Input),
        (payload.id.clone(), Role::SelectedPayload),
        (
            value_operation(pair, &checked.success.children[0])?
                .id
                .clone(),
            Role::SelectedPayload,
        ),
        (value_operation(pair, checked.success)?.id.clone(), Role::Ok),
        (
            value_operation(pair, checked.failure)?.id.clone(),
            Role::Err,
        ),
    ]);
    roles.selector = Some((saved.id.clone(), selector.id.clone()));
    Ok(())
}

fn graph_roles(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    receipt: &CheckedCopyConversion,
) -> Result<Roles, String> {
    let input = unique(pair, |node| node.category == NodeCategory::FormalInput)?;
    let formal = facts
        .signature
        .params
        .first()
        .ok_or("Missing Copy origin formal")?;
    if input.attributes.position != Some(0)
        || input.attributes.symbol_ids != [formal.symbol.clone()]
    {
        return Err("Copy origin graph changes its exact formal position or symbol".into());
    }

    let saved = dependent(pair, input, "reaching_definition")?;
    if saved.category != NodeCategory::Read {
        return Err("Copy input must be one saved formal read".into());
    }
    verify_definition(pair, input, saved)?;
    super::graph::consumers(pair, input, &[saved], "reaching_definition")?;

    let mut roles = Roles {
        values: BTreeMap::from([
            (input.id.clone(), Role::Input),
            (saved.id.clone(), Role::Input),
        ]),
        selector: None,
    };
    let value = if receipt.error().is_none() {
        let value = construct(pair, "data.enum-value")?;
        verify_saved_operands(pair, value, &[saved])?;
        super::graph::consumers(pair, saved, &[value], "value_operand")?;
        value
    } else {
        graph_inverse(pair, saved, &mut roles)?
    };
    let returned = unique(pair, |node| node.category == NodeCategory::Return)?;

    verify_saved_operands(pair, returned, &[value])?;
    roles.values.extend([
        (value.id.clone(), Role::Output),
        (returned.id.clone(), Role::Output),
    ]);
    super::graph::returned(pair, crate::copy_tagged_values::expression(facts)?, value)?;
    Ok(roles)
}

fn graph_inverse<'a>(
    pair: PairView<'a>,
    saved: &'a PdgNode,
    roles: &mut Roles,
) -> Result<&'a PdgNode, String> {
    let selector = construct(pair, "match.enum-variant")?;
    let payload = construct(pair, "data.enum-payload")?;
    let binding = dependent(pair, payload, "value_operand")?;
    let read = dependent(pair, binding, "reaching_definition")?;

    verify_definition(pair, binding, read)?;
    if binding.category != NodeCategory::Declaration
        || read.category != NodeCategory::Read
        || binding.attributes.write_form.as_deref() != Some("initialization")
    {
        return Err(
            "Copy selected payload must initialize its exact immutable binding read".into(),
        );
    }
    let ops = super::graph::Operations {
        saved,
        selector,
        payload,
        binding,
        read,
        wildcard: construct(pair, "match.wildcard")?,
        ok: construct(pair, "data.result-ok")?,
        error: construct(pair, "data.struct")?,
        err: construct(pair, "data.result-err")?,
        merge: unique(pair, |node| node.category == NodeCategory::Merge)?,
    };

    super::graph::verify(pair, &ops)?;
    verify_saved_operands(pair, selector, &[saved])?;
    verify_saved_operands(pair, payload, &[saved])?;
    verify_saved_operands(pair, ops.ok, &[read])?;
    verify_saved_operands(pair, ops.err, &[ops.error])?;

    roles.values.extend([
        (payload.id.clone(), Role::SelectedPayload),
        (read.id.clone(), Role::SelectedPayload),
        (ops.ok.id.clone(), Role::Ok),
        (ops.err.id.clone(), Role::Err),
    ]);
    roles.selector = Some((saved.id.clone(), selector.id.clone()));
    Ok(ops.merge)
}

fn unique<'a>(
    pair: PairView<'a>,
    predicate: impl Fn(&PdgNode) -> bool,
) -> Result<&'a PdgNode, String> {
    let mut nodes = pair.pdg.nodes.iter().filter(|node| predicate(node));
    let node = nodes.next().ok_or("Missing Copy origin graph operation")?;
    if nodes.next().is_some() {
        return Err("Ambiguous Copy origin graph operation".into());
    }
    Ok(node)
}

fn construct<'a>(pair: PairView<'a>, construct: &str) -> Result<&'a PdgNode, String> {
    unique(pair, |node| {
        node.attributes.construct.as_deref() == Some(construct)
    })
}

fn dependent<'a>(pair: PairView<'a>, value: &PdgNode, kind: &str) -> Result<&'a PdgNode, String> {
    let mut edges = pair.pdg.edges.iter().filter(|edge| {
        edge.source == value.id && edge.kind == crate::EdgeFamily::Data && edge.subkind == kind
    });
    let edge = edges.next().ok_or("Missing Copy origin graph consumer")?;
    if edges.next().is_some() {
        return Err("Ambiguous Copy origin graph consumer".into());
    }
    unique(pair, |node| node.id == edge.target)
}

pub(in crate::data_documents) fn verify_definition(
    pair: PairView<'_>,
    binding: &PdgNode,
    read: &PdgNode,
) -> Result<(), String> {
    let edges: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == read.id && edge.kind == crate::EdgeFamily::Data)
        .collect();
    if !matches!(edges.as_slice(), [edge] if edge.source == binding.id && edge.subkind == "reaching_definition"
        && edge.symbol_id.as_ref() == binding.attributes.symbol_ids.first()
        && edge.definition_id.as_ref() == binding.attributes.defines.first())
        || binding.attributes.defines.len() != 1
        || binding.attributes.symbol_ids.len() != 1
        || read.attributes.symbol_ids != binding.attributes.symbol_ids
        || read.attributes.reads.len() != 1
        || read.attributes.reads[0].symbol_id.as_ref() != binding.attributes.symbol_ids.first()
        || read.attributes.reads[0].location_id.is_some()
    {
        return Err(
            "Copy origin read changes its exact reaching definition or formal symbol".into(),
        );
    }
    Ok(())
}

fn inventory(pair: PairView<'_>, expected: &Roles, types: &Types<'_>) -> Result<(), String> {
    for node in &pair.pdg.nodes {
        if node.annotations.contains_key(crate::data_origins::KEY) {
            return Err(
                "Copy value components must not claim legacy flattened reference origins".into(),
            );
        }
        let token = node
            .annotations
            .get("compat.rust.hir-r0:type")
            .and_then(|ty| ty.get("value"))
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing typed Copy value operation")?;
        let reference = crate::data_origins::shared(&crate::data_types::parse(token)?);
        let role = expected.values.get(&node.id);
        if reference != role.is_some() {
            return Err(
                "Copy value graph contains an unaccounted reference-bearing node or scalar role"
                    .into(),
            );
        }
        if let Some(role) = role {
            let tree = match role {
                Role::Input => types.input,
                Role::SelectedPayload => types.product,
                Role::Ok | Role::Err | Role::Output => types.output,
            };
            if token != crate::data_types::TypeToken::from_tree(tree).text()? {
                return Err("Copy value origin role disagrees with its exact endpoint type".into());
            }
        }
    }
    Ok(())
}
