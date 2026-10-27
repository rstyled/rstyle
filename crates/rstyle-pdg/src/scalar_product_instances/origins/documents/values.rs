use super::*;

pub(super) fn direct(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &graph::Replay<'_>,
    contract: &mut Contract,
) -> Result<(), String> {
    input(pair, checked, replay, contract)?;
    elements(pair, checked, replay, contract)?;
    suffix(pair, checked, replay, contract)
}

fn input(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &graph::Replay<'_>,
    contract: &mut Contract,
) -> Result<(), String> {
    let formal = serialized_operation(pair, &checked.prefix().formal().span, |node| {
        node.category == NodeCategory::FormalInput
    })?;
    let ty = source_type(checked.prefix().formal())?;
    formal_identity(formal, checked)?;
    let input = Value {
        ty,
        footprint: Footprint::Direct {
            origins: vec![source::input()],
            guards: vec![Vec::new()],
        },
    };

    contract.insert(formal, input.clone())?;
    contract.insert(replay.saved, input)
}

fn suffix(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &graph::Replay<'_>,
    contract: &mut Contract,
) -> Result<(), String> {
    let suffix = Value {
        ty: source_type(checked.prefix().remainder())?,
        footprint: Footprint::Direct {
            origins: vec![source::suffix(checked)?],
            guards: graph::guards(replay, true),
        },
    };

    contract.insert(replay.suffix.0, suffix.clone())?;
    contract.insert(value_operation(pair, checked.prefix().remainder())?, suffix)
}

fn formal_identity(formal: &PdgNode, checked: &Checked<'_>) -> Result<(), String> {
    let symbol = checked
        .prefix()
        .formal()
        .binding
        .as_ref()
        .ok_or("Digit prefix formal lacks its source binding")?;
    if formal.attributes.position != Some(0)
        || formal.attributes.symbol_ids != [symbol.clone()]
        || !formal.attributes.reads.is_empty()
        || !formal.attributes.writes.is_empty()
    {
        return Err("Digit-prefix input changes its exact original formal identity".into());
    }

    Ok(())
}

fn elements(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &graph::Replay<'_>,
    contract: &mut Contract,
) -> Result<(), String> {
    for ((element, region), digit) in replay.elements.iter().zip(checked.prefix().digits()) {
        let value = Value {
            ty: source_type(digit.pointer())?,
            footprint: Footprint::Direct {
                origins: vec![region.checked_origin(checked.prefix())?],
                guards: graph::guards(replay, true),
            },
        };
        contract.insert(element, value.clone())?;
        contract.insert(value_operation(pair, digit.pointer())?, value)?;
    }

    Ok(())
}

pub(super) fn constructed(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &graph::Replay<'_>,
    contract: &mut Contract,
) -> Result<(), String> {
    let product = value_operation(pair, checked.constructor().node())?;
    constructor(product, "data.scalar-product")?;
    verify_saved_operands(
        pair,
        product,
        &[
            value_operation(pair, checked.prefix().remainder())?,
            value_operation(pair, checked.scalar())?,
        ],
    )?;
    contract.insert(
        product,
        Value {
            ty: source_type(checked.constructor().node())?,
            footprint: Footprint::Product {
                components: components(checked, false)?,
                guards: graph::guards(replay, true),
            },
        },
    )?;
    let some = value_operation(pair, checked.some())?;
    constructor(some, "data.digit-some")?;
    verify_saved_operands(pair, some, &[product])?;
    constructor(replay.none, "data.digit-none")?;
    verify_saved_operands(pair, replay.none, &[])?;

    tagged(pair, checked, replay, contract)
}

fn tagged(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &graph::Replay<'_>,
    contract: &mut Contract,
) -> Result<(), String> {
    let some = Case {
        tag: "Some".into(),
        guards: graph::guards(replay, true),
        components: components(checked, true)?,
    };
    let none = Case {
        tag: "None".into(),
        guards: graph::guards(replay, false),
        components: Vec::new(),
    };
    let ty = source_type(checked.some())?;
    let value = |cases| Value {
        ty: ty.clone(),
        footprint: Footprint::Tagged { cases },
    };
    contract.insert(
        value_operation(pair, checked.some())?,
        value(vec![some.clone()]),
    )?;
    contract.insert(replay.none, value(vec![none.clone()]))?;

    output(pair, checked, value(vec![some, none]), contract)
}

fn output(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    output: Value,
    contract: &mut Contract,
) -> Result<(), String> {
    let merge = value_operation(pair, checked.prefix().matched())?;
    contract.insert(merge, output.clone())?;
    let returns: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == NodeCategory::Return)
        .collect();
    let [returned] = returns.as_slice() else {
        return Err("Digit product requires its one original implicit return".into());
    };

    return_identity(returned, checked)?;
    verify_saved_operands(pair, returned, &[merge])?;
    contract.insert(returned, output)
}

fn return_identity(node: &PdgNode, checked: &Checked<'_>) -> Result<(), String> {
    let matched = checked.prefix().matched();
    if node
        .attributes
        .extensions
        .get("compat.rust.hir-r0:return-form")
        .and_then(serde_json::Value::as_str)
        != Some("implicit")
        || node.attributes.exit_kind != Some(crate::ExitKind::Normal)
        || node.source_span.as_ref().is_none_or(|span| {
            span.start != u64::from(matched.span.start) || span.end != u64::from(matched.span.end)
        })
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
    {
        return Err("Digit product result changes its original normal implicit return".into());
    }

    Ok(())
}

fn constructor(node: &PdgNode, expected: &str) -> Result<(), String> {
    if node.category != NodeCategory::Computation
        || node.attributes.construct.as_deref() != Some(expected)
        || !node.attributes.reads.is_empty()
        || !node.attributes.writes.is_empty()
        || !node.attributes.defines.is_empty()
        || !node.attributes.symbol_ids.is_empty()
        || node.attributes.callee.is_some()
        || node.attributes.call_form.is_some()
        || node.attributes.operator.is_some()
        || node.attributes.literal.is_some()
        || node.attributes.write_form.is_some()
        || node.attributes.position.is_some()
        || node.attributes.exit_kind.is_some()
    {
        return Err(
            "Digit product constructor invents a call, memory access or stored value".into(),
        );
    }

    Ok(())
}
