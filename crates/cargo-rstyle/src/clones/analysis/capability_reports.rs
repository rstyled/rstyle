//! Witnessed capability proposals from original ordered method usage.
mod scope;
use super::{Execution, capability_pairs, declaration_indices::Indices};
use crate::clones::{Args, FunctionKey, Handoff, capture, identity::UnitId};
use rstyle_pdg::consumer_facts::{Availability, dispatch::capabilities};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    schema: &'static str,
    detector: &'static str,
    #[serde(flatten)]
    scope: scope::Summary,
    pub findings: usize,
    functions: Vec<FunctionReport<'a>>,
    proposals: Vec<Proposal>,
    suppressed: Vec<PairDecision>,
    unavailable_pairs: Vec<PairDecision>,
    limitations: &'static [&'static str],
}

#[derive(Serialize)]
pub(super) struct FunctionReport<'a> {
    key: FunctionKey,
    name: &'a str,
    source_file: Option<&'a str>,
    fn_span: rstyle_pdg::ByteSpan,
    pub execution: Execution,
    pub evidence: capabilities::Assessment,
}

#[derive(Serialize)]
struct Proposal {
    left: FunctionKey,
    right: FunctionKey,
    roles: [capability_pairs::Role; 2],
}

#[derive(Serialize)]
struct PairDecision {
    left: FunctionKey,
    right: FunctionKey,
    reason: &'static str,
}

pub(in crate::clones) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    args: &Args,
    indices: &Indices,
) -> Result<Report<'a>, String> {
    let functions = if args.capability_proposals {
        inspect(handoff, args, indices)?
    } else {
        Vec::new()
    };
    let mut scope = scope::Summary::new(handoff, capture, args.capability_proposals);
    let paired = pair(&functions, args.max_capability_pairs, &mut scope)?;
    scope.finish(&functions, paired.unavailable_pairs.len());

    Ok(Report {
        schema: "rstyle.capability-proposals/1",
        detector: "ordered-local-capability/1",
        scope,
        findings: paired.proposals.len(),
        functions,
        proposals: paired.proposals,
        suppressed: paired.suppressed,
        unavailable_pairs: paired.unavailable_pairs,
        limitations: &[
            "Advisory two-member local capabilities witnessed by ordered wrapper calls; no behavioral equivalence or safe rewrite claim.",
            "Plain shared local owners and zero-argument shared-self/u32 methods only; method effects and lifetimes require review.",
            "Exact existing local trait members suppress redundant advice; mixed or unresolved origins remain unavailable.",
            "Legacy review and suppressions are not applied; no legacy capability is retired.",
        ],
    })
}

fn inspect<'a>(
    handoff: &'a Handoff,
    args: &Args,
    indices: &Indices,
) -> Result<Vec<FunctionReport<'a>>, String> {
    let limit = args.max_analysis_functions.unwrap_or(4096).min(4096);
    handoff
        .assessed
        .iter()
        .enumerate()
        .map(|(ordinal, function)| {
            let evidence = if ordinal < limit {
                indices
                    .get(function.key.unit())?
                    .evaluate_capability_wrapper(&function.facts)?
            } else {
                capabilities::Assessment {
                    availability: Availability::Unknown {
                        reason: "Capability owner budget exhausted".into(),
                    },
                    reason: None,
                    budget_exhausted: true,
                }
            };
            Ok(FunctionReport {
                key: function.key,
                name: &function.facts.name,
                source_file: function
                    .facts
                    .build
                    .frontend_options
                    .get("source_file")
                    .and_then(serde_json::Value::as_str),
                fn_span: function.facts.fn_span,
                execution: if evidence.budget_exhausted {
                    Execution::BudgetExhausted
                } else {
                    Execution::Completed
                },
                evidence,
            })
        })
        .collect()
}

#[derive(Default)]
struct Paired {
    proposals: Vec<Proposal>,
    suppressed: Vec<PairDecision>,
    unavailable_pairs: Vec<PairDecision>,
}

