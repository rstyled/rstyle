//! Ordered native-tree differences, with no equivalence or rewrite authority.
//!
//! This closed contract covers a single scalar expression, optionally wrapped
//! in tail-only blocks. It does not align fragments or infer a WL bijection.

mod model;
pub(crate) use model::*;

use super::super::Function;
use rstyle_ccgraph::ImportedFunction;
use rstyle_pdg::{HirFnFacts, HirNode, ParamFact};
use std::collections::BTreeSet;

/// Borrowed endpoint bound to the exact producer-verified admitted graph.
pub(crate) struct Endpoint<'a, 'graph> {
    admitted: &'graph ImportedFunction,
    facts: &'a HirFnFacts,
    validation: Result<(), Gap>,
    native_width_involved: bool,
}

impl<'a, 'graph> Endpoint<'a, 'graph> {
    pub(super) fn checked(
        admitted: &'graph ImportedFunction,
        record: &'a Function,
    ) -> Result<Self, &'static str> {
        // Only the owning analysis module can join the immutable handoff rows.
        // consume_function establishes producer association before retaining them.
        if record.outcome != "admitted" || admitted.key != record.key.to_string() {
            return Err("native evidence differs from admitted endpoint");
        }
        Ok(Self::from_native(admitted, &record.facts))
    }

    fn from_native(admitted: &'graph ImportedFunction, facts: &'a HirFnFacts) -> Self {
        let validation = validate(facts);
        // Scan only the successfully bounded roster, once per retained endpoint.
        let native_width_involved =
            admitted.native_width_involved || validation.is_ok() && uses_native_width(facts);
        Self {
            admitted,
            facts,
            validation,
            native_width_involved,
        }
    }
}

pub(crate) fn explain<'a>(left: &Endpoint<'a, '_>, right: &Endpoint<'a, '_>) -> Assessment<'a> {
    if left
        .admitted
        .extraction_context
        .as_deref()
        .is_none_or(str::is_empty)
        || left.admitted.extraction_context != right.admitted.extraction_context
        || left.admitted.matching_profile != right.admitted.matching_profile
    {
        return Assessment::unavailable(
            Coverage::Unsupported,
            "missing or incompatible extraction context",
        );
    }
    for endpoint in [left, right] {
        if let Err((coverage, reason)) = endpoint.validation {
            return Assessment::unavailable(coverage, reason);
        }
    }
    if (left.native_width_involved || right.native_width_involved)
        && (left.admitted.target_pointer_width.is_none()
            || left.admitted.target_pointer_width != right.admitted.target_pointer_width)
    {
        return Assessment::unavailable(
            Coverage::Unsupported,
            "native-width scalar evidence requires known equal target widths",
        );
    }
    align_complete(left.facts, right.facts)
}

fn uses_native_width(facts: &HirFnFacts) -> bool {
    facts
        .signature
        .params
        .iter()
        .any(|parameter| matches!(parameter.ty.normalized.as_deref(), Some("usize" | "isize")))
        || native_tree(&facts.body)
}

// Called only after closed-tree validation has bounded the full native roster.
fn native_tree(node: &HirNode) -> bool {
    matches!(scalar_type(node), Some("usize" | "isize")) || node.children.iter().any(native_tree)
}

fn align_complete<'a>(left: &'a HirFnFacts, right: &'a HirFnFacts) -> Assessment<'a> {
    if left.signature.params.len() != right.signature.params.len()
        || left.signature.return_ty.normalized != right.signature.return_ty.normalized
        || left
            .signature
            .params
            .iter()
            .zip(&right.signature.params)
            .any(|(a, b)| a.ty.normalized != b.ty.normalized)
    {
        return Assessment::unavailable(
            Coverage::Unsupported,
            "different scalar interface rosters",
        );
    }
    let mut result = Assessment {
        coverage: Coverage::Complete,
        correspondence: Vec::new(),
        holes: Vec::new(),
        reason: None,
    };
    align(
        (&left.body, &right.body),
        (left, right),
        Role::Root,
        &mut result,
    );
    result
}

