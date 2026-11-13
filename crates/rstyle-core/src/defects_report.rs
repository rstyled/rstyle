//! The defects a run's records hold, as the report and SARIF show them.

use crate::config::{Config, DeviantArgumentConfig, IgnoredResultConfig, TaintConfig};
use crate::cpg::defects::deviant_argument::{self, Arguments, MethodArguments};
use crate::cpg::defects::ignored_result::{self, ResultUse};
use crate::cpg::defects::taint::{self, Finding, FunctionFlows};
use crate::cpg::defects::value_flow::ValueFlows;
use crate::cpg::defects::{Defect, Examined};
use crate::cpg::records::{CanonicalKey, Measured, Record};
use crate::{exclusions, suppress};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One thing a rule found, and where.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DefectRow {
    pub path: String,
    pub line: usize,
    pub rule: &'static str,
    pub severity: crate::config::Level,
    pub message: String,
    /// The function it's in, by identity.
    pub function: String,
}

/// A function the rules couldn't read, and why.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct UnknownRow {
    pub function: String,
    pub path: String,
    pub reason: String,
}

/// What the rules found in a run. No defect in a function doesn't make it correct, and
/// `unknown` lists the functions they couldn't read at all.
#[derive(Debug, Serialize)]
pub struct DefectsReport {
    pub defects: Vec<DefectRow>,
    /// Functions read, each once however many compilations hold it.
    pub functions: usize,
    pub suppressed: usize,
    pub unknown: Vec<UnknownRow>,
}

impl DefectsReport {
    pub fn text(&self) -> String {
        let rows = self.defects.iter().map(|row| {
            format!(
                "{}:{}: {}: {}: {}",
                row.path,
                row.line,
                row.rule,
                row.severity.sarif(),
                row.message
            )
        });
        let plural = if self.defects.len() == 1 { "" } else { "s" };
        let summary = format!(
            "Defects: {} defect{plural} in {} functions, {} suppressed, {} unknown",
            self.defects.len(),
            self.functions,
            self.suppressed,
            self.unknown.len(),
        );
        rows.chain(std::iter::once(summary))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Moves each record's defects out, for the functions a person wrote and a run selected. A unit
/// compiled twice, as a library and its test harness are, holds the same function twice, so what
/// repeats is reported once.
pub fn from_records(records: &mut [Record], config: &Config) -> Result<DefectsReport, String> {
    let mut fold = DefectsFold::new(config)?;
    fold.add(records);
    Ok(fold.into_report(config))
}

/// A run's defects, read a batch of records at a time and in any order: what a function holds
/// twice keeps the copy canonical order would, so the report is the same however records arrive.
pub struct DefectsFold {
    read: Gathered,
    excluded: exclusions::Matcher,
}

impl DefectsFold {
    pub fn new(config: &Config) -> Result<Self, String> {
        Ok(Self {
            read: Gathered::default(),
            excluded: exclusions::Matcher::compile(&config.exclude.0)?,
        })
    }

    /// Moves each record's defects out, for the functions a person wrote and the run selected.
    pub fn add(&mut self, records: &mut [Record]) {
        for record in records {
            let Some(examined) = record.defects.take() else {
                continue;
            };
            let written = matches!(record.measured, Measured::Known(_));
            if !written || self.excluded.is_excluded(Path::new(&record.path)) {
                continue;
            }
            self.read.add(record, examined);
        }
    }

    pub fn into_report(self, config: &Config) -> DefectsReport {
        self.read.into_report(config)
    }
}

/// Where a call is, which makes it one call however many compilations hold it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CallSite {
    path: String,
    line: usize,
    column: usize,
}

/// What a call did with its result, and what a report would say of it.
struct Call {
    dropped: bool,
    name: String,
    function: String,
    allowed: bool,
}

type Calls = BTreeMap<CallSite, Call>;

/// What's been read so far, in the sets that make a function count once.
#[derive(Default)]
struct Gathered {
    defects: BTreeSet<DefectRow>,
    functions: BTreeSet<String>,
    suppressed: BTreeSet<(String, usize, &'static str)>,
    unknown: BTreeSet<UnknownRow>,
    /// Each callee's calls, by its identity.
    results: BTreeMap<String, Calls>,
    /// Where each function's values go, by its identity.
    flows: BTreeMap<String, FunctionFlows>,
    /// What each method's calls pass, by its identity.
    arguments: BTreeMap<String, MethodArguments>,
    /// The canonical key of the record each kept entry came from. Canonical order keeps an
    /// identity's first flows and arguments and a call site's last call; keeping the least and
    /// the greatest key does the same whatever order records come in.
    kept_from: KeptFrom,
}

#[derive(Default)]
struct KeptFrom {
    flows: BTreeMap<String, CanonicalKey>,
    arguments: BTreeMap<String, CanonicalKey>,
    results: BTreeMap<(String, CallSite), CanonicalKey>,
}

/// Whether `key` replaces what `kept` holds, when the earliest record in canonical order wins.
fn earlier(kept: Option<&CanonicalKey>, key: &CanonicalKey) -> bool {
    kept.is_none_or(|kept| key < kept)
}

fn preceding(record: &Record) -> &str {
    match &record.measured {
        Measured::Known(facts) => facts.preceding_line.as_str(),
        _ => "",
    }
}

impl Gathered {
    fn add(&mut self, record: &Record, examined: Examined) {
        self.functions.insert(record.identity.clone());
        match examined {
            Examined::Read {
                defects,
                results,
                flows,
                arguments,
            } => {
                self.add_read(record, (defects, results, (flows, *arguments)));
            }
            Examined::Unknown(reason) => self.add_unknown(record, reason),
        }
    }

