//! Optional explanations over accepted pairs; matching authority stays separate.
use super::{Coverage, Execution, helper_reports, helpers, pair_differences as native};
use crate::clones::Handoff;
use rstyle_ccgraph::{Decision, RunReport};
use rstyle_pdg::{ByteSpan, HirNode, HirRef};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
pub(crate) struct Reports<'a> {
    pub pair_explanations: Report<'a>,
    pub helper_proposals: helper_reports::Report<'a>,
}

pub(in crate::clones) struct Requests {
    pub explain: bool,
    pub helpers: bool,
    pub matching_complete: bool,
}

impl Reports<'_> {
    pub(crate) fn permits_success(&self) -> bool {
        self.pair_explanations.permits_success() && self.helper_proposals.permits_success()
    }

    pub(crate) fn render(&self) -> String {
        format!(
            "{}{}",
            self.pair_explanations.render(),
            self.helper_proposals.render()
        )
    }
}

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    schema: &'static str,
    execution: Execution,
    coverage: Coverage,
    population: &'static str,
    expected_pairs: Option<usize>,
    assessed_pairs: usize,
    complete_pairs: usize,
    reasons: Vec<&'static str>,
    pairs: Vec<Pair<'a>>,
    claim: &'static str,
}

#[derive(Serialize)]
struct Pair<'a> {
    first: &'a str,
    second: &'a str,
    coverage: native::Coverage,
    correspondence: Vec<native::Correspondence<'a>>,
    holes: Vec<Hole<'a>>,
    reason: Option<&'static str>,
}

#[derive(Serialize)]
struct Hole<'a> {
    kind: native::HoleKind,
    role: native::Role,
    left: Node<'a>,
    right: Node<'a>,
}

/// Compact references to native occurrences, never recursively exported trees.
#[derive(Serialize)]
struct Node<'a> {
    hir: &'a HirRef,
    span: ByteSpan,
    kind: &'a str,
    scalar_type: Option<&'a str>,
    literal: Option<&'a str>,
    operator: Option<&'a str>,
    formal_target: Option<&'a HirRef>,
}

impl<'a> From<&'a HirNode> for Node<'a> {
    fn from(node: &'a HirNode) -> Self {
        Self {
            hir: &node.hir,
            span: node.span,
            kind: &node.kind,
            scalar_type: node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()),
            literal: node.lit.as_deref(),
            operator: node.dispatch.as_ref().map(|dispatch| dispatch.op.as_str()),
            formal_target: node
                .res
                .as_ref()
                .and_then(|resolution| resolution.target.as_ref()),
        }
    }
}

pub(in crate::clones) fn evaluate<'a>(
    handoff: &'a Handoff,
    matching: &'a RunReport,
    requests: Requests,
) -> Result<Reports<'a>, String> {
    let requested = requests.explain || requests.helpers;
    let compared = if requested {
        explain_accepted(handoff, matching, requests.helpers)?
    } else {
        Vec::new()
    };
    let (pairs, helpers): (Vec<_>, Vec<_>) = compared.into_iter().unzip();
    let helper_proposals = helper_reports::summarize(
        helpers.into_iter().flatten().collect(),
        handoff,
        requests.helpers,
        requests.matching_complete,
    );
    let pair_explanations = summarize(pairs, requested, requests.matching_complete);

    Ok(Reports {
        pair_explanations,
        helper_proposals,
    })
}

fn summarize<'a>(pairs: Vec<Pair<'a>>, requested: bool, matching_complete: bool) -> Report<'a> {
    let complete_pairs = pairs
        .iter()
        .filter(|pair| pair.coverage == native::Coverage::Complete)
        .count();
    let complete = requested && matching_complete && complete_pairs == pairs.len();
    let mut reasons = Vec::new();
    if requested && !matching_complete {
        reasons.push("Matching scope is incomplete; the full accepted-pair denominator is unknown");
    }
    if complete_pairs < pairs.len() {
        reasons.push(
            "Some accepted pairs are outside the bounded native comparison; see pair reasons",
        );
    }

    Report {
        schema: "rstyle.native-pair-differences/1",
        execution: execution(&pairs, requested),
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        population: "accepted strict matching pairs",
        expected_pairs: (requested && matching_complete).then_some(pairs.len()),
        assessed_pairs: pairs.len(),
        complete_pairs,
        reasons,
        pairs,
        claim: "Ordered native scalar differences do not establish equivalence or safe refactoring.",
    }
}

