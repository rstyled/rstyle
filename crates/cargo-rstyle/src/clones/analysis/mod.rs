//! Opt-in local consumers with evidence, execution and scope reported separately.
mod capability_pairs;
pub(super) mod capability_reports;
pub(super) mod declaration_indices;
pub(super) mod dispatch_reports;
pub(super) mod effect_reports;
mod guards;
pub(super) mod helper_reports;
mod helpers;
mod local_scope;
pub(super) mod mutation_reports;
mod pair_differences;
pub(super) mod pair_reports;
mod source_rules;
mod source_suppressions;

use super::{Args, Function, Handoff, capture};
use rstyle_core::config::{Config as SourcePolicy, FnLengthConfig, FnParamsConfig};
use rstyle_pdg::consumer_facts::{Availability, LocalFacts};
use serde::{Deserialize, Serialize};

/// Whether requested work has enough completed coverage to permit a clean exit.
/// Implementations retain their consumer-specific execution and scope rules.
pub(in crate::clones) trait Completion {
    fn permits_success(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Finding,
    EvaluatedNoFinding,
    NotEvaluated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Complete,
    Incomplete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactAvailability {
    Known,
    Unknown,
    Unsupported,
    NotApplicable,
}

#[derive(Debug, Serialize)]
pub struct Assessment {
    pub rule_id: &'static str,
    pub outcome: Outcome,
    pub coverage: Coverage,
    pub availability: FactAvailability,
    pub actual: Option<usize>,
    pub limit: usize,
    pub reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Execution {
    NotRequested,
    Completed,
    BudgetExhausted,
    Failed,
}

#[derive(Debug, Serialize)]
pub struct Scope {
    pub population: &'static str,
    pub coverage: Coverage,
    pub assessed_functions: usize,
    pub expected_functions: Option<usize>,
    pub evaluated_rules: usize,
    pub unavailable_rules: usize,
    pub unavailable_source_suppressions: usize,
    pub reasons: Vec<String>,
}

#[derive(Serialize)]
pub struct Report<'a> {
    schema: &'static str,
    pub execution: Execution,
    pub scope: Scope,
    pub findings: usize,
    pub visible_findings: usize,
    pub suppressed_findings: usize,
    effective_config: Option<Settings>,
    limitations: &'static [&'static str],
    functions: Vec<FunctionReport<'a>>,
}

#[derive(Serialize)]
struct FunctionReport<'a> {
    key: super::FunctionKey,
    name: &'a str,
    source_file: Option<&'a str>,
    fn_span: rstyle_pdg::ByteSpan,
    source_digest: &'a str,
    execution: Execution,
    assessments: Vec<Assessment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_suppression: Option<source_suppressions::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guard_clause: Option<guards::GuardAssessment>,
}

#[derive(Serialize)]
struct Settings {
    max_lines: usize,
    max_params: usize,
    count_self: bool,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SourceConfig {
    fn_length: FnLengthConfig,
    fn_params: FnParamsConfig,
}

pub(super) struct Config {
    source: Option<SourcePolicy>,
    guards: bool,
}

pub(super) fn configuration(args: &Args) -> Result<Option<Config>, String> {
    if !args.source_rules && !args.guard_clauses {
        return Ok(None);
    }
    Ok(Some(Config {
        source: args.source_rules.then(|| source_policy(args)).transpose()?,
        guards: args.guard_clauses,
    }))
}

fn source_policy(args: &Args) -> Result<SourcePolicy, String> {
    let local: SourceConfig = match &args.analysis_config {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?
        }
        None => SourceConfig::default(),
    };
    Ok(SourcePolicy {
        fn_length: local.fn_length,
        fn_params: local.fn_params,
        ..SourcePolicy::default()
    })
}

