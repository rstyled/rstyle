use super::*;

pub(super) fn collect<'a>(pair: PairView<'a>, checked: &Checked<'_>) -> Result<Replay<'a>, String> {
    inventory(pair, checked.digits().len())?;
    let contract = checked.contract();
    let length = operation(pair, checked.prefix().pattern(), LENGTH, "bool")?;
    metadata(length, &contract.selectors[0].kind)?;
    role(length, "prefix_length")?;
    let digits = digit_operations(pair, checked)?;
    let suffix = operation(
        pair,
        checked.prefix().remainder().rest(),
        SUFFIX,
        crate::data_aggregates::token(checked.prefix().remainder().binding())?,
    )?;
    metadata(suffix, &contract.suffix.region)?;
    role(suffix, "prefix_suffix")?;

    Ok(Replay {
        saved: value_operation(pair, checked.scrutinee())?,
        length,
        selector_reads: digits.selector_reads,
        selectors: digits.selectors,
        elements: digits.elements,
        suffix: (suffix, contract.suffix.region),
        none: value_operation(pair, checked.failure())?,
    })
}

#[derive(Default)]
struct DigitOperations<'a> {
    selector_reads: Vec<(&'a PdgNode, Region)>,
    selectors: Vec<&'a PdgNode>,
    elements: Vec<(&'a PdgNode, Region)>,
}

fn digit_operations<'a>(
    pair: PairView<'a>,
    checked: &Checked<'_>,
) -> Result<DigitOperations<'a>, String> {
    let contract = checked.contract();
    let mut result = DigitOperations::default();
    for (index, digit) in checked.digits().iter().enumerate() {
        let region = Region::Element {
            position: index as u8,
        };
        let read = operation(pair, digit.range(), BYTE, "u8")?;
        metadata(read, &region)?;
        role(read, &format!("prefix_selector_read:{index}"))?;
        result.selector_reads.push((read, region.clone()));
        let selector = operation(pair, digit.range(), DIGIT, "bool")?;
        metadata(selector, &contract.selectors[index + 1].kind)?;
        role(selector, &format!("prefix_digit:{index}"))?;
        result.selectors.push(selector);
        let element = operation(
            pair,
            digit.binding(),
            ELEMENT,
            crate::data_aggregates::token(digit.binding())?,
        )?;
        metadata(element, &region)?;
        role(element, &format!("prefix_element:{index}"))?;
        result.elements.push((element, region));
    }

    Ok(result)
}

fn operation<'a>(
    pair: PairView<'a>,
    source: &HirNode,
    construct: &str,
    ty: &str,
) -> Result<&'a PdgNode, String> {
    let node = typed_operation(pair, source, construct, ty)?;
    let category = match construct {
        LENGTH | DIGIT => NodeCategory::Control,
        BYTE => NodeCategory::Read,
        _ => NodeCategory::Computation,
    };
    if node.category != category
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || node.attributes.operator.is_some()
        || node.attributes.callee.is_some()
        || (construct != BYTE && !node.attributes.reads.is_empty())
    {
        return Err("Prefix operation changes its category or invents effects".into());
    }
    Ok(node)
}

pub(super) fn role(node: &PdgNode, expected: &str) -> Result<(), String> {
    if node
        .attributes
        .extensions
        .get("compat.rust.hir-r0:source-operation")
        .and_then(|value| value.get("role"))
        .and_then(serde_json::Value::as_str)
        != Some(expected)
    {
        return Err("Prefix operation loses its exact semantic role".into());
    }
    Ok(())
}

fn metadata<T: serde::Serialize>(node: &PdgNode, expected: &T) -> Result<(), String> {
    let literal = node
        .attributes
        .literal
        .as_ref()
        .and_then(|value| value.known_value())
        .ok_or("Missing prefix region/selector metadata")?;
    let actual: serde_json::Value =
        serde_json::from_str(literal).map_err(|error| error.to_string())?;
    if actual != serde_json::to_value(expected).map_err(|error| error.to_string())? {
        return Err("Prefix metadata disagrees with independently verified source".into());
    }
    Ok(())
}

fn inventory(pair: PairView<'_>, count: usize) -> Result<(), String> {
    for (construct, expected) in [
        (LENGTH, 1),
        (BYTE, count),
        (DIGIT, count),
        (ELEMENT, count),
        (SUFFIX, 1),
    ] {
        if pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some(construct))
            .count()
            != expected
        {
            return Err("Prefix operation inventory differs from original bounded match".into());
        }
    }
    Ok(())
}

pub(super) fn saved(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    saved: &PdgNode,
) -> Result<(), String> {
    let formal = serialized_operation(pair, &checked.formal().span, |node| {
        node.category == NodeCategory::FormalInput
    })?;
    let incoming: Vec<_> = pair
        .pdg
        .edges
        .iter()
        .filter(|edge| edge.target == saved.id && edge.kind == crate::EdgeFamily::Data)
        .collect();
    if saved.category != NodeCategory::Read
        || incoming.len() != 1
        || incoming[0].source != formal.id
        || incoming[0].subkind != "reaching_definition"
        || saved.attributes.symbol_ids != formal.attributes.symbol_ids
        || !saved.attributes.writes.is_empty()
    {
        return Err(
            "Prefix saved input is not the exact original immutable formal snapshot".into(),
        );
    }
    Ok(())
}

pub(super) fn construct_spans(checked: &Checked<'_>, replay: &Replay<'_>) -> Result<(), String> {
    let nodes = std::iter::once(replay.length)
        .chain(replay.selectors.iter().copied())
        .chain(replay.selector_reads.iter().map(|(node, _)| *node))
        .chain(replay.elements.iter().map(|(node, _)| *node))
        .chain(std::iter::once(replay.suffix.0));
    for node in nodes {
        let span =
            &node.attributes.extensions["compat.rust.hir-r0:source-operation"]["construct_span"];
        if span["start"] != u64::from(checked.matched().span.start)
            || span["end"] != u64::from(checked.matched().span.end)
        {
            return Err("Prefix operation loses its original match construct span".into());
        }
    }
    Ok(())
}