    fn add_read(
        &mut self,
        record: &Record,
        (defects, results, calls): (Vec<Defect>, Vec<ResultUse>, (ValueFlows, Arguments)),
    ) {
        self.add_found(record, defects);
        self.add_results(record, results);
        self.add_calls(record, calls);
    }

    /// What a function's calls do with values: where its values go, and what each call passes.
    fn add_calls(&mut self, record: &Record, (flows, arguments): (ValueFlows, Arguments)) {
        self.add_flows(record, flows);
        self.add_arguments(record, arguments);
    }

    fn add_flows(&mut self, record: &Record, flows: ValueFlows) {
        let function = FunctionFlows {
            identity: record.identity.clone(),
            path: record.path.clone(),
            allowed: suppress::allows(preceding(record), taint::RULE),
            flows,
        };
        let key = record.owned_canonical_key();
        if earlier(self.kept_from.flows.get(&record.identity), &key) {
            self.kept_from.flows.insert(record.identity.clone(), key);
            self.flows.insert(record.identity.clone(), function);
        }
    }

    fn add_arguments(&mut self, record: &Record, arguments: Arguments) {
        let method = MethodArguments {
            identity: record.identity.clone(),
            path: record.path.clone(),
            allowed: suppress::allows(preceding(record), deviant_argument::RULE),
            arguments,
        };
        let key = record.owned_canonical_key();
        if earlier(self.kept_from.arguments.get(&record.identity), &key) {
            self.kept_from
                .arguments
                .insert(record.identity.clone(), key);
            self.arguments.insert(record.identity.clone(), method);
        }
    }

    /// Reports the arguments a method passes where its siblings' same calls all pass another.
    fn compare_siblings(&mut self, limits: &DeviantArgumentConfig) {
        let methods: Vec<MethodArguments> =
            std::mem::take(&mut self.arguments).into_values().collect();
        for deviant_argument::Finding { method, defect } in deviant_argument::find(&methods, limits)
        {
            let holder = &methods[method];
            if holder.allowed {
                self.suppressed
                    .insert((holder.path.clone(), defect.line, defect.rule));
                continue;
            }
            self.defects.insert(DefectRow {
                path: holder.path.clone(),
                line: defect.line,
                rule: defect.rule,
                severity: defect.severity,
                message: defect.message,
                function: holder.identity.clone(),
            });
        }
    }

