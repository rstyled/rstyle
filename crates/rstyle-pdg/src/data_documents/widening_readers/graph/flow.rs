//! Three real tag selections retain all branch completions and ordered captures.
use super::*;
use crate::{CfgEdgeKind, slice_patterns::graph::flow::Flow};

pub(super) fn verify(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let flow = Flow::new(pair)?;

    boundaries(pair, &flow, ops)?;
    prefix(&flow, ops)?;
    mapping(&flow, ops)?;
    failure(&flow, ops)?;
    guards(pair, ops)?;
    merges(pair, &flow, ops)
}

fn boundaries(pair: PairView<'_>, flow: &Flow<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let exits: Vec<_> = pair
        .cfg
        .nodes
        .iter()
        .filter(|node| node.kind == crate::CfgNodeKind::Exit)
        .collect();
    let [exit] = exits.as_slice() else {
        return Err("Widening reader requires one declared normal exit".into());
    };
    if exit.exit_kind != Some(crate::CfgExitKind::Normal)
        || pair.cfg.exit_node_ids.as_slice() != [exit.id.clone()]
    {
        return Err("Widening reader changes its complete normal exit inventory".into());
    }

    for (source, kind, target) in [
        (
            pair.cfg.entry_node_id.as_str(),
            CfgEdgeKind::Normal,
            flow.operation(ops.get("slice")?)?,
        ),
        (
            flow.operation(ops.get("returned")?)?,
            CfgEdgeKind::Return,
            exit.id.as_str(),
        ),
    ] {
        let actual: Vec<_> = pair
            .cfg
            .edges
            .iter()
            .filter(|edge| edge.source == source)
            .map(|edge| (edge.kind, edge.target.as_str()))
            .collect();
        if !crate::validate::unique_inventory(&actual, &[(kind, target)]) {
            return Err("Widening reader entry or return changes its required completion".into());
        }
    }
    Ok(())
}

fn chain(flow: &Flow<'_>, ops: &Operations<'_>, names: &[&str]) -> Result<(), String> {
    for adjacent in names.windows(2) {
        flow.outgoing(
            ops.get(adjacent[0])?,
            &[(CfgEdgeKind::Normal, ops.get(adjacent[1])?)],
        )?;
    }
    Ok(())
}

fn prefix(flow: &Flow<'_>, ops: &Operations<'_>) -> Result<(), String> {
    chain(
        flow,
        ops,
        &[
            "slice",
            "message",
            "reader",
            "outer-environment",
            "outer-is-ok",
        ],
    )?;
    flow.outgoing(
        ops.get("outer-is-ok")?,
        &[
            (CfgEdgeKind::True, ops.get("reader-ok-payload")?),
            (CfgEdgeKind::False, ops.get("reader-error-forward")?),
        ],
    )?;
    for (from, to) in [
        (ops.get("reader-ok-payload")?, ops.get("decoded-payload")?),
        (ops.get("decoded-payload")?, ops.bindings[0]),
        (ops.bindings[0], ops.get("count-payload")?),
        (ops.get("count-payload")?, ops.bindings[1]),
        (ops.bindings[1], ops.get("decoded-read")?),
    ] {
        flow.outgoing(from, &[(CfgEdgeKind::Normal, to)])?;
    }
    chain(
        flow,
        ops,
        &["decoded-read", "conversion", "map-environment", "map-is-ok"],
    )
}

fn mapping(flow: &Flow<'_>, ops: &Operations<'_>) -> Result<(), String> {
    flow.outgoing(
        ops.get("map-is-ok")?,
        &[
            (CfgEdgeKind::True, ops.get("conversion-ok-payload")?),
            (CfgEdgeKind::False, ops.get("map-error-forward")?),
        ],
    )?;

    flow.outgoing(
        ops.get("conversion-ok-payload")?,
        &[(CfgEdgeKind::Normal, ops.bindings[2])],
    )?;
    flow.outgoing(
        ops.bindings[2],
        &[(CfgEdgeKind::Normal, ops.get("converted-read")?)],
    )?;
    chain(
        flow,
        ops,
        &["converted-read", "count-capture", "success-tuple", "map-ok"],
    )?;
    for name in ["map-ok", "map-error-forward"] {
        flow.outgoing(
            ops.get(name)?,
            &[(CfgEdgeKind::Normal, ops.get("failure-environment")?)],
        )?;
    }
    Ok(())
}

fn failure(flow: &Flow<'_>, ops: &Operations<'_>) -> Result<(), String> {
    chain(flow, ops, &["failure-environment", "failure-is-err"])?;
    flow.outgoing(
        ops.get("failure-is-err")?,
        &[
            (CfgEdgeKind::True, ops.get("conversion-error-discard")?),
            (CfgEdgeKind::False, ops.get("final-success-payload")?),
        ],
    )?;

    chain(
        flow,
        ops,
        &[
            "conversion-error-discard",
            "message-capture",
            "constructor",
            "final-error",
        ],
    )?;
    chain(flow, ops, &["final-success-payload", "final-ok"])?;
    for name in ["reader-error-forward", "final-error", "final-ok"] {
        flow.outgoing(
            ops.get(name)?,
            &[(CfgEdgeKind::Normal, ops.get("returned")?)],
        )?;
    }
    Ok(())
}