pub(super) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    config: Option<&Config>,
    args: &Args,
) -> Report<'a> {
    let limit = args.max_analysis_functions.unwrap_or(usize::MAX);
    let functions: Vec<_> = config
        .map(|config| {
            handoff
                .assessed
                .iter()
                .enumerate()
                .map(|(index, function)| assess(function, config, index >= limit))
                .collect()
        })
        .unwrap_or_default();
    let scope = scope(&functions, handoff, capture, config.is_some());
    let execution = execution(&functions, &scope, config.is_some());
    let findings = count(&functions, Outcome::Finding);
    let visible_findings = functions.iter().map(FunctionReport::visible_findings).sum();

    Report {
        schema: "rstyle.consumer-analysis/1",
        execution,
        scope,
        findings,
        visible_findings,
        suppressed_findings: findings - visible_findings,
        effective_config: config
            .and_then(|config| config.source.as_ref())
            .map(|config| Settings {
                max_lines: config.fn_length.max_lines,
                max_params: config.fn_params.max_params,
                count_self: config.fn_params.count_self,
            }),
        limitations: &[
            "Source rules do not establish body effects or refactoring safety.",
            "Length uses the legacy code-line policy, including its string/comment limitations.",
            "Source rules honor captured legacy length/parameter suppressions; exclusions and review ledgers remain separate.",
            "Guard opportunities are bounded syntactic advice; review conditions, lifetimes and drops before editing.",
        ],
        functions,
    }
}

fn scope(
    functions: &[FunctionReport<'_>],
    handoff: &Handoff,
    capture: &capture::Capture,
    requested: bool,
) -> Scope {
    let mut reasons: Vec<_> = capture
        .failures
        .iter()
        .chain(&handoff.failures)
        .cloned()
        .collect();
    let expected_functions = reasons.is_empty().then_some(handoff.assessed.len());
    let unavailable_rules = count(functions, Outcome::NotEvaluated);
    let unavailable_source_suppressions = unavailable_source_suppressions(functions);
    if unavailable_rules > 0 {
        reasons.push(
            "Some requested rules could not be evaluated; see per-function evidence reasons".into(),
        );
    }
    if unavailable_source_suppressions > 0 {
        reasons.push("Some source suppression observations are unavailable".into());
    }
    let complete = requested
        && reasons.is_empty()
        && functions
            .iter()
            .all(|function| function.execution == Execution::Completed)
        && expected_functions == Some(functions.len());

    Scope {
        population: "captured function owners; inactive definitions remain in owned_source_inventory",
        coverage: if complete {
            Coverage::Complete
        } else {
            Coverage::Incomplete
        },
        assessed_functions: functions
            .iter()
            .filter(|function| function.execution != Execution::BudgetExhausted)
            .count(),
        expected_functions,
        evaluated_rules: count(functions, Outcome::Finding)
            + count(functions, Outcome::EvaluatedNoFinding),
        unavailable_rules,
        unavailable_source_suppressions,
        reasons,
    }
}

fn unavailable_source_suppressions(functions: &[FunctionReport<'_>]) -> usize {
    functions
        .iter()
        .filter_map(|function| function.source_suppression.as_ref())
        .filter(|suppression| !suppression.complete())
        .count()
}

fn count(functions: &[FunctionReport<'_>], outcome: Outcome) -> usize {
    functions
        .iter()
        .flat_map(|function| {
            function
                .assessments
                .iter()
                .map(|assessment| assessment.outcome)
                .chain(function.guard_clause.iter().map(|guard| guard.outcome))
        })
        .filter(|value| *value == outcome)
        .count()
}

fn execution(functions: &[FunctionReport<'_>], scope: &Scope, requested: bool) -> Execution {
    if !requested {
        Execution::NotRequested
    } else if scope.expected_functions.is_none()
        || functions
            .iter()
            .any(|function| function.execution == Execution::Failed)
    {
        Execution::Failed
    } else if functions
        .iter()
        .any(|function| function.execution == Execution::BudgetExhausted)
    {
        Execution::BudgetExhausted
    } else {
        Execution::Completed
    }
}

fn assess<'a>(function: &'a Function, config: &Config, exhausted: bool) -> FunctionReport<'a> {
    let (execution, facts) = inputs(function, exhausted);
    let mut assessments = config
        .source
        .as_ref()
        .map(|policy| source_rules::evaluate(&facts, policy))
        .unwrap_or_default();
    let source_suppression = config.source.as_ref().map(|_| {
        let suppression = source_suppressions::evaluate(&facts.preceding_comment);
        assessments.extend(suppression.invalid_assessment());
        suppression
    });
    let guard_clause = config.guards.then(|| {
        let mut guard = guards::evaluate(&facts.source);
        if execution != Execution::Completed {
            guard.execution = execution;
        }
        guard
    });
    let execution = if execution == Execution::Completed {
        guard_clause
            .as_ref()
            .map_or(execution, |guard| guard.execution)
    } else {
        execution
    };

    FunctionReport {
        key: function.key,
        name: &function.facts.name,
        source_file: function
            .facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(|v| v.as_str()),
        fn_span: function.facts.fn_span,
        source_digest: &function.facts.source.digest,
        execution,
        assessments,
        source_suppression,
        guard_clause,
    }
}

fn inputs(function: &Function, exhausted: bool) -> (Execution, LocalFacts<'_>) {
    if exhausted {
        return (
            Execution::BudgetExhausted,
            unavailable("Analysis function budget exhausted"),
        );
    }
    match rstyle_pdg::consumer_facts::inspect(&function.facts) {
        Ok(facts) => (Execution::Completed, facts),
        Err(error) => (Execution::Failed, unavailable(&error)),
    }
}

fn unavailable(reason: &str) -> LocalFacts<'static> {
    LocalFacts {
        preceding_comment: Availability::Unknown {
            reason: reason.into(),
        },
        source: Availability::Unknown {
            reason: reason.into(),
        },
        signature: Availability::Unknown {
            reason: reason.into(),
        },
    }
}

impl Report<'_> {
    pub fn permits_success(&self) -> bool {
        self.execution == Execution::NotRequested
            || (self.execution == Execution::Completed && self.scope.coverage == Coverage::Complete)
    }

    pub fn render(&self) -> String {
        if self.execution == Execution::NotRequested {
            return String::new();
        }
        let mut output = format!(
            "\nLocal analysis: {:?}; {:?} coverage; {} visible of {} findings; {} suppressed; {} evaluated, {} unavailable rules.",
            self.execution,
            self.scope.coverage,
            self.visible_findings,
            self.findings,
            self.suppressed_findings,
            self.scope.evaluated_rules,
            self.scope.unavailable_rules
        );
        output.push_str(
            &self
                .functions
                .iter()
                .map(FunctionReport::render)
                .collect::<String>(),
        );
        output
    }
}

