//! Runtime evidence stays separate from abstraction proposals and their review identities.
use super::{Context, UnitId};
use crate::clones::FunctionKey;
use rstyle_core::{
    advisory::runtime_bounds::{Input, KEY, Observation},
    runtime_bounds::{self, Outcome},
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Report {
    schema: &'static str,
    pub input_complete: bool,
    limitations: &'static [&'static str],
    rows: Vec<Row>,
    selected: Vec<usize>,
}

#[derive(Serialize)]
struct Row {
    key: FunctionKey,
    name: String,
    path: String,
    #[serde(flatten)]
    evidence: Evidence,
}

#[derive(Serialize)]
struct Evidence {
    capture_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing: Option<String>,
}

pub(super) fn evaluate(ctx: &Context<'_>, unit: UnitId) -> Option<Report> {
    if !ctx.args.compiler_runtime_bounds {
        return None;
    }
    let mut rows: Vec<_> = ctx
        .handoff
        .assessed
        .iter()
        .filter(|function| function.key.unit() == unit)
        .filter(|function| {
            ctx.args.compiler_pattern_scope != crate::clones::CompilerSourceScope::Authored
                || !function
                    .facts
                    .view()
                    .source_diagnostics
                    .iter()
                    .any(|finding| finding.construct == "macro_generated_function")
        })
        .map(|function| row(function, &ctx.capture.plan.root))
        .collect();
    rows.sort_by_key(|row| (std::cmp::Reverse(degree(row)), row.key));
    Some(Report {
        schema: "rstyle.compiler-runtime-bounds-report/1",
        input_complete: rows.iter().all(|row| row.evidence.missing.is_none()),
        limitations: &[
            "Bounds count loop-body entries and modeled call-site invocations; whole runtime remains unknown.",
            "Compiler-resolved collection length and traversal adapters are abstracted, excluded from modeled call counts.",
            "An upper bound does not establish tightness; high polynomial degree alone is not repeated-scan evidence.",
        ],
        selected: (0..rows
            .len()
            .min(ctx.args.compiler_runtime_top.unwrap_or(usize::MAX)))
            .collect(),
        rows,
    })
}

fn row(function: &crate::clones::Function, root: &std::path::Path) -> Row {
    let path = super::source_path(function, root, true);
    let input = path
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|_| read(function));
    Row {
        key: function.key,
        name: function.facts.view().name.to_string(),
        path: path.unwrap_or_default(),
        evidence: evidence(input),
    }
}

fn evidence(input: Result<Observation, String>) -> Evidence {
    let observation = match input {
        Ok(observation) => observation,
        Err(reason) => {
            return Evidence {
                capture_status: "missing",
                inputs: None,
                result: None,
                missing: Some(reason),
            };
        }
    };
    let (capture_status, inputs, result) = match observation.input {
        Input::Captured { program } => {
            let result = runtime_bounds::analyze(&program);
            ("captured", Some(program.inputs), result)
        }
        Input::BudgetExhausted { reason } => {
            ("budget_exhausted", None, Outcome::BudgetExceeded { reason })
        }
        Input::Unavailable { reason } => ("unavailable", None, Outcome::Unknown { reason }),
    };
    Evidence {
        capture_status,
        inputs,
        result: Some(result),
        missing: None,
    }
}

fn read(function: &crate::clones::Function) -> Result<Observation, String> {
    let raw = function
        .facts
        .view()
        .build
        .frontend_options
        .get(KEY)
        .ok_or("Runtime-bound observations were not captured")?;
    let observation = Observation::read(raw)?;
    if observation.owner != owner(function)? {
        return Err("Runtime-bound observation disagrees with captured compiler owner".into());
    }
    Ok(observation)
}

impl Report {
    pub(super) fn render(&self, unit: UnitId) -> String {
        let rows: String = self
            .selected
            .iter()
            .map(|index| {
                let row = &self.rows[*index];
                format!("\n{}: {}: {}", row.path, row.name, render_result(row))
            })
            .collect();
        format!(
            "\nRuntime upper bounds for unit {} (loop-body entries and modeled call-site invocations; input complete={}):{rows}\nCollection length and traversal adapters are abstracted, excluded from call counts.\nUpper bounds do not establish tightness or repeated scans.\n",
            unit.ordinal(),
            self.input_complete
        )
    }
}