fn guards(pair: PairView<'_>, ops: &Operations<'_>) -> Result<(), String> {
    let guards = crate::data_documents::conditional_payloads::Guards::new(pair)?;
    outer_guards(&guards, ops)?;

    branch_guards(
        &guards,
        ops,
        "map-is-ok",
        [
            &[
                "conversion-ok-payload",
                "converted-read",
                "count-capture",
                "success-tuple",
                "map-ok",
            ],
            &["map-error-forward"],
        ],
    )?;
    guards.guarded(ops.get("map-is-ok")?, ops.bindings[2], CfgEdgeKind::True)?;
    branch_guards(
        &guards,
        ops,
        "failure-is-err",
        [
            &[
                "conversion-error-discard",
                "message-capture",
                "constructor",
                "final-error",
            ],
            &["final-success-payload", "final-ok"],
        ],
    )
}

fn outer_guards(
    guards: &crate::data_documents::conditional_payloads::Guards<'_>,
    ops: &Operations<'_>,
) -> Result<(), String> {
    let outer = ops.get("outer-is-ok")?;
    for (name, node) in &ops.nodes {
        if ![
            "slice",
            "message",
            "reader",
            "outer-environment",
            "outer-is-ok",
            "reader-error-forward",
            "returned",
            "map-merge",
            "failure-merge",
            "outer-merge",
        ]
        .contains(name)
        {
            guards.guarded(outer, node, CfgEdgeKind::True)?;
        }
    }
    for node in ops.bindings {
        guards.guarded(outer, node, CfgEdgeKind::True)?;
    }
    guards.guarded(outer, ops.get("reader-error-forward")?, CfgEdgeKind::False)
}

fn branch_guards(
    guards: &crate::data_documents::conditional_payloads::Guards<'_>,
    ops: &Operations<'_>,
    selector: &str,
    branches: [&[&str]; 2],
) -> Result<(), String> {
    let [yes, no] = branches;
    for (outcome, names) in [(CfgEdgeKind::True, yes), (CfgEdgeKind::False, no)] {
        for name in names {
            guards.guarded(ops.get(selector)?, ops.get(name)?, outcome)?;
        }
    }
    Ok(())
}

fn merges(pair: PairView<'_>, flow: &Flow<'_>, ops: &Operations<'_>) -> Result<(), String> {
    merge(
        pair,
        flow,
        ops,
        Merge {
            role: ["map-merge", "map-is-ok", "failure-environment"],
            values: ["map-ok", "map-error-forward"],
            completions: [&["map-ok"], &["map-error-forward"]],
        },
    )?;
    merge(
        pair,
        flow,
        ops,
        Merge {
            role: ["failure-merge", "failure-is-err", "returned"],
            values: ["final-error", "final-ok"],
            completions: [&["final-error"], &["final-ok"]],
        },
    )?;
    merge(
        pair,
        flow,
        ops,
        Merge {
            role: ["outer-merge", "outer-is-ok", "returned"],
            values: ["failure-merge", "reader-error-forward"],
            completions: [&["final-error", "final-ok"], &["reader-error-forward"]],
        },
    )
}

struct Merge<'a> {
    role: [&'a str; 3],
    values: [&'a str; 2],
    completions: [&'a [&'a str]; 2],
}

fn merge(
    pair: PairView<'_>,
    flow: &Flow<'_>,
    ops: &Operations<'_>,
    expected: Merge<'_>,
) -> Result<(), String> {
    let Merge {
        role,
        values,
        completions,
    } = expected;
    let node = ops.get(role[0])?;
    let site = crate::data_documents::callbacks::order::site(pair, &node.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing widening result alternatives")?;
    if alternatives.len() != 2
        || site["controller_pdg_node_id"] != ops.get(role[1])?.id
        || site["cfg_successor_node_id"] != flow.operation(ops.get(role[2])?)?
    {
        return Err("Widening result changes its selector or complete continuation".into());
    }

    alternative_edges(pair, node, [ops.get(values[0])?, ops.get(values[1])?])?;
    for (index, outcome) in ["true", "false"].into_iter().enumerate() {
        let value = ops.get(values[index])?;
        let selected = alternatives
            .iter()
            .filter(|alternative| alternative["outcome"] == outcome)
            .collect::<Vec<_>>();
        let [alternative] = selected.as_slice() else {
            return Err("Widening result changes its exact branch outcomes".into());
        };
        selected_completion(
            flow,
            ops,
            alternative,
            Selected {
                value,
                names: completions[index],
            },
        )?;
    }
    Ok(())
}

fn alternative_edges(
    pair: PairView<'_>,
    node: &PdgNode,
    values: [&PdgNode; 2],
) -> Result<(), String> {
    crate::data_documents::checked_roles::alternative_edges(
        pair,
        node,
        [(values[0], "true"), (values[1], "false")],
    )?;
    for value in values {
        crate::data_documents::value_consumers::verify(pair, value, &[node], "value_alternative")?;
    }
    Ok(())
}

struct Selected<'a> {
    value: &'a PdgNode,
    names: &'a [&'a str],
}

fn selected_completion(
    flow: &Flow<'_>,
    ops: &Operations<'_>,
    alternative: &Value,
    selected: Selected<'_>,
) -> Result<(), String> {
    let Selected { value, names } = selected;
    let expected = names
        .iter()
        .map(|name| flow.operation(ops.get(name)?).map(str::to_owned))
        .collect::<Result<Vec<_>, String>>()?;
    let actual = alternative["completion_cfg_node_ids"]
        .as_array()
        .ok_or("Missing widening completion frontier")?;
    let actual: std::collections::BTreeSet<_> = actual.iter().filter_map(Value::as_str).collect();

    if alternative["value_pdg_node_id"] != value.id
        || actual.len() != expected.len()
        || expected.iter().any(|node| !actual.contains(node.as_str()))
    {
        return Err("Widening result loses its exact selected value/completion frontier".into());
    }
    Ok(())
}
