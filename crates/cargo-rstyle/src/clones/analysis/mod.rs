//! Opt-in local consumers with evidence, execution and scope reported separately.
mod capability_pairs;
pub(super) mod capability_reports;
mod changed_file_reports;
pub(in crate::clones) use changed_file_reports::{checked_path, selected_file};
pub(super) mod changed_scope;
pub(super) mod declaration_indices;
pub(super) mod dispatch_reports;
pub(super) mod effect_reports;
mod guard_reviews;
pub(in crate::clones) mod guards;
pub(in crate::clones) use guard_reviews::GuardReview;
pub(super) mod helper_reports;
mod helpers;
mod local_scope;
pub(super) mod mutation_reports;
mod pair_differences;
pub(super) mod pair_reports;
mod source_exclusions;
mod source_reviews;
mod source_rules;
mod source_spacing;
mod source_suppressions;

use super::{Args, Function, Handoff, capture};
use rstyle_core::config::Config as SourcePolicy;
use rstyle_pdg::consumer_facts::{Availability, LocalFacts};
use serde::Serialize;

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

pub(in crate::clones) struct SourceReview<'a> {
    pub key: super::FunctionKey,
    pub selected: bool,
    pub signature: Option<rstyle_pdg::consumer_facts::Signature>,
    pub count_self: bool,
    pub parameter: RuleReview<'a>,
    pub length: RuleReview<'a>,
    pub spacing: RuleReview<'a>,
    pub min_statements: Option<usize>,
}

pub(in crate::clones) struct RuleReview<'a> {
    pub suppressed: bool,
    pub assessment: Option<&'a Assessment>,
    pub complete: bool,
    pub visible: bool,
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
    #[serde(flatten)]
    source: SourceScope,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    changed_scope: Option<changed_scope::Report<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guard_changed_scope: Option<changed_scope::Report<'a>>,
    effective_config: Option<Settings<'a>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    changed_file: Option<bool>,
    #[serde(skip)]
    signature: Option<rstyle_pdg::consumer_facts::Signature>,
    #[serde(skip)]
    source_review_hidden: source_reviews::Hidden,
    #[serde(skip)]
    guard_review_hidden: std::collections::BTreeSet<usize>,
    #[serde(skip)]
    guard_review_source: Option<&'a str>,
    execution: Execution,
    assessments: Vec<Assessment>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    outside_source_rules: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_suppression: Option<source_suppressions::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_exclusion: Option<source_exclusions::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guard_clause: Option<guards::GuardAssessment>,
}

#[derive(Serialize)]
struct Settings<'a> {
    exclude: &'a [String],
    min_statements: Option<usize>,
    max_lines: usize,
    max_params: usize,
    count_self: bool,
}

pub(super) struct Config {
    source: Option<ConfiguredSource>,
    guards: bool,
    changed_files: Option<changed_scope::Selection>,
}

impl Config {
    pub(in crate::clones) fn changed_files(&self) -> Option<&changed_scope::Selection> {
        self.changed_files.as_ref()
    }
}

pub(super) fn configuration(args: &Args) -> Result<Option<Config>, String> {
    if !args.source_rules && !args.guard_clauses && args.changed_files.is_none() {
        return Ok(None);
    }
    Ok(Some(Config {
        source: args.source_rules.then(|| source_policy(args)).transpose()?,
        guards: args.guard_clauses,
        changed_files: args
            .changed_files
            .as_deref()
            .map(changed_scope::Selection::load)
            .transpose()?,
    }))
}

struct ConfiguredSource {
    spacing: bool,
    rules: SourcePolicy,
    exclusions: rstyle_core::exclusions::Matcher,
}

impl<'a> Settings<'a> {
    fn new(config: &'a ConfiguredSource) -> Self {
        Self {
            exclude: &config.rules.exclude.0,
            min_statements: config
                .spacing
                .then_some(config.rules.fn_spacing.min_statements),
            max_lines: config.rules.fn_length.max_lines,
            max_params: config.rules.fn_params.max_params,
            count_self: config.rules.fn_params.count_self,
        }
    }
}

fn source_policy(args: &Args) -> Result<ConfiguredSource, String> {
    let (rules, spacing) = match &args.analysis_config {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            let document: toml::Value =
                toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
            let spacing = document.get("fn_spacing").is_some();
            let rules: SourcePolicy = document
                .try_into()
                .map_err(|error| format!("{}: {error}", path.display()))?;
            (rules, spacing)
        }
        None => (SourcePolicy::default(), false),
    };

    let exclusions = rstyle_core::exclusions::Matcher::compile(&rules.exclude.0)?;
    Ok(ConfiguredSource {
        spacing,
        rules,
        exclusions,
    })
}

