//! Input reaching where it's dangerous: a value from a source, through what functions do with it,
//! to an argument a sink treats as more than data. Which calls those are is data, per frontend
//! namespace, and a project adds its own. Each function's summary says which of its parameters reach
//! a sink or what it returns, so a value can be followed across a run with no document in hand.
//!
//! A path says what to look at, not that it can be exploited, and no path doesn't say there's none.

mod builder;

use self::builder::ShellCode;
use super::Defect;
use super::value_flow::{End, FlowCall, Start, ValueFlows};
use crate::config::TaintConfig;
use crate::versioned::ByNamespace;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const RULE: &str = "taint_flow";

const SCHEMA: &str = "rstyle.taint-models/1";
/// Model files compiled into the engine, one per frontend namespace.
const EMBEDDED: &[&str] = &[
    include_str!("../contracts/taint/go.toml"),
    include_str!("../contracts/taint/cxx.toml"),
    include_str!("../contracts/taint/rust.toml"),
];

/// How a shell is told to take its next argument as code.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Shell {
    executables: Vec<String>,
    flag: String,
}

/// Calls made on what a sink returns that add arguments after its own, as a builder's do: each of
/// `one` adds what it's given, and each of `each` every element of a list written out for it.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Appends {
    #[serde(default)]
    one: Vec<String>,
    #[serde(default)]
    each: Vec<String>,
}

/// Where a value is dangerous.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sink {
    risk: String,
    cwe: String,
    arguments: Vec<usize>,
    callees: Vec<String>,
    shell: Option<Shell>,
    #[serde(default)]
    appends: Appends,
}

/// Where input comes from and goes, for one frontend namespace.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Models {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    namespace: String,
    #[serde(default)]
    sources: Vec<String>,
    #[serde(default)]
    sinks: Vec<Sink>,
}

impl Models {
    /// The models of a frontend namespace; none for a namespace without a file.
    pub fn of(namespace: &str) -> &'static Models {
        static MODELS: ByNamespace<Models> = ByNamespace::empty();
        MODELS.of(namespace, (EMBEDDED, parse))
    }
}

fn parse(text: &str) -> Result<(String, Models), String> {
    let models: Models =
        crate::versioned::parse(text, (SCHEMA, "taint models"), |models: &Models| {
            &models.schema
        })?;
    Ok((models.namespace.clone(), models))
}

/// Whether a key names this call: by its operation, its name or its identity.
fn names(key: &str, call: &FlowCall) -> bool {
    let operation = call.operation.as_deref();
    let without_namespace = operation.and_then(|operation| operation.split_once(':'));
    key == call.name
        || key == call.identity
        || operation == Some(key)
        || without_namespace.is_some_and(|(_, operation)| operation == key)
}

/// The base of a path, which is how a program names a shell.
fn program(path: &str) -> Option<&str> {
    std::path::Path::new(path).file_name()?.to_str()
}

/// Whether a known shell is told to run the argument after its flag, which is code to it, given the
/// text of each argument that's a literal. What isn't a literal can't establish that.
fn shell_code_at(shell: &Shell, program_at: usize, literals: &[(usize, String)]) -> Option<usize> {
    let literal = |at: usize| {
        let found = literals.iter().find(|(position, _)| *position == at);
        found.map(|(_, text)| text.as_str())
    };
    let known = shell
        .executables
        .iter()
        .any(|name| Some(name.as_str()) == literal(program_at).and_then(program));
    (known && literal(program_at + 1) == Some(shell.flag.as_str())).then_some(program_at + 2)
}

/// What a sink means by a dangerous argument, at this call.
fn dangerous(sink: &Sink, call: &FlowCall, position: usize) -> bool {
    let shell_code = sink.shell.as_ref().zip(sink.arguments.first());
    sink.arguments.contains(&position)
        || shell_code
            .is_some_and(|(shell, &at)| shell_code_at(shell, at, &call.literals) == Some(position))
}

/// A sink's risk and what it's called, wherever it's defined.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Risk {
    risk: String,
    cwe: Option<String>,
}

impl Risk {
    fn text(&self) -> String {
        match &self.cwe {
            Some(cwe) => format!("{}, {cwe}", self.risk),
            None => self.risk.clone(),
        }
    }
}

/// What a project and a frontend's models say of calls.
struct Reader<'a> {
    project: &'a TaintConfig,
}

