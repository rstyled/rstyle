//! Runtime evidence stays separate from abstraction proposals and their review identities.
use super::{Context, UnitId};
use crate::clones::FunctionKey;
use rstyle_core::{
    advisory::runtime_bounds::{Input, KEY, Observation},
    cpg::records::{Measured, Record},
    runtime_bounds::{
        self, Limits, Outcome,
        compose::{Analyzed, Composed, Composer},
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

#[derive(Serialize)]
struct Evidence {
    capture_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    composed: Option<Composed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing: Option<String>,
}

const LIMITATIONS: &[&str] = &[
    "Bounds count loop-body entries and modeled call-site invocations; whole runtime remains unknown.",
    "Compiler-resolved collection length and traversal adapters are abstracted, excluded from modeled call counts.",
    "Resolved standard iterator chains over stable extents or primitive ranges are synthetic scans; callbacks are inlined and counted at most once per pulled item.",
    "Partial results are sound bounds relative to opaque factors: ?loop(site) is the number of header executions of an unproven loop per entry of its enclosing loop, ?call and ?invocations the work and calls of an unsummarized callee; whole-owner failures stay unknown.",
    "Composed results add resolved workspace callee summaries; standard-library bodies are outside that cost unit, and recursive, unresolved or unmappable callees contribute opaque ?call or ?input factors.",
    "Library steps sum data contracts for keyed standard operations, such as log(len(map)) per B-tree lookup, up to each contract's constant factor; uncontracted operations contribute ?library factors and never change the composed verdict.",
    "An upper bound does not establish tightness; high polynomial degree alone is not repeated-scan evidence.",
];

pub(super) fn evaluate(ctx: &Context<'_>, unit: UnitId) -> Option<Report> {
    if !ctx.args.compiler_runtime_bounds {
        return None;
    }
    let index = owners(ctx);
    let lookup = |id: &str| analyzed(index.get(id).copied().flatten()?);
    let mut composer = Composer::new(lookup, Limits::default());

    let mut rows: Vec<_> = ctx
        .handoff
        .assessed
        .iter()
        .filter(|function| function.key.unit() == unit)
        .filter(|function| {
            ctx.args.compiler_pattern_scope != crate::clones::CompilerSourceScope::Authored
                || !function.facts.view().generated
        })
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

/// A function's bounds, as a run over documents reports them.
#[derive(Serialize)]
struct DocumentRow {
    name: String,
    path: String,
    #[serde(flatten)]
    evidence: Evidence,
}

/// Bounds of a run over documents, which names functions by path alone.
#[derive(Serialize)]
pub(crate) struct DocumentReport {
    schema: &'static str,
    input_complete: bool,
    limitations: &'static [&'static str],
    rows: Vec<DocumentRow>,
}

/// The bounds of every authored function of a run, highest degree first, with callees
/// resolved across all of its units.
pub(crate) fn from_records(records: &[Record], top: Option<usize>) -> DocumentReport {
    let mut index: HashMap<String, Option<&Record>> = HashMap::new();
    for record in records {
        index
            .entry(record_key(record))
            .and_modify(|entry| *entry = None)
            .or_insert(Some(record));
    }
    let lookup = |id: &str| record_analyzed(index.get(id).copied().flatten()?);
    let mut composer = Composer::new(lookup, Limits::default());
    let mut rows: Vec<_> = records
        .iter()
        .filter_map(|record| {
            let Measured::Known(facts) = &record.measured else {
                return None;
            };
            Some(DocumentRow {
                name: record.name.clone(),
                path: facts.path.clone(),
                evidence: record_evidence(record, &mut composer),
            })
        })
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(degree(&row.evidence)));
    rows.truncate(top.unwrap_or(usize::MAX));

    DocumentReport {
        schema: "rstyle.cpg-runtime-bounds-report/1",
        input_complete: rows.iter().all(|row| row.evidence.missing.is_none()),
        limitations: LIMITATIONS,
        rows,
    }
}

impl DocumentReport {
    pub(crate) fn text(&self) -> String {
        let rows: String = self
            .rows
            .iter()
            .map(|row| {
                format!(
                    "\n{}: {}: {}",
                    row.path,
                    row.name,
                    render_result(&row.evidence)
                )
            })
            .collect();
        format!(
            "Runtime upper bounds (loop-body entries and modeled call-site invocations; input complete={}):{rows}\nUpper bounds do not establish tightness or repeated scans.\n",
            self.input_complete
        )
    }
}

fn record_key(record: &Record) -> String {
    format!("{}/{}", record.compilation, record.identity)
}

fn record_analyzed(record: &Record) -> Option<Analyzed> {
    let Some(Input::Captured { program }) = record.bounds.clone() else {
        return None;
    };
    Some(Analyzed {
        outcome: runtime_bounds::analyze(&program),
        program,
    })
}

fn record_evidence<F: FnMut(&str) -> Option<Analyzed>>(
    record: &Record,
    composer: &mut Composer<F>,
) -> Evidence {
    let observation = Observation {
        schema: KEY.into(),
        owner: record.identity.clone(),
        krate: record.compilation.clone(),
        input: record.bounds.clone().unwrap_or(Input::Unavailable {
            reason: "Runtime bounds were not requested".into(),
        }),
    };
    let composed = matches!(observation.input, Input::Captured { .. })
        .then(|| composer.composed(&record_key(record)));
    Evidence {
        composed,
        ..evidence(Ok(observation))
    }
}

// Owners across every unit, keyed by compilation and path so callees in other
// workspace crates are reachable; a key captured twice is ambiguous and unavailable.
fn owners<'a>(ctx: &'a Context<'_>) -> HashMap<String, Option<&'a crate::clones::Function>> {
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

fn key(function: &crate::clones::Function) -> Option<String> {
    let krate = match function.facts.document() {
        Some(document) => {
            document.record.bounds.as_ref()?;
            document.record.compilation.as_str()
        }
        None => function.facts.recorded(KEY)?["krate"].as_str()?,
    };
    Some(format!("{krate}/{}", owner(function).ok()?))
}

fn analyzed(function: &crate::clones::Function) -> Option<Analyzed> {
    let Input::Captured { program } = read(function).ok()?.input else {
        return None;
    };
    Some(Analyzed {
        outcome: runtime_bounds::analyze(&program),
        program,
    })
}

fn row<F: FnMut(&str) -> Option<Analyzed>>(
    function: &crate::clones::Function,
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
            composer.composed(&format!("{}/{}", observation.krate, observation.owner))
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

fn evidence(input: Result<Observation, String>) -> Evidence {
    let observation = match input {
        Ok(observation) => observation,
        Err(reason) => {
            return Evidence {
                capture_status: "missing",
                inputs: None,
                result: None,
                composed: None,
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
        composed: None,
        missing: None,
    }
}

fn read(function: &crate::clones::Function) -> Result<Observation, String> {
    let observation = match function.facts.document() {
        Some(document) => Observation {
            schema: KEY.into(),
            owner: document.record.identity.clone(),
            krate: document.record.compilation.clone(),
            input: document
                .record
                .bounds
                .clone()
                .ok_or("Runtime-bound observations were not captured")?,
        },
        None => Observation::read(
            function
                .facts
                .recorded(KEY)
                .ok_or("Runtime-bound observations were not captured")?,
        )?,
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

// Order is a search heuristic, not an ordering over multivariate runtimes.
fn degree(evidence: &Evidence) -> (bool, runtime_bounds::Order) {
    let Some(Outcome::Proven(assessment) | Outcome::Partial(assessment)) = &evidence.result else {
        return (false, runtime_bounds::Order::default());
    };
    (true, assessment.loop_body_entries.order())
}

// collect_inputs validated compiler observations before this report is evaluated.
// Project only the owner identity here rather than rebuilding the entire facts graph.
fn owner(function: &crate::clones::Function) -> Result<String, String> {
    use rstyle_core::advisory::generated_effects;
    if let Some(document) = function.facts.document() {
        return document
            .compiled_id()
            .map(str::to_owned)
            .ok_or_else(|| "Runtime-bound owner identity is unavailable".into());
    }
    if let Some(id) = function.facts.compiled_id().filter(|id| !id.is_empty()) {
        return Ok(id.to_owned());
    }
    let raw = function
        .facts
        .recorded(generated_effects::CONNECTED_KEY)
        .ok_or("Runtime-bound owner identity is unavailable")?;
    let observation = generated_effects::Observation::read(raw)?;

    if observation.name != function.facts.view().name || !function.facts.view().generated {
        return Err("Runtime-bound generated owner identity disagrees with capture".into());
    }
    observation
        .connections
        .map(|connections| connections.id)
        .ok_or("Runtime-bound generated owner identity is unavailable".into())
}

fn render_result(evidence: &Evidence) -> String {
    match (&evidence.result, &evidence.missing) {
        (_, Some(reason)) => format!("missing: {reason}"),
        (Some(Outcome::Unknown { reason }), _) => format!("unknown: {reason}"),
        (Some(Outcome::BudgetExceeded { reason }), _) => format!("budget exhausted: {reason}"),
        (Some(Outcome::Proven(assessment)), _) => render_assessment(evidence, assessment, ""),
        (Some(Outcome::Partial(assessment)), _) => {
            render_assessment(evidence, assessment, "partial: ")
        }
        _ => "missing result".into(),
    }
}

fn render_assessment(
    evidence: &Evidence,
    assessment: &runtime_bounds::Assessment,
    label: &str,
) -> String {
    let loops: String = assessment.loops.iter().map(render_loop).collect();
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
        "{label}loop body entries <= {}; whole runtime {}{loops}{calls}{}",
        expression(&assessment.loop_body_entries),
        assessment.whole_runtime,
        render_composed(evidence.composed.as_ref())
    )
}

fn render_loop(item: &runtime_bounds::LoopBound) -> String {
    let source = item.source.as_ref().map_or_else(String::new, |source| {
        format!("{}:{}:{}: ", source.file, source.line, source.column)
    });
    let opaque = item.opaque.as_ref().map_or_else(String::new, |opaque| {
        format!(" (opaque: {})", opaque.reason)
    });
    format!(
        "\n  {source}loop body entries <= {}{opaque}",
        expression(&item.body_entries)
    )
}

fn render_composed(composed: Option<&Composed>) -> String {
    match composed {
        Some(Composed::Proven(composition)) => render_composition(composition, ""),
        Some(Composed::Partial(composition)) => render_composition(composition, " (partial)"),
        Some(Composed::Unknown { reason }) => format!("\n  composed: unknown: {reason}"),
        Some(Composed::BudgetExceeded { reason }) => {
            format!("\n  composed: budget exhausted: {reason}")
        }
        None => String::new(),
    }
}

fn render_composition(composition: &runtime_bounds::compose::Composition, label: &str) -> String {
    let factors = |factors: &[runtime_bounds::compose::OpaqueFactor]| -> String {
        factors
            .iter()
            .map(|factor| format!("\n    opaque {}: {}", factor.site, factor.reason))
            .collect()
    };
    format!(
        "\n  composed{label} workspace loop body entries <= {}; workspace invocations <= {}{}\n  library steps <= {} (contract orders){}",
        expression(&composition.loop_body_entries),
        expression(&composition.workspace_invocations),
        factors(&composition.opaque_factors),
        expression(&composition.library_steps),
        factors(&composition.library_factors),
    )
}

fn expression(bound: &runtime_bounds::Bound) -> String {
    bound.to_string()
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
        assert_eq!(bound.order().polynomial, 2);
    }

    // `while ? {}` at src/lib.rs:3:5: an unprovable loop in an otherwise empty owner.
    fn polling() -> runtime_bounds::Program {
        use runtime_bounds::*;
        let source = Source {
            file: "src/lib.rs".into(),
            line: 3,
            column: 5,
            evidence: "test".into(),
        };
        let node = |flow, source| Node {
            action: Action::Nop,
            flow,
            source,
        };
        Program {
            inputs: Vec::new(),
            variables: Vec::new(),
            nodes: vec![
                node(
                    Flow::Branch {
                        condition: Condition::Unknown,
                        then_node: 1,
                        else_node: 2,
                    },
                    Some(source),
                ),
                node(Flow::Goto(0), None),
                node(Flow::Return, None),
            ],
            entry: 0,
            loop_headers: vec![0],
            arithmetic: Arithmetic::Checked,
            formals: Vec::new(),
            extents: Vec::new(),
        }
    }

    fn observation(input: Input) -> Observation {
        Observation {
            schema: KEY.into(),
            owner: "w::poll".into(),
            krate: "k".into(),
            input,
        }
    }

    fn row(evidence: Evidence) -> Row {
        Row {
            key: "function:u0:f1".parse().unwrap(),
            name: "poll".into(),
            path: "src/lib.rs".into(),
            evidence,
        }
    }

    #[test]
    fn unavailable_budget_and_missing_inputs_keep_their_status() {
        let missing = evidence(Err("not captured".into()));
        assert_eq!(missing.capture_status, "missing");
        assert_eq!(
            render_result(&row(missing).evidence),
            "missing: not captured"
        );

        let reason = "recorded".to_string();
        let unavailable = evidence(Ok(observation(Input::Unavailable {
            reason: reason.clone(),
        })));
        assert_eq!(
            render_result(&row(unavailable).evidence),
            "unknown: recorded"
        );
        let budget = evidence(Ok(observation(Input::BudgetExhausted { reason })));
        assert_eq!(
            render_result(&row(budget).evidence),
            "budget exhausted: recorded"
        );
    }

    #[test]
    fn partial_rows_render_opaque_loops_and_compositions() {
        let program = polling();
        let mut composer = Composer::new(
            |_: &str| {
                Some(Analyzed {
                    outcome: runtime_bounds::analyze(&program),
                    program: program.clone(),
                })
            },
            Limits::default(),
        );
        let captured = evidence(Ok(observation(Input::Captured { program: polling() })));
        assert!(matches!(captured.result, Some(Outcome::Partial(_))));
        let mut partial = row(Evidence {
            composed: Some(composer.composed("w::poll")),
            ..captured
        });
        let order = degree(&partial.evidence);
        assert_eq!(order, (true, runtime_bounds::Order::default()));

        let text = render_result(&partial.evidence);
        assert!(
            text.starts_with("partial: loop body entries <= ?loop(src/lib.rs:3:5#0)"),
            "{text}"
        );
        assert!(
            text.contains("(opaque: Loop requires an integer less-than guard)"),
            "{text}"
        );
        assert!(
            text.contains("composed (partial) workspace loop body entries"),
            "{text}"
        );
        assert!(
            text.contains("opaque src/lib.rs:3:5#0: Loop requires"),
            "{text}"
        );

        partial.evidence.composed = Some(Composed::Unknown { reason: "x".into() });
        assert!(render_result(&partial.evidence).ends_with("composed: unknown: x"));
        partial.evidence.composed = Some(Composed::BudgetExceeded { reason: "y".into() });
        assert!(render_result(&partial.evidence).ends_with("composed: budget exhausted: y"));

        let report = Report {
            schema: "test",
            input_complete: true,
            limitations: &[],
            rows: vec![partial],
            selected: vec![0],
        };
        let key: FunctionKey = "function:u0:f1".parse().unwrap();
        let text = report.render(key.unit());
        assert!(text.contains("src/lib.rs: poll: partial:"), "{text}");
    }

    #[test]
    fn opaque_factors_render_by_kind_without_raising_degree() {
        use runtime_bounds::{Bound, OpaqueKind};
        let bound = Bound::Mul(vec![
            Bound::opaque(OpaqueKind::Call, "w::f"),
            Bound::opaque(OpaqueKind::Input, "w::f::n"),
            Bound::Input("m".into()),
        ]);
        assert_eq!(expression(&bound), "?call(w::f) * ?input(w::f::n) * m");
        assert_eq!(bound.order().polynomial, 1);
    }
}