fn execution(pairs: &[Pair<'_>], requested: bool) -> Execution {
    if pairs
        .iter()
        .any(|pair| pair.coverage == native::Coverage::BudgetExhausted)
    {
        Execution::BudgetExhausted
    } else if requested {
        Execution::Completed
    } else {
        Execution::NotRequested
    }
}

type Compared<'a> = (Pair<'a>, Option<helper_reports::Pair<'a>>);

fn explain_accepted<'a>(
    handoff: &'a Handoff,
    matching: &'a RunReport,
    helpers_requested: bool,
) -> Result<Vec<Compared<'a>>, String> {
    let endpoints = endpoints(handoff, matching)?;
    matching
        .pairs
        .iter()
        .filter(|pair| pair.decision == Decision::Accepted)
        .map(|pair| {
            let left = endpoints
                .get(pair.first.as_str())
                .ok_or_else(|| format!("Missing admitted endpoint {}", pair.first))?;
            let right = endpoints
                .get(pair.second.as_str())
                .ok_or_else(|| format!("Missing admitted endpoint {}", pair.second))?;
            let comparison = native::compare(left, right);
            let helper = helpers_requested.then(|| helper_reports::Pair {
                first: &pair.first,
                second: &pair.second,
                assessment: helpers::evaluate(&comparison),
            });
            let result = comparison.into_assessment();
            Ok((export(pair, result), helper))
        })
        .collect()
}

fn export<'a>(pair: &'a rstyle_ccgraph::PairRecord, result: native::Assessment<'a>) -> Pair<'a> {
    Pair {
        first: &pair.first,
        second: &pair.second,
        coverage: result.coverage,
        correspondence: result.correspondence,
        holes: result
            .holes
            .into_iter()
            .map(|hole| Hole {
                kind: hole.kind,
                role: hole.role,
                left: hole.left.into(),
                right: hole.right.into(),
            })
            .collect(),
        reason: result.reason,
    }
}

fn endpoints<'a>(
    handoff: &'a Handoff,
    matching: &RunReport,
) -> Result<BTreeMap<&'a str, native::Endpoint<'a, 'a>>, String> {
    let records: BTreeMap<_, _> = handoff
        .assessed
        .iter()
        .map(|record| (record.key.to_string(), record))
        .collect();
    let required = accepted_keys(matching);
    // Validate each requested immutable endpoint once, including negative outcomes.
    handoff
        .admitted
        .iter()
        .filter(|graph| required.contains(graph.key.as_str()))
        .map(|graph| {
            let record = records
                .get(&graph.key)
                .ok_or_else(|| format!("Missing native endpoint {}", graph.key))?;
            Ok((
                graph.key.as_str(),
                native::Endpoint::checked(graph, record)?,
            ))
        })
        .collect()
}

fn accepted_keys(matching: &RunReport) -> BTreeSet<&str> {
    matching
        .pairs
        .iter()
        .filter(|pair| pair.decision == Decision::Accepted)
        .flat_map(|pair| [pair.first.as_str(), pair.second.as_str()])
        .collect()
}

impl Report<'_> {
    pub(crate) fn permits_success(&self) -> bool {
        self.execution == Execution::NotRequested
            || (self.execution == Execution::Completed && self.coverage == Coverage::Complete)
    }

    pub(crate) fn render(&self) -> String {
        if self.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nPair explanations: {:?} coverage; {} of {} assessed pairs complete. {}",
            self.coverage, self.complete_pairs, self.assessed_pairs, self.claim
        );
        for pair in &self.pairs {
            output.push_str(&format!(
                "\nNative differences {} <> {}: {:?}; {} holes. {}",
                pair.first,
                pair.second,
                pair.coverage,
                pair.holes.len(),
                pair.reason.unwrap_or("")
            ));
        }
        output
    }
}