impl Reader<'_> {
    fn counts(&self, call: &FlowCall) -> bool {
        !self.project.ignored.iter().any(|key| names(key, call))
    }

    fn is_source(&self, models: &Models, call: &FlowCall) -> bool {
        let keys = models.sources.iter().chain(&self.project.sources);
        self.counts(call) && keys.into_iter().any(|key| names(key, call))
    }

    fn is_sanitizer(&self, call: &FlowCall) -> bool {
        self.project.sanitizers.iter().any(|key| names(key, call))
    }

    fn risk_at(&self, models: &Models, call: &FlowCall, position: usize) -> Option<Risk> {
        if !self.counts(call) {
            return None;
        }
        let modeled = models
            .sinks
            .iter()
            .filter(|sink| sink.callees.iter().any(|key| names(key, call)))
            .find(|sink| dangerous(sink, call, position))
            .map(|sink| Risk {
                risk: sink.risk.clone(),
                cwe: Some(sink.cwe.clone()),
            });
        modeled.or_else(|| {
            let sinks = self.project.sinks.iter();
            let named = sinks
                .filter(|sink| sink.callees.iter().any(|key| names(key, call)))
                .find(|sink| sink.arguments.contains(&position));
            named.map(|sink| Risk {
                risk: sink.risk.clone(),
                cwe: sink.cwe.clone(),
            })
        })
    }
}

/// Where input came from, for whoever has to read a result.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Origin {
    name: String,
    path: String,
    line: usize,
}

/// A place a value turns dangerous, in whichever function holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SinkSite {
    function: usize,
    line: usize,
    name: String,
    risk: Risk,
}

/// What a function does with what it's given, as its callers see it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Summary {
    /// Input it gives back from a source of its own.
    returns_input: Option<Origin>,
    /// Which parameters reach what it returns.
    passes: BTreeSet<usize>,
    /// Which parameters reach a sink, and which one.
    sinks: BTreeMap<usize, SinkSite>,
}

/// One function's flows, and what a report says of where it is.
pub struct FunctionFlows {
    pub identity: String,
    pub path: String,
    /// Whether a comment above it silences this rule.
    pub allowed: bool,
    pub flows: ValueFlows,
}

/// Something a value reached.
pub struct Finding {
    /// Which of the functions given holds the sink.
    pub function: usize,
    pub defect: Defect,
}

/// An argument a value reached: the call, the position, and the element of a list written out there.
type Reached = (usize, usize, Option<usize>);

/// What a value does as it goes through one function.
#[derive(Default)]
struct Walk {
    arguments: BTreeSet<Reached>,
    returned: bool,
}

struct Solver<'a> {
    functions: &'a [FunctionFlows],
    reader: Reader<'a>,
    /// Whose body each identity is.
    by_identity: BTreeMap<&'a str, usize>,
    summaries: Vec<Summary>,
    /// Where each function's commands put what a shell runs as code.
    shell_code: Vec<ShellCode>,
}