pub(super) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    config: Option<&'a Config>,
    args: &Args,
) -> Report<'a> {
    let config = config.filter(|config| config.source.is_some() || config.guards);
    let mut functions = assess_functions(
        handoff,
        &capture.plan.root,
        config,
        args.max_analysis_functions.unwrap_or(usize::MAX),
    );
    let scope = scope(&functions, handoff, capture, config.is_some());
    let execution = execution(&functions, &scope, config.is_some());
    let findings = count(&functions, Outcome::Finding);
    let visible_findings = functions.iter().map(FunctionReport::visible_findings).sum();
    let changed = changed_file_reports::evaluate(
        config,
        &mut functions,
        &capture.plan.root,
        args.select_guards,
    );

    Report {
        schema: "rstyle.consumer-analysis/1",
        execution,
        scope,
        findings,
        visible_findings,
        suppressed_findings: findings - visible_findings,
        changed_scope: changed.sources,
        guard_changed_scope: changed.guards,
        effective_config: config
            .and_then(|config| config.source.as_ref())
            .map(Settings::new),
        limitations: &[
            "Source rules do not establish body effects or refactoring safety.",
            "Length uses the legacy code-line policy, including its string/comment limitations.",
            "Source rules honor configured workspace path exclusions and captured named source suppressions; review ledgers remain separate.",
            "Guard opportunities are bounded syntactic advice; review conditions, lifetimes and drops before editing.",
        ],
        functions,
    }
}

fn assess_functions<'a>(
    handoff: &'a Handoff,
    root: &std::path::Path,
    config: Option<&Config>,
    limit: usize,
) -> Vec<FunctionReport<'a>> {
    config
        .map(|config| {
            handoff
                .assessed
                .iter()
                .enumerate()
                .map(|(index, function)| assess(function, config, index >= limit, root))
                .collect()
        })
        .unwrap_or_default()
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
    let source = source_scope(functions);
    let unavailable_rules = rule_statuses(functions)
        .filter(|(_, coverage)| *coverage == Coverage::Incomplete)
        .count();
    if unavailable_rules > 0 {
        reasons.push(
            "Some requested rules could not be evaluated; see per-function evidence reasons".into(),
        );
    }
    source.add_reasons(&mut reasons);
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
        source,
        reasons,
    }
}

#[derive(Debug, Default, Serialize)]
struct SourceScope {
    unavailable_source_suppressions: usize,
    unavailable_source_exclusions: usize,
    excluded_source_functions: usize,
    excluded_rules: usize,
    #[serde(skip_serializing_if = "is_zero")]
    outside_source_functions: usize,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

fn source_scope(functions: &[FunctionReport<'_>]) -> SourceScope {
    functions
        .iter()
        .fold(SourceScope::default(), |mut scope, function| {
            scope.unavailable_source_suppressions += usize::from(
                function
                    .source_suppression
                    .as_ref()
                    .is_some_and(|value| !value.complete()),
            );
            scope.unavailable_source_exclusions += usize::from(
                function
                    .source_exclusion
                    .as_ref()
                    .is_some_and(|value| !value.complete()),
            );
            scope.excluded_source_functions += usize::from(
                function
                    .source_exclusion
                    .as_ref()
                    .is_some_and(|value| value.excluded()),
            );
            scope.outside_source_functions += usize::from(function.outside_source_rules);
            scope.excluded_rules += function
                .assessments
                .iter()
                .filter(|rule| {
                    !function.outside_source_rules
                        && rule.outcome == Outcome::NotEvaluated
                        && rule.coverage == Coverage::Complete
                })
                .count();
            scope
        })
}

impl SourceScope {
    fn add_reasons(&self, reasons: &mut Vec<String>) {
        for (count, reason) in [
            (
                self.unavailable_source_suppressions,
                "Some source suppression observations are unavailable",
            ),
            (
                self.unavailable_source_exclusions,
                "Some source exclusion observations are unavailable",
            ),
        ] {
            if count > 0 {
                reasons.push(reason.into());
            }
        }
    }
}

fn rule_statuses<'a>(
    functions: &'a [FunctionReport<'_>],
) -> impl Iterator<Item = (Outcome, Coverage)> + 'a {
    functions.iter().flat_map(|function| {
        function
            .assessments
            .iter()
            .map(|assessment| (assessment.outcome, assessment.coverage))
            .chain(
                function
                    .guard_clause
                    .iter()
                    .map(|guard| (guard.outcome, guard.coverage)),
            )
    })
}