type Gap = (Coverage, &'static str);

fn validate(facts: &HirFnFacts) -> Result<(), Gap> {
    if facts.signature.params.len() > 64 {
        return Err((
            Coverage::BudgetExhausted,
            "formal parameter budget exhausted",
        ));
    }
    let mut identities = formal_identities(&facts.signature.params)?;
    if !facts
        .signature
        .params
        .iter()
        .enumerate()
        .all(|(index, param)| {
            usize::try_from(param.position) == Ok(index)
                && param.pat.kind == "pat_binding"
                && scalar_type(&param.pat) == param.ty.normalized.as_deref()
                && scalar_type(&param.pat).is_some()
                && param.pat.binding.as_deref() == Some(param.symbol.as_str())
        })
    {
        return Err((
            Coverage::Unsupported,
            "formal parameter roster is incomplete or unsupported",
        ));
    }
    if scalar_type(&facts.body) != facts.signature.return_ty.normalized.as_deref() {
        return Err((
            Coverage::Unsupported,
            "native return type differs from signature",
        ));
    }

    validate_node(&facts.body, facts, &mut identities, 0)
}

fn formal_identities(parameters: &[ParamFact]) -> Result<BTreeSet<(u32, u32, u32)>, Gap> {
    let identities: BTreeSet<_> = parameters
        .iter()
        .map(|parameter| {
            let hir = &parameter.pat.hir;
            (hir.krate, hir.owner, hir.local)
        })
        .collect();
    let positions: BTreeSet<_> = parameters
        .iter()
        .map(|parameter| parameter.position)
        .collect();
    let symbols: BTreeSet<_> = parameters
        .iter()
        .map(|parameter| &parameter.symbol)
        .collect();

    if [identities.len(), positions.len(), symbols.len()]
        .iter()
        .any(|length| *length != parameters.len())
    {
        return Err((
            Coverage::Ambiguous,
            "duplicate formal parameter identity or role",
        ));
    }
    Ok(identities)
}

fn validate_node(
    node: &HirNode,
    facts: &HirFnFacts,
    identities: &mut BTreeSet<(u32, u32, u32)>,
    depth: usize,
) -> Result<(), Gap> {
    if depth > 64 || identities.len() >= 512 {
        return Err((Coverage::BudgetExhausted, "native tree budget exhausted"));
    }
    if !identities.insert((node.hir.krate, node.hir.owner, node.hir.local)) {
        return Err((Coverage::Ambiguous, "duplicate native node identity"));
    }
    if node.expansion.is_some() || node.is_unsafe_block || scalar_type(node).is_none() {
        return Err((Coverage::Unsupported, "opaque or non-scalar native node"));
    }

    let supported = match node.kind.as_str() {
        "expr_block" => node.children.len() == 1,
        "expr_binary" | "expr_unary" => valid_dispatch(node),
        "expr_lit" => node.children.is_empty() && node.lit.is_some(),
        "expr_path" => {
            node.children.is_empty()
                && formal(node, &facts.signature.params).is_some_and(|parameter| {
                    parameter.ty.normalized.as_deref() == scalar_type(node)
                })
        }
        _ => false,
    };
    if !supported {
        return Err((
            Coverage::Unsupported,
            "native operation or formal identity outside closed scalar contract",
        ));
    }
    for child in &node.children {
        validate_node(child, facts, identities, depth + 1)?;
    }

    Ok(())
}

fn scalar_type(node: &HirNode) -> Option<&str> {
    let ty = node.ty.as_ref()?.normalized.as_deref()?;
    matches!(
        ty,
        "bool"
            | "char"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
    )
    .then_some(ty)
}

fn valid_dispatch(node: &HirNode) -> bool {
    let Some(dispatch) = &node.dispatch else {
        return false;
    };
    let arity = if node.kind == "expr_binary" { 2 } else { 1 };
    dispatch.kind == "builtin"
        && node.children.len() == arity
        && dispatch.operand_tys.len() == arity
        && dispatch.result_ty == scalar_type(node).unwrap_or_default()
        && node
            .children
            .iter()
            .zip(&dispatch.operand_tys)
            .all(|(child, ty)| scalar_type(child) == Some(ty.as_str()))
        && if arity == 2 {
            matches!(
                dispatch.op.as_str(),
                "+" | "-"
                    | "*"
                    | "/"
                    | "%"
                    | "&"
                    | "|"
                    | "^"
                    | "<<"
                    | ">>"
                    | "=="
                    | "!="
                    | "<"
                    | "<="
                    | ">"
                    | ">="
            )
        } else {
            matches!(dispatch.op.as_str(), "!" | "-")
        }
}

fn formal<'a>(node: &HirNode, params: &'a [ParamFact]) -> Option<&'a ParamFact> {
    let resolution = node.res.as_ref()?;
    if resolution.kind != "local" {
        return None;
    }
    let target = resolution.target.as_ref()?;
    let mut candidates = params.iter().filter(|param| &param.pat.hir == target);
    let result = candidates.next()?;

    candidates.next().is_none().then_some(result)
}

fn align<'a>(
    (left, right): (&'a HirNode, &'a HirNode),
    (left_facts, right_facts): (&HirFnFacts, &HirFnFacts),
    role: Role,
    result: &mut Assessment<'a>,
) {
    if left.kind != right.kind || scalar_type(left) != scalar_type(right) {
        result.holes.push(Hole {
            left,
            right,
            kind: HoleKind::Shape,
            role,
        });
        return;
    }
    align_matching((left, right), (left_facts, right_facts), role, result);
}

fn align_matching<'a>(
    (left, right): (&'a HirNode, &'a HirNode),
    (left_facts, right_facts): (&HirFnFacts, &HirFnFacts),
    role: Role,
    result: &mut Assessment<'a>,
) {
    result.correspondence.push(Correspondence {
        left: &left.hir,
        right: &right.hir,
        role,
    });
    let difference = match left.kind.as_str() {
        "expr_lit" if left.lit != right.lit => Some(HoleKind::Literal),
        "expr_path"
            if formal(left, &left_facts.signature.params).map(|p| p.position)
                != formal(right, &right_facts.signature.params).map(|p| p.position) =>
        {
            Some(HoleKind::FormalRole)
        }
        "expr_binary" | "expr_unary" if left.dispatch != right.dispatch => {
            Some(HoleKind::Operation)
        }
        _ => None,
    };
    if let Some(kind) = difference {
        result.holes.push(Hole {
            left,
            right,
            kind,
            role,
        });
    }

    // Child positions remain semantic operand positions, including subtraction.
    for (index, children) in left.children.iter().zip(&right.children).enumerate() {
        let role = match (left.kind.as_str(), index) {
            ("expr_block", _) => Role::BlockTail,
            ("expr_unary", _) => Role::UnaryOperand,
            ("expr_binary", 0) => Role::BinaryLeft,
            _ => Role::BinaryRight,
        };
        align(children, (left_facts, right_facts), role, result);
    }
}

#[cfg(test)]
mod tests;
