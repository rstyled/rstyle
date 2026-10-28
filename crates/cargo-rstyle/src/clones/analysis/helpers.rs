//! Advisory parameterized helpers over exact completed native comparisons.
use super::{Coverage, Execution, FactAvailability, Outcome, pair_differences as native};
use rstyle_pdg::{ByteSpan, HirNode, HirRef};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Policy {
    pub minimum_builtin_operations: usize,
    pub max_literal_holes: usize,
    pub scalar_type: &'static str,
}

pub(super) const POLICY: Policy = Policy {
    minimum_builtin_operations: 2,
    max_literal_holes: 1,
    scalar_type: "u32",
};

#[derive(Serialize)]
pub(super) struct Assessment<'a> {
    pub execution: Execution,
    pub outcome: Outcome,
    pub coverage: Coverage,
    pub availability: FactAvailability,
    pub reasons: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<Proposal<'a>>,
}

#[derive(Serialize)]
pub(super) struct Proposal<'a> {
    kind: &'static str,
    member_roots: [&'a HirRef; 2],
    builtin_operations: [usize; 2],
    formal_parameters: Vec<FormalParameter>,
    literal_arguments: Vec<LiteralArgument<'a>>,
    correspondence_count: usize,
    advisory: &'static str,
}

#[derive(Serialize)]
struct FormalParameter {
    position: u32,
    scalar_type: &'static str,
}

#[derive(Serialize)]
struct LiteralArgument<'a> {
    position: usize,
    left: Literal<'a>,
    right: Literal<'a>,
}

#[derive(Serialize)]
struct Literal<'a> {
    hir: &'a HirRef,
    span: ByteSpan,
    scalar_type: &'static str,
    value: &'a str,
}

pub(super) fn evaluate<'a>(comparison: &native::ComparedPair<'a>) -> Assessment<'a> {
    let inputs = match comparison.helper_inputs() {
        Ok(inputs) => inputs,
        Err(gap) => return unavailable(gap),
    };
    if inputs
        .builtin_operations
        .iter()
        .any(|count| *count < POLICY.minimum_builtin_operations)
    {
        return no_proposal(
            "Each member must contain at least two builtin operations; forwarding and one-operation bodies are below the usefulness boundary",
        );
    }
    let native = comparison.assessment();
    if native
        .holes
        .iter()
        .any(|hole| hole.kind != native::HoleKind::Literal)
    {
        return no_proposal(
            "Changed formal roles, operators or subtree shapes are outside this helper policy",
        );
    }
    if native.holes.len() > POLICY.max_literal_holes {
        return no_proposal(
            "More than one independent literal argument is outside this helper policy",
        );
    }
    let Some(literal_arguments) = literal_arguments(&native.holes) else {
        return unavailable(native::HelperUnavailable {
            coverage: native::Coverage::Unsupported,
            reason: "Literal argument evidence is incomplete",
        });
    };

    finding(inputs, native, literal_arguments)
}

fn finding<'a>(
    inputs: native::HelperInputs<'a>,
    native: &native::Assessment<'a>,
    literal_arguments: Vec<LiteralArgument<'a>>,
) -> Assessment<'a> {
    Assessment {
        execution: Execution::Completed,
        outcome: Outcome::Finding,
        coverage: Coverage::Complete,
        availability: FactAvailability::Known,
        reasons: Vec::new(),
        proposal: Some(Proposal {
            kind: "parameterized_helper",
            member_roots: [&inputs.left.body.hir, &inputs.right.body.hir],
            builtin_operations: inputs.builtin_operations,
            formal_parameters: inputs
                .left
                .signature
                .params
                .iter()
                .map(|parameter| FormalParameter {
                    position: parameter.position,
                    scalar_type: POLICY.scalar_type,
                })
                .collect(),
            literal_arguments,
            correspondence_count: native.correspondence.len(),
            advisory: "Consider extracting a helper preserving ordered formals and common operations, with the listed literal arguments. Review integer overflow behavior and call-site evaluation before editing; this is not an automatic safe rewrite.",
        }),
    }
}

fn literal_arguments<'a>(holes: &[native::Hole<'a>]) -> Option<Vec<LiteralArgument<'a>>> {
    match holes {
        [] => Some(Vec::new()),
        [hole] => Some(vec![LiteralArgument {
            position: 0,
            left: literal(hole.left)?,
            right: literal(hole.right)?,
        }]),
        _ => None,
    }
}

fn literal(node: &HirNode) -> Option<Literal<'_>> {
    Some(Literal {
        hir: &node.hir,
        span: node.span,
        scalar_type: POLICY.scalar_type,
        value: node.lit.as_deref()?,
    })
}

fn no_proposal(reason: &'static str) -> Assessment<'static> {
    Assessment {
        execution: Execution::Completed,
        outcome: Outcome::EvaluatedNoFinding,
        coverage: Coverage::Complete,
        availability: FactAvailability::Known,
        reasons: vec![reason],
        proposal: None,
    }
}

fn unavailable(gap: native::HelperUnavailable) -> Assessment<'static> {
    let exhausted = gap.coverage == native::Coverage::BudgetExhausted;
    Assessment {
        execution: if exhausted {
            Execution::BudgetExhausted
        } else {
            Execution::Completed
        },
        outcome: Outcome::NotEvaluated,
        coverage: Coverage::Incomplete,
        availability: if exhausted {
            FactAvailability::Unknown
        } else {
            FactAvailability::Unsupported
        },
        reasons: vec![gap.reason],
        proposal: None,
    }
}

impl Proposal<'_> {
    pub(super) fn advisory(&self) -> &str {
        self.advisory
    }
}