// Degree is a search heuristic, not an ordering over multivariate runtimes.
fn degree(row: &Row) -> usize {
    let Some(Outcome::Proven(assessment)) = &row.evidence.result else {
        return 0;
    };
    bound_degree(&assessment.loop_body_entries).saturating_add(1)
}

fn bound_degree(bound: &runtime_bounds::Bound) -> usize {
    use runtime_bounds::Bound;
    match bound {
        Bound::Const(_) => 0,
        Bound::Input(_) => 1,
        Bound::Add(values) | Bound::Max(values) => {
            values.iter().map(bound_degree).max().unwrap_or(0)
        }
        Bound::Mul(values) => values.iter().map(bound_degree).sum(),
    }
}

// collect_inputs validated compiler observations before this report is evaluated.
// Project only the owner identity here rather than rebuilding the entire facts graph.
fn owner(function: &crate::clones::Function) -> Result<String, String> {
    use rstyle_core::advisory::{self, generated_effects};
    let options = &function.facts.view().build.frontend_options;
    if let Some(id) = options
        .get(advisory::KEY)
        .and_then(|raw| raw["facts"]["id"].as_str())
        .filter(|id| !id.is_empty())
    {
        return Ok(id.to_owned());
    }
    let raw = options
        .get(generated_effects::CONNECTED_KEY)
        .ok_or("Runtime-bound owner identity is unavailable")?;
    let observation = generated_effects::Observation::read(raw)?;

    if observation.name != function.facts.view().name
        || !function
            .facts
            .view()
            .source_diagnostics
            .iter()
            .any(|finding| finding.construct == "macro_generated_function")
    {
        return Err("Runtime-bound generated owner identity disagrees with capture".into());
    }
    observation
        .connections
        .map(|connections| connections.id)
        .ok_or("Runtime-bound generated owner identity is unavailable".into())
}

fn render_result(row: &Row) -> String {
    match (&row.evidence.result, &row.evidence.missing) {
        (_, Some(reason)) => format!("missing: {reason}"),
        (Some(Outcome::Unknown { reason }), _) => format!("unknown: {reason}"),
        (Some(Outcome::BudgetExceeded { reason }), _) => format!("budget exhausted: {reason}"),
        (Some(Outcome::Proven(assessment)), _) => {
            let loops: String = assessment
                .loops
                .iter()
                .map(|item| {
                    let source = item.source.as_ref().map_or_else(String::new, |source| {
                        format!("{}:{}:{}: ", source.file, source.line, source.column)
                    });
                    format!(
                        "\n  {source}loop body entries <= {}",
                        expression(&item.body_entries)
                    )
                })
                .collect();
            let calls: String = assessment
                .calls
                .iter()
                .map(|item| {
                    format!(
                        "\n  {} invocations <= {}",
                        item.callee,
                        expression(&item.count)
                    )
                })
                .collect();
            format!(
                "loop body entries <= {}; whole runtime {}{loops}{calls}",
                expression(&assessment.loop_body_entries),
                assessment.whole_runtime
            )
        }
        _ => "missing result".into(),
    }
}

fn expression(bound: &runtime_bounds::Bound) -> String {
    use runtime_bounds::Bound;
    match bound {
        Bound::Const(value) => value.to_string(),
        Bound::Input(name) => name.clone(),
        Bound::Add(values) => values
            .iter()
            .map(expression)
            .collect::<Vec<_>>()
            .join(" + "),
        Bound::Mul(values) => values
            .iter()
            .map(|value| {
                let text = expression(value);
                if matches!(value, Bound::Add(_)) {
                    format!("({text})")
                } else {
                    text
                }
            })
            .collect::<Vec<_>>()
            .join(" * "),
        Bound::Max(values) => format!(
            "max({})",
            values.iter().map(expression).collect::<Vec<_>>().join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multivariate_expressions_preserve_sum_grouping() {
        use runtime_bounds::Bound::*;
        let bound = Mul(vec![
            Input("n".into()),
            Add(vec![Input("m".into()), Const(1)]),
        ]);
        assert_eq!(expression(&bound), "n * (m + 1)");
        assert_eq!(bound_degree(&bound), 2);
    }
}
