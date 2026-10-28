//! Opt-in local consumers with evidence, execution and scope reported separately.
mod source_rules;

use super::{Args, Function, Handoff, capture};
use rstyle_core::config::{Config, FnLengthConfig, FnParamsConfig};
use rstyle_pdg::consumer_facts::{Availability, LocalFacts};
use serde::{Deserialize, Serialize};

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
    pub reasons: Vec<String>,
}

#[derive(Serialize)]
pub struct Report<'a> {
    schema: &'static str,
    pub execution: Execution,
    pub scope: Scope,
    pub findings: usize,
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

pub(super) fn configuration(args: &Args) -> Result<Option<Config>, String> {
    if !args.source_rules {
        return Ok(None);
    }
    let local: SourceConfig = match &args.analysis_config {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?
        }
        None => SourceConfig::default(),
    };
    Ok(Some(Config {
        fn_length: local.fn_length,
        fn_params: local.fn_params,
        ..Config::default()
    }))
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

    Report {
        schema: "rstyle.consumer-analysis/1",
        execution,
        scope,
        findings,
        effective_config: config.map(|config| Settings {
            max_lines: config.fn_length.max_lines,
            max_params: config.fn_params.max_params,
            count_self: config.fn_params.count_self,
        }),
        limitations: &[
            "Source rules do not establish body effects or refactoring safety.",
            "Length uses the legacy code-line policy, including its string/comment limitations.",
            "Legacy exclusions, suppressions and review ledgers are not applied by this opt-in consumer.",
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
    if unavailable_rules > 0 {
        reasons.push(
            "Some requested rules could not be evaluated; see per-function evidence reasons".into(),
        );
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
        reasons,
    }
}

fn count(functions: &[FunctionReport<'_>], outcome: Outcome) -> usize {
    functions
        .iter()
        .flat_map(|function| &function.assessments)
        .filter(|assessment| assessment.outcome == outcome)
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
    let (execution, assessments) = if exhausted {
        (
            Execution::BudgetExhausted,
            unavailable(config, "Analysis function budget exhausted"),
        )
    } else {
        match rstyle_pdg::consumer_facts::inspect(&function.facts) {
            Ok(facts) => (Execution::Completed, source_rules::evaluate(&facts, config)),
            Err(error) => (Execution::Failed, unavailable(config, &error)),
        }
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
    }
}

fn unavailable(config: &Config, reason: &str) -> Vec<Assessment> {
    source_rules::evaluate(
        &LocalFacts {
            source: Availability::Unknown {
                reason: reason.into(),
            },
            signature: Availability::Unknown {
                reason: reason.into(),
            },
        },
        config,
    )
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
            "\nSource rules: {:?}; {:?} coverage; {} findings; {} evaluated, {} unavailable rules.",
            self.execution,
            self.scope.coverage,
            self.findings,
            self.scope.evaluated_rules,
            self.scope.unavailable_rules
        );
        for function in &self.functions {
            for assessment in &function.assessments {
                if assessment.outcome == Outcome::Finding {
                    output.push_str(&format!(
                        "\n{} {} at {} [bytes {}..{}]: {} > {}.",
                        assessment.rule_id,
                        function.name,
                        function.source_file.unwrap_or("<source unavailable>"),
                        function.fn_span.start,
                        function.fn_span.end,
                        assessment.actual.unwrap_or(0),
                        assessment.limit
                    ));
                }
            }
        }
        output
    }
}