impl FunctionReport<'_> {
    fn visible(&self, assessment: &Assessment) -> bool {
        assessment.outcome == Outcome::Finding
            && !self
                .source_suppression
                .as_ref()
                .is_some_and(|suppression| suppression.allows(assessment.rule_id))
    }

    fn visible_findings(&self) -> usize {
        self.assessments
            .iter()
            .filter(|rule| self.visible(rule))
            .count()
            + usize::from(
                self.guard_clause
                    .as_ref()
                    .is_some_and(|guard| guard.outcome == Outcome::Finding),
            )
    }

    fn location(&self) -> String {
        format!(
            "{} at {} [bytes {}..{}]",
            self.name,
            self.source_file.unwrap_or("<source unavailable>"),
            self.fn_span.start,
            self.fn_span.end
        )
    }

    fn render(&self) -> String {
        let location = self.location();
        let source = self
            .assessments
            .iter()
            .filter(|assessment| self.visible(assessment))
            .map(|assessment| {
                format!(
                    "\n{} {location}: {} > {}. {}",
                    assessment.rule_id,
                    assessment.actual.unwrap_or(0),
                    assessment.limit,
                    assessment.reasons.join(" ")
                )
            })
            .collect::<String>();
        let guard = self
            .guard_clause
            .as_ref()
            .and_then(|guard| guard.opportunity.as_ref())
            .map(|opportunity| {
                format!(
                    "\nguard_clause {location}: {} (fallback {:?}).",
                    opportunity.advisory, opportunity.fallback_branch
                )
            })
            .unwrap_or_default();
        format!("{source}{guard}")
    }
}
