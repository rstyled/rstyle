//! Conservative panic assessment under captured integer widths/settings.
//! Concrete snapshot propagation discharges fixture-local arithmetic proofs;
//! an unavailable value or an unproved bound retains an explicit limitation.

use super::{Analysis, Graph, Operation};
use crate::NodeCategory;
use std::collections::BTreeMap;

#[derive(serde::Serialize)]
pub struct Assessment {
    pub limitations: Vec<String>,
    pub checked_operations: Vec<Proof>,
}

#[derive(serde::Serialize)]
pub struct Proof {
    operation_id: String,
    resolved_type: String,
    operand_constants: Vec<Option<String>>,
    omitted_panic_impossible: bool,
    method: &'static str,
}

pub fn assess(
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &Graph,
    analysis: &Analysis,
) -> Assessment {
    let mut values = BTreeMap::new();
    let mut assessment = Assessment {
        limitations: Vec::new(),
        checked_operations: Vec::new(),
    };
    for node in graph
        .nodes
        .iter()
        .filter(|n| analysis.reachable.contains(&n.id) || n.category == NodeCategory::FormalInput)
    {
        let operands: Vec<_> = node
            .operands
            .iter()
            .map(|id| values.get(id).copied().flatten())
            .collect();
        let value =
            constant(node, graph, analysis, &values).filter(|value| fits(facts, &node.ty, *value));
        let safe = proven_safe(facts, graph, node, &operands);
        if relevant(node) && !safe {
            assessment.limitations.push(limitation(facts, node));
        }
        if relevant(node) {
            assessment.checked_operations.push(Proof {
                operation_id: node.id.clone(),
                resolved_type: node.ty.clone(),
                operand_constants: operands
                    .iter()
                    .map(|value| value.map(|v| v.to_string()))
                    .collect(),
                omitted_panic_impossible: safe,
                method: proof_method(node),
            });
        }
        values.insert(node.id.clone(), value);
    }

    assessment
}

fn proof_method(node: &Operation) -> &'static str {
    if matches!(
        node.construct.as_deref(),
        Some("data.bytes.unwrap" | "data.bytes.unwrap-borrow")
    ) {
        return "concrete Result unwrap requires Ok; exceptional transfer and cleanup omitted";
    }
    if node.operator.as_deref() == Some("assert") {
        "resolved standard assertion; only a directly known true boolean discharges omitted panic flow"
    } else {
        "resolved-width checked constant evaluation, unique reaching-definition snapshots, captured overflow settings, and source literal-negation exception"
    }
}

fn limitation(facts: &dyn crate::unit_store::EvidenceFacts, node: &Operation) -> String {
    if node.operator.as_deref() == Some("assert") {
        return format!(
            "Assertion at bytes {}..{} may panic; successful normal continuation is modeled, while exceptional transfer and unwinding/cleanup are omitted.",
            node.span.start, node.span.end
        );
    }
    format!(
        "{} at bytes {}..{} may panic under captured {}-bit target and overflow-checks={:?}; exceptional transfer and unwinding/cleanup are omitted.",
        node.operator.as_deref().unwrap_or("integer operation"),
        node.span.start,
        node.span.end,
        facts.build.pointer_width.value.unwrap_or(0),
        facts.build.overflow_checks.value
    )
}

fn relevant(node: &Operation) -> bool {
    matches!(
        node.construct.as_deref(),
        Some("data.bytes.unwrap" | "data.bytes.unwrap-borrow")
    ) || node.category == NodeCategory::Computation
        && node.ty != "bool"
        && matches!(
            node.operator.as_deref(),
            Some("+" | "-" | "*" | "/" | "%" | "<<" | ">>" | "assert")
        )
}

fn constant(
    node: &Operation,
    graph: &Graph,
    analysis: &Analysis,
    values: &BTreeMap<String, Option<i128>>,
) -> Option<i128> {
    if node.category == NodeCategory::Read {
        let definition_set = analysis
            .definition_in
            .get(&node.id)?
            .get(node.symbol.as_ref()?)?;
        let constants: Option<Vec<_>> = definition_set
            .iter()
            .map(|id| {
                let definition = graph
                    .nodes
                    .iter()
                    .find(|n| n.definition.as_ref() == Some(id))?;
                let producer = definition.operands.first()?;
                values.get(producer).copied().flatten()
            })
            .collect();
        let constants = constants?;
        let first = *constants.first()?;
        return constants
            .iter()
            .all(|value| *value == first)
            .then_some(first);
    }
    if let Some(literal) = &node.literal {
        return integer_literal(literal);
    }
    let known: Option<Vec<_>> = node
        .operands
        .iter()
        .map(|id| values.get(id).copied().flatten())
        .collect();
    let known = known?;

    evaluate(node.operator.as_deref()?, &known)
}

