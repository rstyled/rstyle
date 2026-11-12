//! What a run's runtime bounds report, for any route: each function's bound, composed through
//! its callees, and how it reads as text. Bounds count loop-body entries and modeled call-site
//! invocations; they don't bound whole runtime.
use crate::{
    advisory::runtime_bounds::{Input, KEY, Observation},
    cpg::records::{Measured, Record},
    runtime_bounds::{
        self, Limits, Outcome,
        compose::{Analyzed, Composed, Composer},
    },
    suppress,
};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Serialize)]
pub struct Evidence {
    pub capture_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composed: Option<Composed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<String>,
}

pub const LIMITATIONS: &[&str] = &[
    "Bounds count loop-body entries and modeled call-site invocations; whole runtime remains unknown.",
    "Compiler-resolved collection length and traversal adapters are abstracted, excluded from modeled call counts.",
    "Resolved standard iterator chains over stable extents or primitive ranges are synthetic scans; callbacks are inlined and counted at most once per pulled item.",
    "Partial results are sound bounds relative to opaque factors: ?loop(site) is the number of header executions of an unproven loop per entry of its enclosing loop, ?call and ?invocations the work and calls of an unsummarized callee; whole-owner failures stay unknown.",
    "Composed results add resolved workspace callee summaries; standard-library bodies are outside that cost unit, and recursive, unresolved or unmappable callees contribute opaque ?call or ?input factors.",
    "Library steps sum data contracts for keyed standard operations, such as log(len(map)) per B-tree lookup, up to each contract's constant factor; uncontracted operations contribute ?library factors and never change the composed verdict.",
    "An upper bound does not establish tightness; high polynomial degree alone is not repeated-scan evidence.",
];

/// The rule a superlinear bound is reported under, and what `rstyle-allow` names to silence it.
pub const RULE: &str = "algorithmic_complexity";

/// A function's bounds, as a run over documents reports them.
#[derive(Serialize)]
struct DocumentRow {
    name: String,
    path: String,
    #[serde(flatten)]
    evidence: Evidence,
    /// An `rstyle-allow` above the function names `RULE`. Its bound stays in this report.
    #[serde(skip)]
    allowed: bool,
}

/// Bounds of a run over documents, which names functions by path alone.
#[derive(Serialize)]
pub struct DocumentReport {
    schema: &'static str,
    input_complete: bool,
    limitations: &'static [&'static str],
    rows: Vec<DocumentRow>,
}

/// The bounds of every authored function of a run, highest degree first, with callees
/// resolved across all of its units.
pub fn from_records(records: &[Record], top: Option<usize>) -> DocumentReport {
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
                allowed: suppress::allows(&facts.preceding_line, RULE),
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
    pub fn text(&self) -> String {
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

/// A function whose proven loop bound grows faster than linearly, as SARIF reports it.
/// `repeated` names an input the bound multiplies by itself, as nested scans of one
/// collection do. The bound is an upper bound: it doesn't say every iteration runs.
pub struct Superlinear<'a> {
    pub name: &'a str,
    pub path: &'a str,
    /// The outermost loop's line, or 0 when no loop has a source.
    pub line: usize,
    pub bound: String,
    pub repeated: Option<String>,
}

impl DocumentReport {
    /// Proven bounds only: a partial bound's opaque factors could be anything. A function that
    /// allows the rule is left out.
    pub fn superlinear(&self) -> impl Iterator<Item = Superlinear<'_>> {
        self.rows.iter().filter_map(DocumentRow::superlinear)
    }
}

impl DocumentRow {
    fn superlinear(&self) -> Option<Superlinear<'_>> {
        if self.allowed {
            return None;
        }
        let Some(Outcome::Proven(assessment)) = &self.evidence.result else {
            return None;
        };
        let bound = &assessment.loop_body_entries;
        if bound.order().polynomial < 2 {
            return None;
        }
        let lines = assessment
            .loops
            .iter()
            .filter_map(|item| item.source.as_ref());
        Some(Superlinear {
            name: &self.name,
            path: &self.path,
            line: lines.map(|source| source.line).min().unwrap_or(0),
            bound: readable(&expression(bound)),
            repeated: repeated_input(bound).map(|input| readable(&input)),
        })
    }
}