    fn add_unknown(&mut self, record: &Record, reason: String) {
        self.unknown.insert(UnknownRow {
            function: record.identity.clone(),
            path: record.path.clone(),
            reason,
        });
    }

    fn add_found(&mut self, record: &Record, found: Vec<Defect>) {
        for defect in found {
            if suppress::allows(preceding(record), defect.rule) {
                self.suppressed
                    .insert((record.path.clone(), defect.line, defect.rule));
                continue;
            }
            self.defects.insert(DefectRow {
                path: record.path.clone(),
                line: defect.line,
                rule: defect.rule,
                severity: defect.severity,
                message: defect.message,
                function: record.identity.clone(),
            });
        }
    }

    fn add_results(&mut self, record: &Record, results: Vec<ResultUse>) {
        let allowed = suppress::allows(preceding(record), ignored_result::RULE);
        for result in results {
            let site = CallSite {
                path: record.path.clone(),
                line: result.line,
                column: result.column,
            };
            let call = Call {
                dropped: result.dropped,
                name: result.name,
                function: record.identity.clone(),
                allowed,
            };
            // The latest record in canonical order wins a call site.
            let key = record.owned_canonical_key();
            let slot = (result.callee.clone(), site.clone());
            if self
                .kept_from
                .results
                .get(&slot)
                .is_none_or(|kept| key >= *kept)
            {
                self.kept_from.results.insert(slot, key);
                self.results
                    .entry(result.callee)
                    .or_default()
                    .insert(site, call);
            }
        }
    }

    /// Reports the calls that drop a result nearly all of a callee's other calls use.
    fn weigh(&mut self, calls: Calls, limits: &IgnoredResultConfig) {
        let (used, total) = (
            calls.values().filter(|call| !call.dropped).count(),
            calls.len(),
        );
        if !ignored_result::stands_out(used, total, limits) {
            return;
        }

        for (site, call) in calls.into_iter().filter(|(_, call)| call.dropped) {
            if call.allowed {
                self.suppressed
                    .insert((site.path, site.line, ignored_result::RULE));
                continue;
            }
            self.defects.insert(DefectRow {
                path: site.path,
                line: site.line,
                rule: ignored_result::RULE,
                severity: crate::config::Level::Warning,
                message: ignored_result::message(&call.name, used, total),
                function: call.function,
            });
        }
    }

    /// The rules only the run's functions together can judge.
    fn across_functions(&mut self, config: &Config) {
        self.follow_input(&config.taint);
        self.compare_siblings(&config.deviant_argument);
    }

    /// Reports the input that reaches a sink, wherever in the run each end is.
    fn follow_input(&mut self, project: &TaintConfig) {
        let functions: Vec<FunctionFlows> = std::mem::take(&mut self.flows).into_values().collect();
        for Finding { function, defect } in taint::find(&functions, project) {
            let holder = &functions[function];
            if holder.allowed {
                self.suppressed
                    .insert((holder.path.clone(), defect.line, defect.rule));
                continue;
            }
            self.defects.insert(DefectRow {
                path: holder.path.clone(),
                line: defect.line,
                rule: defect.rule,
                severity: defect.severity,
                message: defect.message,
                function: holder.identity.clone(),
            });
        }
    }

    fn into_report(mut self, config: &Config) -> DefectsReport {
        for calls in std::mem::take(&mut self.results).into_values() {
            self.weigh(calls, &config.ignored_result);
        }
        self.across_functions(config);

        DefectsReport {
            defects: self.defects.into_iter().collect(),
            functions: self.functions.len(),
            suppressed: self.suppressed.len(),
            unknown: self.unknown.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests;
