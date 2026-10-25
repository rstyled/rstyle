//! Independent guarded consuming-callback checks; callee conditions stay call-scoped.
use super::*;
use crate::{
    nonzero_digit_calls::{self, Checked},
    unit_store::EvidenceFacts,
};

const KEY: &str = "rstyle.data:nonzero-digit-composition";
const SITES: &str = "compat.rust.hir-r0:value-sites";

fn contract(checked: &Checked<'_>) -> Result<serde_json::Value, String> {
    let receipt = Composition {
        identity: "rstyle.data.nonzero-digit-composition/1",
        parser: &checked.parser().hir,
        callback: &checked.calls.callback.hir,
        member: &checked.calls.nested["hir"],
        callee: checked.dependency.wire()?,
        input: &checked.input,
        output: &checked.output,
        semantics: "None -> None; Some(suffix,scalar) -> save suffix; new(scalar) once; zero -> None; nonzero -> Some(suffix,nonzero)/1",
        origins: "Some-only field[0] from saved callee result; no suffix reads; None reference-free/1",
        conditions: "callee-graph-scoped-under-parser-call/1",
    };
    serde_json::to_value(receipt).map_err(|error| error.to_string())
}

pub(super) fn install(pair: &mut Pair, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = nonzero_digit_calls::verify(facts)? else {
        return Ok(false);
    };

    graph(pair.as_view(), facts, &checked)?;
    pair.pdg.extensions.insert(KEY.into(), contract(&checked)?);

    Ok(true)
}

pub(super) fn verify(pair: PairView<'_>, facts: &dyn EvidenceFacts) -> Result<bool, String> {
    let Some(checked) = nonzero_digit_calls::verify(facts)? else {
        if pair.pdg.extensions.contains_key(KEY) {
            return Err("Unproved nonzero composition".into());
        }
        return Ok(false);
    };
    crate::extraction_profile::require_document(
        &pair.pdg.profile,
        nonzero_digit_calls::CAPABILITY,
        || true,
        "Nonzero consumption requires its registered document policy",
    )?;

    if pair.pdg.extensions.get(KEY) != Some(&contract(&checked)?) {
        return Err(
            "Nonzero composition changes its selected callback, input/output or scoped callee"
                .into(),
        );
    }

    graph(pair, facts, &checked)?;

    Ok(true)
}

fn graph(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &Checked<'_>,
) -> Result<(), String> {
    if pair.pdg.nodes.len() != 17 {
        return Err("Nonzero composition requires its entire guarded value graph".into());
    }

    let values = values(pair, checked)?;

    let input = input(pair, facts)?;

    super::copy_tagged_values::origins::verify_definition(pair, input, values["saved"])?;
    edges(pair, &values)?;

    let ordered = [
        "parser",
        "environment",
        "some",
        "payload",
        "suffix",
        "scalar",
        "nonzero",
        "value",
        "product",
        "success",
    ]
    .map(|role| values[role]);
    super::callbacks::order::sequence(pair, &ordered)?;

    branches(pair, checked, &values)?;

    returned(pair, &values, output_token(checked)?)?;
    origins(pair)
}