fn integer_literal(literal: &str) -> Option<i128> {
    let digits = literal.replace('_', "");
    // Decimal suffixes start with `i`/`u`; bases have their own digit alphabet.
    let end = digits.rfind(['i', 'u']).unwrap_or(digits.len());
    let value = &digits[..end];

    if let Some(value) = value.strip_prefix("0x") {
        i128::from_str_radix(value, 16).ok()
    } else if let Some(value) = value.strip_prefix("0o") {
        i128::from_str_radix(value, 8).ok()
    } else if let Some(value) = value.strip_prefix("0b") {
        i128::from_str_radix(value, 2).ok()
    } else {
        value.parse().ok()
    }
}

fn evaluate(op: &str, values: &[i128]) -> Option<i128> {
    match (op, values) {
        ("-", [value]) => value.checked_neg(),
        ("!", [value]) => Some(!value),
        ("+", [a, b]) => a.checked_add(*b),
        ("-", [a, b]) => a.checked_sub(*b),
        ("*", [a, b]) => a.checked_mul(*b),
        ("/", [a, b]) => a.checked_div(*b),
        ("%", [a, b]) => a.checked_rem(*b),
        ("<<", [a, b]) => a.checked_shl(u32::try_from(*b).ok()?),
        (">>", [a, b]) => a.checked_shr(u32::try_from(*b).ok()?),
        ("&", [a, b]) => Some(a & b),
        ("|", [a, b]) => Some(a | b),
        ("^", [a, b]) => Some(a ^ b),
        _ => None,
    }
}

fn width(facts: &dyn crate::unit_store::EvidenceFacts, ty: &str) -> Option<u32> {
    if matches!(ty, "isize" | "usize") {
        return facts
            .build
            .pointer_width
            .value
            .and_then(|v| u32::try_from(v).ok());
    }

    ty.get(1..)?.parse().ok()
}

fn fits(facts: &dyn crate::unit_store::EvidenceFacts, ty: &str, value: i128) -> bool {
    let Some(bits) = width(facts, ty) else {
        return false;
    };
    if ty.starts_with('i') {
        bits == 128 || (value >= -(1i128 << (bits - 1)) && value < (1i128 << (bits - 1)))
    } else {
        value >= 0 && (bits >= 127 || value < (1i128 << bits))
    }
}

fn proven_safe(
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &Graph,
    node: &Operation,
    operands: &[Option<i128>],
) -> bool {
    let Some(op) = node.operator.as_deref() else {
        return false;
    };
    if op == "assert" {
        return graph
            .nodes
            .iter()
            .find(|candidate| node.operands.first() == Some(&candidate.id))
            .is_some_and(|condition| {
                condition.literal.as_deref() == Some("true") && condition.ty == "bool"
            });
    }
    if matches!(op, "+" | "-" | "*") && facts.build.overflow_checks.value == Some(false) {
        return true;
    }
    if matches!(op, "<<" | ">>") {
        if facts.build.overflow_checks.value == Some(false) {
            return true;
        }
        return operands
            .get(1)
            .copied()
            .flatten()
            .zip(width(facts, &node.ty))
            .is_some_and(|(shift, bits)| shift >= 0 && shift < i128::from(bits));
    }
    if op == "-" && node.operands.len() == 1 && literal_negation_safe(facts, graph, node) {
        return true;
    }
    if matches!(op, "/" | "%") {
        let denominator = operands.get(1).copied().flatten();
        if denominator.is_some_and(|d| d != 0 && (d != -1 || node.ty.starts_with('u'))) {
            return true;
        }
    }
    let known: Option<Vec<_>> = operands.iter().copied().collect();

    known
        .and_then(|values| evaluate(op, &values))
        .is_some_and(|result| fits(facts, &node.ty, result))
}

fn literal_negation_safe(
    facts: &dyn crate::unit_store::EvidenceFacts,
    graph: &Graph,
    node: &Operation,
) -> bool {
    let literal = graph
        .nodes
        .iter()
        .find(|n| node.operands.first() == Some(&n.id))
        .and_then(|n| n.literal.as_deref());
    literal
        .and_then(integer_literal)
        .and_then(i128::checked_neg)
        .is_some_and(|value| fits(facts, &node.ty, value))
}