impl Solver<'_> {
    fn callee_summary(&self, call: &FlowCall) -> Option<&Summary> {
        let &at = self.by_identity.get(call.selected.as_str())?;
        Some(&self.summaries[at])
    }

    /// A call gives back what it was handed, unless its body says otherwise or a project says it
    /// cleans it. A call with no body here can't be seen into.
    fn passes(&self, call: &FlowCall, position: usize) -> bool {
        !self.reader.is_sanitizer(call)
            && self
                .callee_summary(call)
                .is_none_or(|summary| summary.passes.contains(&position))
    }

    /// Everything these starts reach in one function, through the calls that pass them on.
    fn walk(&self, flows: &ValueFlows, starts: &[Start]) -> Walk {
        let mut walk = Walk::default();
        let mut seen: BTreeSet<Start> = starts.iter().copied().collect();
        let mut stack = starts.to_vec();

        while let Some(start) = stack.pop() {
            for hop in flows.hops.iter().filter(|hop| hop.from == start) {
                let End::Argument {
                    call,
                    position,
                    element,
                } = hop.to
                else {
                    walk.returned = true;
                    continue;
                };
                walk.arguments.insert((call, position, element));
                let next = Start::Result(call);
                if self.passes(&flows.calls[call], position) && seen.insert(next) {
                    stack.push(next);
                }
            }
        }
        walk
    }

    /// The sink an argument reaches, here or in the callee's body.
    fn sink_at(&self, at: usize, reached: Reached) -> Option<SinkSite> {
        let (call, position, _) = reached;
        let flows = &self.functions[at].flows;
        let called = &flows.calls[call];
        let models = Models::of(&flows.namespace);

        let code = self.shell_code[at].get(&reached);
        let code = code.filter(|_| self.reader.counts(called)).cloned();
        if let Some(risk) = self.reader.risk_at(models, called, position).or(code) {
            return Some(SinkSite {
                function: at,
                line: called.line,
                name: called.name.clone(),
                risk,
            });
        }
        self.callee_summary(called)?.sinks.get(&position).cloned()
    }

    /// Each call that starts a flow of input here: a source, or a call that returns one's.
    fn input_starts(&self, at: usize) -> Vec<(Start, Origin)> {
        let function = &self.functions[at];
        let models = Models::of(&function.flows.namespace);
        let starts = function.flows.calls.iter().enumerate();
        starts
            .filter(|(_, call)| !self.reader.is_sanitizer(call))
            .filter_map(|(index, call)| {
                let origin = if self.reader.is_source(models, call) {
                    Origin {
                        name: call.name.clone(),
                        path: function.path.clone(),
                        line: call.line,
                    }
                } else {
                    self.callee_summary(call)?.returns_input.clone()?
                };
                Some((Start::Result(index), origin))
            })
            .collect()
    }

    fn summary_of(&self, at: usize) -> Summary {
        let flows = &self.functions[at].flows;
        let mut summary = Summary::default();
        for (start, origin) in self.input_starts(at) {
            if summary.returns_input.is_none() && self.walk(flows, &[start]).returned {
                summary.returns_input = Some(origin);
            }
        }

        let parameters = flows.hops.iter().filter_map(|hop| match hop.from {
            Start::Parameter(position) => Some(position),
            Start::Result(_) => None,
        });
        for position in parameters.collect::<BTreeSet<_>>() {
            let walk = self.walk(flows, &[Start::Parameter(position)]);
            if walk.returned {
                summary.passes.insert(position);
            }
            let sinks = walk.arguments.iter();
            let sink = sinks.filter_map(|&reached| self.sink_at(at, reached));
            if let Some(site) = sink.min_by_key(|site| (site.function, site.line)) {
                summary.sinks.insert(position, site);
            }
        }
        summary
    }

    /// Summaries only grow, and they say more as their callees' do, so this settles.
    fn solve(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            for at in 0..self.functions.len() {
                let summary = self.summary_of(at);
                if summary != self.summaries[at] {
                    self.summaries[at] = summary;
                    changed = true;
                }
            }
        }
    }

    fn findings_in(&self, at: usize) -> Vec<(SinkSite, Origin)> {
        let flows = &self.functions[at].flows;
        let mut found = Vec::new();
        for (start, origin) in self.input_starts(at) {
            let walk = self.walk(flows, &[start]);
            let sinks = walk.arguments.iter();
            for site in sinks.filter_map(|&reached| self.sink_at(at, reached)) {
                found.push((site, origin.clone()));
            }
        }
        found
    }
}

fn finding(functions: &[FunctionFlows], site: SinkSite, origin: &Origin) -> Finding {
    let holder = &functions[site.function];
    let from = if origin.path == holder.path {
        format!("line {}", origin.line)
    } else {
        format!("{}:{}", origin.path, origin.line)
    };
    let message = format!(
        "input from `{}` on {from} reaches `{}` ({}): check how it's validated",
        origin.name,
        site.name,
        site.risk.text(),
    );
    Finding {
        function: site.function,
        defect: Defect {
            rule: RULE,
            line: site.line,
            message,
        },
    }
}

/// What reaches a sink across a run's functions, each named by what its callers call it.
pub fn find(functions: &[FunctionFlows], project: &TaintConfig) -> Vec<Finding> {
    let by_identity: BTreeMap<&str, usize> = functions
        .iter()
        .enumerate()
        .map(|(at, function)| (function.flows.identity.as_str(), at))
        .collect();
    let has_body = |call: &FlowCall| by_identity.contains_key(call.selected.as_str());
    let shell_code = functions
        .iter()
        .map(|function| builder::shell_code(&function.flows, has_body))
        .collect();
    let mut solver = Solver {
        functions,
        reader: Reader { project },
        by_identity,
        summaries: vec![Summary::default(); functions.len()],
        shell_code,
    };
    solver.solve();

    let mut found: BTreeMap<(usize, usize, String), Finding> = BTreeMap::new();
    for at in 0..functions.len() {
        for (site, origin) in solver.findings_in(at) {
            let finding = finding(functions, site, &origin);
            let key = (
                finding.function,
                finding.defect.line,
                finding.defect.message.clone(),
            );
            found.entry(key).or_insert(finding);
        }
    }
    found.into_values().collect()
}

#[cfg(test)]
mod tests;