/// Drops the `#0` suffixes frontends add to tell inputs apart, so `len(items)#0` reads
/// `len(items)`.
fn readable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let numbered = c == '#' && chars.peek().is_some_and(char::is_ascii_digit);
        if !numbered {
            out.push(c);
            continue;
        }
        while chars.next_if(char::is_ascii_digit).is_some() {}
    }
    out
}

/// An input that a product multiplies by itself, such as `n` in `n * (n + 1)`.
fn repeated_input(bound: &runtime_bounds::Bound) -> Option<String> {
    use runtime_bounds::Bound::*;
    match bound {
        Mul(items) => {
            let mut seen = BTreeSet::new();
            let squared = items
                .iter()
                .flat_map(|item| polynomial_inputs(item).into_iter())
                .find(|name| !seen.insert(name.clone()));
            squared.or_else(|| items.iter().find_map(repeated_input))
        }
        Add(items) | Max(items) => items.iter().find_map(repeated_input),
        Const(_) | Input(_) | Opaque(_) | Log(_) => None,
    }
}

/// The inputs a factor grows with at least linearly. A logarithm grows slower, so it's left out.
fn polynomial_inputs(bound: &runtime_bounds::Bound) -> BTreeSet<String> {
    use runtime_bounds::Bound::*;
    match bound {
        Input(name) => BTreeSet::from([name.clone()]),
        Add(items) | Max(items) | Mul(items) => items.iter().flat_map(polynomial_inputs).collect(),
        Const(_) | Opaque(_) | Log(_) => BTreeSet::new(),
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
        .then(|| composer.composed(&record_key(record), record.wording));
    Evidence {
        composed,
        ..evidence(Ok(observation))
    }
}

pub fn evidence(input: Result<Observation, String>) -> Evidence {
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

// Order is a search heuristic, not an ordering over multivariate runtimes.
pub fn degree(evidence: &Evidence) -> (bool, runtime_bounds::Order) {
    let Some(Outcome::Proven(assessment) | Outcome::Partial(assessment)) = &evidence.result else {
        return (false, runtime_bounds::Order::default());
    };
    (true, assessment.loop_body_entries.order())
}

pub fn render_result(evidence: &Evidence) -> String {
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

    fn row(outcome: Outcome) -> DocumentRow {
        DocumentRow {
            name: "dedupe".into(),
            path: "src/lib.rs".into(),
            evidence: Evidence {
                capture_status: "captured",
                inputs: None,
                result: Some(outcome),
                composed: None,
                missing: None,
            },
            allowed: false,
        }
    }

    fn assessment(bound: runtime_bounds::Bound, lines: &[usize]) -> runtime_bounds::Assessment {
        let loop_at = |line: &usize| runtime_bounds::LoopBound {
            header: 0,
            source: Some(runtime_bounds::Source {
                file: "src/lib.rs".into(),
                line: *line,
                column: 5,
                evidence: "test".into(),
            }),
            body_entries: bound.clone(),
            opaque: None,
        };
        runtime_bounds::Assessment {
            cost_scope: "test".into(),
            loop_body_entries: bound.clone(),
            loops: lines.iter().map(loop_at).collect(),
            calls: Vec::new(),
            whole_runtime: "unknown".into(),
        }
    }

    fn report(rows: Vec<DocumentRow>) -> DocumentReport {
        DocumentReport {
            schema: "test",
            input_complete: true,
            limitations: LIMITATIONS,
            rows,
        }
    }

    #[test]
    fn readable_drops_input_numbers() {
        assert_eq!(
            readable("len(items)#0 + len(items)#0 * len(items)#12"),
            "len(items) + len(items) * len(items)"
        );
        assert_eq!(readable("a#b"), "a#b");
    }

    #[test]
    fn only_proven_superlinear_bounds_are_reported_at_the_outer_loop() {
        use runtime_bounds::Bound::*;
        let squared = Mul(vec![Input("n".into()), Input("n".into())]);
        let proven = report(vec![row(Outcome::Proven(assessment(
            squared.clone(),
            &[9, 7],
        )))]);
        let found: Vec<_> = proven.superlinear().collect();
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].line, found[0].bound.as_str()), (7, "n * n"));
        assert_eq!(found[0].repeated.as_deref(), Some("n"));

        let partial = report(vec![row(Outcome::Partial(assessment(squared, &[7])))]);
        let linear = report(vec![row(Outcome::Proven(assessment(
            Input("n".into()),
            &[7],
        )))]);
        assert_eq!(partial.superlinear().count(), 0);
        assert_eq!(linear.superlinear().count(), 0);
    }

    #[test]
    fn a_superlinear_bound_is_a_result_of_the_check_log() {
        use runtime_bounds::Bound::*;
        let squared = Mul(vec![Input("n".into()), Input("n".into())]);
        let bounds = report(vec![row(Outcome::Proven(assessment(squared, &[7])))]);
        let root = std::path::Path::new(if cfg!(windows) { r"C:\w" } else { "/w" });
        let log = |levels: &str| {
            let config = crate::config::Config::parse(levels).unwrap();
            let bounds = Some((&bounds, &config.algorithmic_complexity));
            let log = crate::sarif::check(&Default::default(), bounds, root).unwrap();
            serde_json::to_value(log).unwrap()
        };

        let results = &log("")["runs"][0]["results"];
        assert_eq!(results.as_array().unwrap().len(), 1);
        assert_eq!(results[0]["ruleId"], RULE);
        assert_eq!(results[0]["level"], "error");
        let lowered = log("[algorithmic_complexity]\nseverity_level = \"warning\"");
        assert_eq!(lowered["runs"][0]["results"][0]["level"], "warning");
    }

    #[test]
    fn a_function_that_allows_the_rule_keeps_its_bound_and_reports_nothing() {
        use runtime_bounds::Bound::*;
        let squared = Mul(vec![Input("n".into()), Input("n".into())]);
        let mut allowed = row(Outcome::Proven(assessment(squared, &[7])));
        allowed.allowed =
            suppress::allows("// rstyle-allow(algorithmic_complexity): small n", RULE);

        assert!(allowed.allowed);
        assert!(allowed.superlinear().is_none());
        assert!(serde_json::to_value(&allowed).unwrap()["result"].is_object());
    }

    #[test]
    fn a_product_that_squares_one_input_names_it() {
        use runtime_bounds::Bound::*;
        let n = || Input("n".into());
        let m = || Input("m".into());
        let squared = |items| repeated_input(&Mul(items));
        assert_eq!(squared(vec![n(), n()]), Some("n".into()));
        assert_eq!(
            squared(vec![n(), Add(vec![n(), Const(1)])]),
            Some("n".into())
        );
        assert_eq!(squared(vec![m(), Mul(vec![n(), n()])]), Some("n".into()));
        assert_eq!(squared(vec![n(), m()]), None);
        assert_eq!(squared(vec![n(), Log(Box::new(n()))]), None);
        assert_eq!(
            repeated_input(&Add(vec![Mul(vec![n(), n()]), m()])),
            Some("n".into())
        );
    }

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

    #[test]
    fn unavailable_budget_and_missing_inputs_keep_their_status() {
        let missing = evidence(Err("not captured".into()));
        assert_eq!(missing.capture_status, "missing");
        assert_eq!(render_result(&missing), "missing: not captured");

        let reason = "recorded".to_string();
        let unavailable = evidence(Ok(observation(Input::Unavailable {
            reason: reason.clone(),
        })));
        assert_eq!(render_result(&unavailable), "unknown: recorded");
        let budget = evidence(Ok(observation(Input::BudgetExhausted { reason })));
        assert_eq!(render_result(&budget), "budget exhausted: recorded");
    }

    #[test]
    fn partial_results_render_opaque_loops_and_compositions() {
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
        let mut partial = Evidence {
            composed: Some(composer.composed("w::poll", crate::wording::Wording::of("rust"))),
            ..captured
        };
        let order = degree(&partial);
        assert_eq!(order, (true, runtime_bounds::Order::default()));

        let text = render_result(&partial);
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

        partial.composed = Some(Composed::Unknown { reason: "x".into() });
        assert!(render_result(&partial).ends_with("composed: unknown: x"));
        partial.composed = Some(Composed::BudgetExceeded { reason: "y".into() });
        assert!(render_result(&partial).ends_with("composed: budget exhausted: y"));
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
