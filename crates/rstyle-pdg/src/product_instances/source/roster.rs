//! A bijection over every original typed node, including generic-dependent reads.
use super::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn verify(checked: &CheckedSource<'_>, observation: &Value) -> Result<()> {
    let nodes = observation["nodes"]
        .as_array()
        .ok_or("Missing product node observation roster")?;

    let mut expected = inventory(checked)?;
    if nodes.len() != 16 || expected.len() != 16 {
        return Err("Product typed-node roster must retain all sixteen original sites".into());
    }
    signature(observation)?;
    conversion(checked, observation)?;
    for value in nodes {
        let hir: crate::HirRef =
            crate::HirRef::deserialize(&value["hir"]).map_err(|e| e.to_string())?;
        let key = (hir.krate, hir.owner, hir.local);
        let node = expected
            .remove(&key)
            .ok_or("Product observation duplicates or invents an original typed site")?;
        let observed_span = crate::ByteSpan::deserialize(&value["span"])
            .map_err(|_| "Product typed observation changes original kind/span")?;
        if value["kind"] != "value"
            || value["original_kind"] != node.kind
            || observed_span != node.span
        {
            return Err("Product typed observation changes original kind/span".into());
        }
        let (original_ty, projected_ty) = role(checked, node, observation, value)?;
        if value["original"] != *original_ty || value["projected"] != *projected_ty {
            return Err(
                "Product typed observation changes its independently specified source role".into(),
            );
        }
    }
    if !expected.is_empty() {
        return Err("Product typed observation omits original sites".into());
    }
    Ok(())
}

fn inventory<'a>(checked: &CheckedSource<'a>) -> Result<BTreeMap<(u32, u32, u32), &'a HirNode>> {
    let mut expected = BTreeMap::new();

    let mut all_sites = BTreeSet::new();
    collect(checked.pattern(), &mut expected, &mut all_sites)?;
    collect(checked.input(), &mut expected, &mut all_sites)?;
    collect(&checked.facts().body, &mut expected, &mut all_sites)?;
    Ok(expected)
}

fn signature(observation: &Value) -> Result<()> {
    let original = &observation["original_signature"];

    let inputs = original["inputs"]
        .as_array()
        .ok_or("Product original input signature missing")?;
    let projected = observation["projected_inputs"]
        .as_array()
        .ok_or("Product projected input signature missing")?;
    if inputs.len() != 2
        || projected.len() != 2
        || inputs[1] != serde_json::json!({"kind":"parameter","index":0,"name":"S"})
        || projected[1] != observation["arguments"]
    {
        return Err("Product original/projected signature roster differs".into());
    }
    Ok(())
}

#[derive(Deserialize)]
struct ConversionSite {
    hir: crate::HirRef,
    span: crate::ByteSpan,
    receiver: crate::HirRef,
}

fn conversion(checked: &CheckedSource<'_>, observation: &Value) -> Result<()> {
    let conversions = observation["conversions"]
        .as_array()
        .ok_or("Product conversion observations missing")?;

    let [conversion] = conversions.as_slice() else {
        return Err("Product conversion roster must be singular".into());
    };
    let site = ConversionSite::deserialize(conversion)
        .map_err(|_| "Product conversion observation changes its exact original source role")?;
    if site.hir != checked.conversion().hir
        || site.span != checked.conversion().span
        || site.receiver != checked.receiver().hir
        || conversion["receiver_type"] != observation["arguments"]
    {
        return Err("Product conversion observation changes its exact original source role".into());
    }
    Ok(())
}

fn collect<'a>(
    node: &'a HirNode,
    output: &mut BTreeMap<(u32, u32, u32), &'a HirNode>,
    all_sites: &mut BTreeSet<(u32, u32, u32)>,
) -> Result<()> {
    if !all_sites.insert((node.hir.krate, node.hir.owner, node.hir.local)) {
        return Err("Product source duplicates original HIR identity".into());
    }
    if node.ty.is_some()
        && output
            .insert((node.hir.krate, node.hir.owner, node.hir.local), node)
            .is_some()
    {
        return Err("Product source duplicates original typed HIR identity".into());
    }
    for child in &node.children {
        collect(child, output, all_sites)?;
    }
    Ok(())
}

fn role<'a>(
    checked: &CheckedSource<'_>,
    node: &HirNode,
    value: &'a Value,
    record: &'a Value,
) -> Result<(&'a Value, &'a Value)> {
    let pattern = node.hir == checked.pattern().hir || node.hir == checked.result_operands()[0].hir;

    let input = node.hir == checked.input().hir || node.hir == checked.receiver().hir;
    let result = node.hir == checked.facts().body.hir || node.hir == checked.result().hir;
    if pattern {
        return Ok((
            &value["original_signature"]["inputs"][0],
            &value["projected_inputs"][0],
        ));
    }
    if input {
        return Ok((
            &value["original_signature"]["inputs"][1],
            &value["arguments"],
        ));
    }
    if result {
        return Ok((
            &value["original_signature"]["output"],
            &value["projected_result"],
        ));
    }
    let conversion = &value["conversions"][0];
    // Non-generic Span sites are bound to the exact original concrete Into result.
    let original_span = &value["owners"][1]["clauses"];
    let clause = original_span
        .as_array()
        .ok_or("Product original trait clauses missing")?
        .iter()
        .find(|clause| clause["trait_definition"]["path"] == "core::convert::Into")
        .ok_or("Product original Into clause missing")?;
    if node.hir == checked.assertion().hir
        || node.hir == checked.condition().hir
        || checked
            .endpoints()
            .iter()
            .any(|field| field.hir == node.hir)
    {
        // Scalar and unit records are checked separately against fixed structural types.
        return scalar(node, record);
    }
    Ok((&clause["original_arguments"][1], &conversion["result_type"]))
}

fn scalar<'a>(node: &HirNode, record: &'a Value) -> Result<(&'a Value, &'a Value)> {
    let expected = scalar_type(node)?;

    let projected = record["projected"]
        .as_array()
        .ok_or("Product scalar projection absent")?;
    if record["original"] != expected
        || projected.len() != 1
        || projected[0]["kind"] != "type"
        || projected[0]["display_only"] != true
        || projected[0]["identity_status"] != "known"
        || projected[0]["identity"] != expected
    {
        return Err("Product scalar observation differs from its builtin source role".into());
    }
    Ok((&record["original"], &record["projected"]))
}

fn scalar_type(node: &HirNode) -> Result<Value> {
    match node.kind.as_str() {
        "expr_assert" => Ok(serde_json::json!({"kind":"tuple","elements":[]})),
        "expr_binary" => Ok(builtin("bool")),
        "expr_field" => Ok(builtin("usize")),
        _ => Err("Product scalar role is unsupported".into()),
    }
}

fn builtin(name: &str) -> Value {
    serde_json::json!({"kind":"builtin","name":name})
}