fn count(functions: &[FunctionReport<'_>], outcome: Outcome) -> usize {
    rule_statuses(functions)
        .filter(|(value, _)| *value == outcome)
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

fn assess<'a>(
    function: &'a Function,
    config: &Config,
    exhausted: bool,
    root: &std::path::Path,
) -> FunctionReport<'a> {
    let statements = config
        .source
        .as_ref()
        .is_some_and(|policy| policy.spacing && policy.rules.fn_spacing.min_statements > 0);
    let (execution, facts) = inputs(function, exhausted, statements);
    let source = config
        .source
        .as_ref()
        .map(|policy| source_assessments(function, policy, &facts, root))
        .unwrap_or_default();
    let guard_clause = config
        .guards
        .then(|| assess_guard(&facts.source, execution));
    let execution = guard_execution(execution, guard_clause.as_ref());

    FunctionReport {
        key: function.key,
        name: &function.facts.name,
        source_file: source_file(function),
        fn_span: function.facts.fn_span,
        source_digest: &function.facts.source.digest,
        changed_file: None,
        signature: match facts.signature {
            Availability::Known(signature) => Some(signature),
            _ => None,
        },
        source_review_hidden: source_reviews::Hidden::default(),
        guard_review_hidden: std::collections::BTreeSet::new(),
        guard_review_source: match facts.source {
            Availability::Known(source) if config.guards => Some(source),
            _ => None,
        },
        execution,
        assessments: source.rules,
        outside_source_rules: source.outside,
        source_suppression: source.suppression,
        source_exclusion: source.exclusion,
        guard_clause,
    }
}

fn guard_execution(execution: Execution, guard: Option<&guards::GuardAssessment>) -> Execution {
    if execution == Execution::Completed {
        guard.map_or(execution, |assessment| assessment.execution)
    } else {
        execution
    }
}

fn assess_guard(source: &Availability<&str>, execution: Execution) -> guards::GuardAssessment {
    let mut guard = guards::evaluate(source);
    if execution != Execution::Completed {
        guard.execution = execution;
    }
    guard
}

#[derive(Default)]
struct SourceAssessments {
    rules: Vec<Assessment>,
    exclusion: Option<source_exclusions::Report>,
    suppression: Option<source_suppressions::Report>,
    outside: bool,
}

fn source_assessments(
    function: &Function,
    policy: &ConfiguredSource,
    facts: &LocalFacts<'_>,
    root: &std::path::Path,
) -> SourceAssessments {
    if matches!(facts.source, Availability::Unsupported { .. })
        && function
            .facts
            .source_diagnostics
            .iter()
            .any(|finding| finding.construct == "macro_generated_function")
    {
        return SourceAssessments {
            rules: source_rules::not_applicable(
                &policy.rules,
                policy.spacing,
                "Expansion-generated signatures are outside original source rules",
            ),
            outside: true,
            ..SourceAssessments::default()
        };
    }
    let exclusion = source_exclusions::evaluate(facts, function, root, &policy.exclusions);
    if exclusion.excluded() {
        return SourceAssessments {
            rules: source_rules::not_applicable(
                &policy.rules,
                policy.spacing,
                "Original source path is excluded by the configured workspace policy",
            ),
            exclusion: Some(exclusion),
            ..SourceAssessments::default()
        };
    }

    let mut rules = source_rules::evaluate(facts, &policy.rules, policy.spacing);
    let suppression = source_suppressions::evaluate(&facts.preceding_comment);
    rules.extend(suppression.invalid_assessment());
    SourceAssessments {
        rules,
        exclusion: Some(exclusion),
        suppression: Some(suppression),
        outside: false,
    }
}

fn inputs(function: &Function, exhausted: bool, statements: bool) -> (Execution, LocalFacts<'_>) {
    if exhausted {
        return (
            Execution::BudgetExhausted,
            unavailable("Analysis function budget exhausted"),
        );
    }
    let inspected = if statements {
        rstyle_pdg::consumer_facts::inspect_with_statements(&function.facts)
    } else {
        rstyle_pdg::consumer_facts::inspect(&function.facts)
    };
    match inspected {
        Ok(facts) => {
            let execution = match facts.execution {
                rstyle_pdg::consumer_facts::InspectionExecution::Completed => Execution::Completed,
                rstyle_pdg::consumer_facts::InspectionExecution::BudgetExhausted => {
                    Execution::BudgetExhausted
                }
            };
            (execution, facts)
        }
        Err(error) => (Execution::Failed, unavailable(&error)),
    }
}