fn values<'a>(
    pair: PairView<'a>,
    checked: &Checked<'_>,
) -> Result<BTreeMap<&'static str, &'a PdgNode>, String> {
    let specs = value_specs(checked)?;

    let mut values = specs
        .into_iter()
        .map(|(role, ty, category)| operation(pair, role, &ty, category).map(|node| (role, node)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let saved = super::value_operation(pair, &checked.parser().children[0])?;

    let parser = super::value_operation(pair, checked.parser())?;

    if saved.category != NodeCategory::Read || parser.category != NodeCategory::Call {
        return Err("Nonzero composition loses its once-only parser input evaluation".into());
    }
    values.insert("saved", saved);
    values.insert("parser", parser);

    Ok(values)
}

fn value_specs(checked: &Checked<'_>) -> Result<[(&'static str, String, NodeCategory); 9], String> {
    Ok([
        ("environment", "unit".into(), NodeCategory::Computation),
        ("some", "bool".into(), NodeCategory::Control),
        (
            "payload",
            checked.input.type_token(),
            NodeCategory::Computation,
        ),
        (
            "suffix",
            crate::data_types::concrete_text(&checked.input.fields[0].ty),
            NodeCategory::Computation,
        ),
        (
            "scalar",
            checked.input.argument.as_str().into(),
            NodeCategory::Computation,
        ),
        ("nonzero", "bool".into(), NodeCategory::Control),
        (
            "value",
            crate::data_types::concrete_text(&checked.output.fields[1].ty),
            NodeCategory::Computation,
        ),
        (
            "product",
            checked.output.type_token(),
            NodeCategory::Computation,
        ),
        (
            "success",
            output_token(checked)?.into(),
            NodeCategory::Computation,
        ),
    ])
}

fn edges(pair: PairView<'_>, values: &BTreeMap<&str, &PdgNode>) -> Result<(), String> {
    let specs: [(&str, &[&str]); 10] = [
        ("parser", &["saved"]),
        ("environment", &[]),
        ("some", &["parser"]),
        ("payload", &["parser"]),
        ("suffix", &["payload"]),
        ("scalar", &["payload"]),
        ("nonzero", &["scalar"]),
        ("value", &["scalar"]),
        ("product", &["suffix", "value"]),
        ("success", &["product"]),
    ];

    for (consumer, sources) in specs {
        let sources: Vec<_> = sources.iter().map(|role| values[role]).collect();
        operands(pair, values[consumer], &sources)?;
    }

    Ok(())
}

fn output_token<'a>(checked: &'a Checked<'_>) -> Result<&'a str, String> {
    checked
        .calls
        .outer
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .ok_or_else(|| "Missing complete nonzero output type".into())
}

fn operation<'a>(
    pair: PairView<'a>,
    role: &str,
    ty: &str,
    category: NodeCategory,
) -> Result<&'a PdgNode, String> {
    let construct = format!("callback.nonzero-{role}");

    let nodes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some(&construct))
        .collect();

    let [node] = nodes.as_slice() else {
        return Err(format!(
            "Nonzero graph requires exactly one {role} operation"
        ));
    };

    super::callbacks::effects(node, category)?;

    if node.annotations["compat.rust.hir-r0:type"]["value"].as_str() != Some(ty) {
        return Err("Nonzero graph changes its exact saved or constructed type".into());
    }

    Ok(node)
}

fn operands(pair: PairView<'_>, consumer: &PdgNode, operands: &[&PdgNode]) -> Result<(), String> {
    super::verify_saved_operands(pair, consumer, operands)
}

fn input<'a>(pair: PairView<'a>, facts: &dyn EvidenceFacts) -> Result<&'a PdgNode, String> {
    let nodes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::FormalInput)
        .collect();

    let [input] = nodes.as_slice() else {
        return Err("Nonzero graph requires its sole complete formal".into());
    };

    if input.attributes.position != Some(0)
        || input.attributes.symbol_ids != [facts.signature.params[0].symbol.clone()]
    {
        return Err("Nonzero graph changes its formal binding".into());
    }

    Ok(input)
}

fn branches(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    values: &BTreeMap<&str, &PdgNode>,
) -> Result<(), String> {
    let some = values["some"];

    let nonzero = values["nonzero"];

    let payloads = [
        values["payload"],
        values["suffix"],
        values["scalar"],
        nonzero,
    ];

    let successful = [values["value"], values["product"], values["success"]];

    let guards = super::conditional_payloads::Guards::new(pair)?;

    for node in payloads.iter().chain(&successful) {
        guards.guarded(some, node, crate::CfgEdgeKind::True)?;
    }

    for node in successful {
        guards.guarded(nonzero, node, crate::CfgEdgeKind::True)?;
    }

    for node in pair.pdg.nodes.iter().filter(|node| {
        node.attributes
            .construct
            .as_deref()
            .is_some_and(|value| value.starts_with("callback.nonzero-"))
    }) {
        let source = if node.attributes.construct.as_deref() == Some("callback.nonzero-environment")
        {
            checked.calls.callback
        } else {
            checked.calls.outer
        };

        if node.source_span.as_ref().is_none_or(|span| {
            span.start != u64::from(source.span.start) || span.end != u64::from(source.span.end)
        }) {
            return Err(
                "Nonzero graph changes its original summary/callback source boundary".into(),
            );
        }
    }

    Ok(())
}