fn pair(
    functions: &[FunctionReport<'_>],
    limit: usize,
    scope: &mut scope::Summary,
) -> Result<Paired, String> {
    let groups = wrapper_groups(functions);
    let mut remaining = limit;
    let batches = groups
        .values()
        .map(|group| {
            let wrappers: Vec<_> = group.iter().map(|(_, wrapper)| *wrapper).collect();
            let pairs = capability_pairs::evaluate(&wrappers, remaining)?;
            remaining = remaining.saturating_sub(pairs.assessed_pairs);
            scope.accept_pairs(&pairs);
            Ok(export_pairs(pairs, group))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(batches
        .into_iter()
        .fold(Paired::default(), |mut all, batch| {
            all.proposals.extend(batch.proposals);
            all.suppressed.extend(batch.suppressed);
            all.unavailable_pairs.extend(batch.unavailable_pairs);
            all
        }))
}

fn wrapper_groups<'a>(
    functions: &'a [FunctionReport<'_>],
) -> BTreeMap<UnitId, Vec<(FunctionKey, &'a capabilities::Wrapper)>> {
    let mut groups: BTreeMap<UnitId, Vec<_>> = BTreeMap::new();
    for function in functions {
        if let Availability::Known(Some(wrapper)) = &function.evidence.availability {
            groups
                .entry(function.key.unit())
                .or_default()
                .push((function.key, wrapper));
        }
    }
    groups
}

fn export_pairs(
    pairs: capability_pairs::Report,
    group: &[(FunctionKey, &capabilities::Wrapper)],
) -> Paired {
    Paired {
        proposals: pairs
            .proposals
            .into_iter()
            .map(|pair| Proposal {
                left: group[pair.left].0,
                right: group[pair.right].0,
                roles: pair.roles,
            })
            .collect(),
        suppressed: export_decisions(pairs.suppressed, group).collect(),
        unavailable_pairs: export_decisions(pairs.unavailable, group).collect(),
    }
}

pub(in crate::clones) struct Finding<'a> {
    pub left: FunctionKey,
    pub right: FunctionKey,
    pub first: &'a capabilities::Wrapper,
    pub second: &'a capabilities::Wrapper,
}

impl Report<'_> {
    pub(in crate::clones) fn finding_proposals(&self) -> impl Iterator<Item = Finding<'_>> {
        let wrappers: BTreeMap<_, _> = self
            .functions
            .iter()
            .filter_map(|function| {
                if let Availability::Known(Some(wrapper)) = &function.evidence.availability {
                    Some((function.key, wrapper))
                } else {
                    None
                }
            })
            .collect();
        self.proposals.iter().map(move |proposal| Finding {
            left: proposal.left,
            right: proposal.right,
            first: wrappers[&proposal.left],
            second: wrappers[&proposal.right],
        })
    }

    pub(in crate::clones) fn render(&self) -> String {
        if self.scope.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nCapability proposals: {:?} coverage; {} findings; {} existing-trait suppressions.",
            self.scope.coverage,
            self.findings,
            self.suppressed.len()
        );
        let functions: BTreeMap<_, _> = self
            .functions
            .iter()
            .map(|function| (function.key, function))
            .collect();

        for proposal in &self.proposals {
            output.push_str(&format!(
                "\nAdvisory capability_set {} / {}: two ordered method roles.",
                functions[&proposal.left].location(),
                functions[&proposal.right].location()
            ));
        }
        output
    }
}

impl FunctionReport<'_> {
    fn location(&self) -> String {
        format!(
            "{} {} at {} [bytes {}..{}]",
            self.key,
            self.name,
            self.source_file.unwrap_or("<source unavailable>"),
            self.fn_span.start,
            self.fn_span.end
        )
    }
}

impl super::local_scope::FunctionStatus for FunctionReport<'_> {
    fn status(&self) -> super::local_scope::Status {
        let assessed = self.execution == Execution::Completed;
        super::local_scope::Status {
            assessed,
            evaluated: assessed && matches!(self.evidence.availability, Availability::Known(_)),
        }
    }
}

fn export_decisions<'a>(
    decisions: Vec<capability_pairs::PairDecision>,
    group: &'a [(FunctionKey, &capabilities::Wrapper)],
) -> impl Iterator<Item = PairDecision> + 'a {
    decisions
        .into_iter()
        .map(move |pair| PairDecision::from_pair(group, pair))
}

impl PairDecision {
    fn from_pair(
        group: &[(FunctionKey, &capabilities::Wrapper)],
        pair: capability_pairs::PairDecision,
    ) -> Self {
        Self {
            left: group[pair.left].0,
            right: group[pair.right].0,
            reason: pair.reason,
        }
    }
}

impl crate::clones::analysis::Completion for Report<'_> {
    fn permits_success(&self) -> bool {
        self.scope.permits_success()
    }
}