fn unavailable(reason: &str) -> LocalFacts<'static> {
    LocalFacts {
        execution: rstyle_pdg::consumer_facts::InspectionExecution::Completed,
        top_statements: Availability::Unknown {
            reason: reason.into(),
        },
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
        changed_scope::permits_success(
            self.execution,
            self.scope.coverage,
            self.changed_scope.as_ref(),
        ) && self
            .guard_changed_scope
            .as_ref()
            .is_none_or(changed_scope::Report::permits_success)
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
        if let Some(changed) = &self.changed_scope {
            output.push_str(&changed.render());
        }
        if let Some(changed) = &self.guard_changed_scope {
            output.push_str(&changed.render_for("guard advice"));
        }
        output.push_str(
            &self
                .functions
                .iter()
                .map(|function| {
                    function.render(
                        self.changed_scope.is_none() || function.changed_file == Some(true),
                        self.guard_changed_scope.is_none() || function.changed_file == Some(true),
                    )
                })
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
        self.source_visible_findings() + usize::from(self.guard_finding())
    }

    fn source_visible_findings(&self) -> usize {
        self.assessments
            .iter()
            .filter(|rule| self.visible(rule))
            .count()
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

    fn render(&self, show_source: bool, show_guard: bool) -> String {
        let location = self.location();
        let source = self
            .assessments
            .iter()
            .filter(|assessment| {
                show_source
                    && self.visible(assessment)
                    && !self.source_review_hidden.contains(assessment.rule_id)
            })
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
            .filter(|_| show_guard)
            .into_iter()
            .flat_map(|guard| guard.opportunity.iter().chain(&guard.closure_opportunities))
            .enumerate()
            .filter(|(index, _)| !self.guard_review_hidden.contains(index))
            .map(|(_, opportunity)| {
                format!(
                    "\nguard_clause {location}: {} (fallback {:?}).",
                    opportunity.advisory, opportunity.fallback_branch
                )
            })
            .collect::<String>();
        format!("{source}{guard}")
    }
}

impl Report<'_> {
    pub(in crate::clones) fn guard_findings(
        &self,
    ) -> impl Iterator<Item = (super::FunctionKey, &guards::GuardOpportunity)> + '_ {
        self.functions.iter().flat_map(|function| {
            function.guard_clause.iter().flat_map(move |guard| {
                guard
                    .opportunity
                    .iter()
                    .chain(&guard.closure_opportunities)
                    .map(move |opportunity| (function.key, opportunity))
            })
        })
    }

    pub(in crate::clones) fn guard_population_complete(&self) -> bool {
        self.population_complete(|function| {
            function.guard_clause.as_ref().is_some_and(|guard| {
                guard.execution == Execution::Completed && guard.coverage == Coverage::Complete
            })
        })
    }

    pub(in crate::clones) fn guard_execution(&self) -> Execution {
        if self.scope.expected_functions.is_none()
            || self.functions.iter().any(|function| {
                function
                    .guard_clause
                    .as_ref()
                    .is_some_and(|guard| guard.execution == Execution::Failed)
            })
        {
            Execution::Failed
        } else if self.functions.iter().any(|function| {
            function
                .guard_clause
                .as_ref()
                .is_some_and(|guard| guard.execution == Execution::BudgetExhausted)
        }) {
            Execution::BudgetExhausted
        } else {
            Execution::Completed
        }
    }

    pub(in crate::clones) fn parameter_findings(
        &self,
    ) -> impl Iterator<Item = super::FunctionKey> + '_ {
        self.functions
            .iter()
            .filter(|function| {
                function.assessments.iter().any(|assessment| {
                    assessment.rule_id == "fn_params" && assessment.outcome == Outcome::Finding
                }) && !function
                    .source_suppression
                    .as_ref()
                    .is_some_and(|suppression| suppression.allows("fn_params"))
            })
            .map(|function| function.key)
    }

    pub(in crate::clones) fn parameter_population_complete(&self) -> bool {
        self.population_complete(|function| {
            function.execution == Execution::Completed && function.parameter_source_complete()
        })
    }

    fn population_complete(&self, completed: impl Fn(&FunctionReport<'_>) -> bool) -> bool {
        self.scope.expected_functions == Some(self.functions.len())
            && self.functions.iter().all(completed)
    }
}

pub(super) fn source_file(function: &Function) -> Option<&str> {
    function
        .facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
}
