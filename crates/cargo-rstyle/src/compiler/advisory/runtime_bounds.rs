//! Runtime evidence stays separate from abstraction proposals and their review identities.
use super::{Context, UnitId};
use crate::compiler::FunctionKey;
use rstyle_core::wording::Wording;
use rstyle_core::{
    advisory::runtime_bounds::{Input, KEY, Observation},
    cpg::bounds::report::{Evidence, LIMITATIONS, degree, evidence, render_result},
    runtime_bounds::{
        self, Limits,
        compose::{Analyzed, Composer},
    },
};
use serde::Serialize;
use std::collections::HashMap;

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

pub(super) fn evaluate(ctx: &Context<'_>, unit: UnitId) -> Option<Report> {
    if !ctx.args.compiler_runtime_bounds {
        return None;
    }
    let index = owners(ctx);
    let lookup = |id: &str| analyzed(index.get(id).copied().flatten()?);
    let mut composer = Composer::new(lookup, Limits::default());

    let mut assessed: Vec<_> = ctx
        .handoff
        .assessed
        .iter()
        .filter(|function| function.key.unit() == unit)
        .filter(|function| {
            ctx.args.compiler_pattern_scope != crate::compiler::CompilerSourceScope::Authored
                || !function.facts.view().generated
        })
        .collect();
    // Composing shares what it has worked out, so visit functions as a run over documents does.
    assessed.sort_by_key(|function| function.facts.document().record.canonical_key());

    let mut rows: Vec<_> = assessed
        .into_iter()
        .map(|function| row(function, &ctx.capture.plan.root, &mut composer))
        .collect();
    rows.sort_by_key(|row| (std::cmp::Reverse(degree(&row.evidence)), row.key));
    Some(Report {
        schema: "rstyle.compiler-runtime-bounds-report/2",
        input_complete: rows.iter().all(|row| row.evidence.missing.is_none()),
        limitations: LIMITATIONS,
        selected: (0..rows
            .len()
            .min(ctx.args.compiler_runtime_top.unwrap_or(usize::MAX)))
            .collect(),
        rows,
    })
}

// Owners across every unit, keyed by compilation and path so callees in other
// workspace crates are reachable; a key captured twice is ambiguous and unavailable.
fn owners<'a>(ctx: &'a Context<'_>) -> HashMap<String, Option<&'a crate::compiler::Function>> {
    let mut index = HashMap::new();
    for function in &ctx.handoff.assessed {
        if let Some(key) = key(function) {
            index
                .entry(key)
                .and_modify(|entry| *entry = None)
                .or_insert(Some(function));
        }
    }
    index
}

fn key(function: &crate::compiler::Function) -> Option<String> {
    let document = function.facts.document();
    document.record.bounds.as_ref()?;
    Some(format!(
        "{}/{}",
        document.record.compilation,
        owner(function).ok()?
    ))
}

fn analyzed(function: &crate::compiler::Function) -> Option<Analyzed> {
    let Input::Captured { program } = read(function).ok()?.input else {
        return None;
    };
    Some(Analyzed {
        outcome: runtime_bounds::analyze(&program),
        program,
    })
}

fn row<F: FnMut(&str) -> Option<Analyzed>>(
    function: &crate::compiler::Function,
    root: &std::path::Path,
    composer: &mut Composer<F>,
) -> Row {
    let path = super::source_path(function, root, true);
    let input = path
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|_| read(function));
    let composed = input
        .as_ref()
        .ok()
        .filter(|observation| matches!(observation.input, Input::Captured { .. }))
        .map(|observation| {
            let owner = format!("{}/{}", observation.krate, observation.owner);
            composer.composed(&owner, Wording::Rust)
        });
    Row {
        key: function.key,
        name: function.facts.view().name.to_string(),
        path: path.unwrap_or_default(),
        evidence: Evidence {
            composed,
            ..evidence(input)
        },
    }
}

fn read(function: &crate::compiler::Function) -> Result<Observation, String> {
    let document = function.facts.document();
    let observation = Observation {
        schema: KEY.into(),
        owner: document.record.identity.clone(),
        krate: document.record.compilation.clone(),
        input: document
            .record
            .bounds
            .clone()
            .ok_or("Runtime-bound observations were not captured")?,
    };
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
                format!(
                    "\n{}: {}: {}",
                    row.path,
                    row.name,
                    render_result(&row.evidence)
                )
            })
            .collect();
        format!(
            "\nRuntime upper bounds for unit {} (loop-body entries and modeled call-site invocations; input complete={}):{rows}\nCollection length and traversal adapters are abstracted, excluded from call counts; iterator callbacks are inlined.\nUpper bounds do not establish tightness or repeated scans.\n",
            unit.ordinal(),
            self.input_complete
        )
    }
}

// collect_inputs validated compiler observations before this report is evaluated.
// Project only the owner identity here rather than rebuilding the entire facts graph.
fn owner(function: &crate::compiler::Function) -> Result<String, String> {
    function
        .facts
        .document()
        .compiled_id()
        .map(str::to_owned)
        .ok_or_else(|| "Runtime-bound owner identity is unavailable".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_renders_its_selected_rows_under_its_unit() {
        let observation = Observation {
            schema: KEY.into(),
            owner: "w::poll".into(),
            krate: "k".into(),
            input: Input::Unavailable {
                reason: "recorded".into(),
            },
        };
        let row = Row {
            key: "function:u0:f1".parse().unwrap(),
            name: "poll".into(),
            path: "src/lib.rs".into(),
            evidence: evidence(Ok(observation)),
        };
        let report = Report {
            schema: "test",
            input_complete: true,
            limitations: &[],
            rows: vec![row],
            selected: vec![0],
        };
        let key: FunctionKey = "function:u0:f1".parse().unwrap();
        let text = report.render(key.unit());
        assert!(
            text.contains("src/lib.rs: poll: unknown: recorded"),
            "{text}"
        );
    }
}