fn returned(pair: PairView<'_>, values: &BTreeMap<&str, &PdgNode>, ty: &str) -> Result<(), String> {
    let some = values["some"];

    let nonzero = values["nonzero"];

    let success = values["success"];

    let left = operation(pair, "zero-none", ty, NodeCategory::Computation)?;

    let right = operation(pair, "parser-none", ty, NodeCategory::Computation)?;

    for none in [left, right] {
        operands(pair, none, &[])?;
    }

    let inner = merge(pair, nonzero, success, left)?;

    let outer = merge(pair, some, inner, right)?;

    super::conditional_payloads::guarded(pair, nonzero, left, crate::CfgEdgeKind::False)?;

    super::conditional_payloads::guarded(pair, some, left, crate::CfgEdgeKind::True)?;

    super::conditional_payloads::guarded(pair, some, right, crate::CfgEdgeKind::False)?;

    let returned: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Return)
        .collect();

    let [returned] = returned.as_slice() else {
        return Err("Nonzero graph requires its sole complete return".into());
    };

    operands(pair, returned, &[outer])?;

    super::callbacks::order::before(
        pair,
        &super::callbacks::order::completion(pair, outer)?,
        returned,
    )
}

fn merge<'a>(
    pair: PairView<'a>,
    selector: &PdgNode,
    yes: &PdgNode,
    no: &PdgNode,
) -> Result<&'a PdgNode, String> {
    let records = pair.pdg.extensions[SITES]
        .as_array()
        .ok_or("Missing nonzero merge sites")?;

    let selected: Vec<_> = records
        .iter()
        .filter(|site| site["controller_pdg_node_id"] == selector.id)
        .collect();

    let [site] = selected.as_slice() else {
        return Err("Nonzero graph requires one merge per exact tag selector".into());
    };

    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing nonzero alternatives")?;

    if alternatives.len() != 2
        || [("true", yes), ("false", no)]
            .iter()
            .any(|(outcome, value)| {
                alternatives
                    .iter()
                    .filter(|alternative| {
                        alternative["outcome"] == *outcome
                            && alternative["value_pdg_node_id"] == value.id
                    })
                    .count()
                    != 1
            })
    {
        return Err("Nonzero merge changes its guarded Some/None values".into());
    }

    let node = pair
        .pdg
        .nodes
        .iter()
        .find(|node| site["pdg_node_id"] == node.id)
        .ok_or("Missing nonzero merge node")?;

    super::callbacks::effects(node, NodeCategory::Merge)?;

    Ok(node)
}

fn origins(pair: PairView<'_>) -> Result<(), String> {
    if pair.pdg.nodes.iter().any(|node| {
        node.annotations.contains_key(crate::data_origins::KEY)
            || node
                .annotations
                .contains_key(super::copy_tagged_values::origins::ROLE_KEY)
    }) {
        return Err("Nonzero composition cannot flatten guarded callee or payload origins".into());
    }

    Ok(())
}

#[derive(serde::Serialize)]
struct Composition<'a> {
    identity: &'static str,
    parser: &'a crate::frontend::HirRef,
    callback: &'a crate::frontend::HirRef,
    member: &'a serde_json::Value,
    callee: serde_json::Value,
    input: &'a crate::scalar_product_instances::Observation,
    output: &'a crate::scalar_product_instances::Observation,
    semantics: &'static str,
    origins: &'static str,
    conditions: &'static str,
}
