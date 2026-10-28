//! Opt-in caller-visible scalar mutation observations, separate from rule budgets.
mod scope;
mod witnesses;
use super::{Coverage, Execution};
use crate::clones::{Args, Function, FunctionKey, Handoff, capture};
use rstyle_pdg::{
    consumer_facts::{Availability, InspectionExecution, effects, mutations, statement_inventory},
    unit_store::UnitStore,
};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Report<'a> {
    schema: &'static str,
    statement_metric: &'static str,
    #[serde(flatten)]
    scope: scope::Summary,
    functions: Vec<FunctionReport<'a>>,
    summaries: Vec<Summary<'a>>,
    units: Vec<UnitWork>,
    limitations: &'static [&'static str],
}

#[derive(Serialize)]
struct FunctionReport<'a> {
    key: FunctionKey,
    unit: usize,
    name: &'a str,
    source_file: Option<&'a str>,
    fn_span: rstyle_pdg::ByteSpan,
    evidence: effects::Assessment,
    statement_inventory: statement_inventory::Inventory,
    direct_sites: Vec<mutations::WriteSite>,
    boundaries: Vec<mutations::Boundary>,
}

impl super::local_scope::FunctionStatus for FunctionReport<'_> {
    fn status(&self) -> super::local_scope::Status {
        let evidence = &self.evidence;
        let assessed = evidence.execution != mutations::Execution::BudgetExhausted;
        super::local_scope::Status {
            assessed,
            evaluated: assessed
                && evidence.coverage == mutations::Coverage::Complete
                && matches!(
                    evidence.availability,
                    rstyle_pdg::consumer_facts::Availability::Known(())
                ),
        }
    }
}

#[derive(Serialize)]
struct Summary<'a> {
    unit: usize,
    summary_index: usize,
    function_index: usize,
    definition_identity: Option<&'a str>,
    name: &'a str,
    fn_span: rstyle_pdg::ByteSpan,
    sites: Vec<witnesses::Site>,
}

struct Group<'a> {
    store: &'a UnitStore,
    functions: Vec<(usize, &'a Function)>,
}

#[derive(Serialize)]
struct UnitWork {
    unit: usize,
    statistics: effects::Statistics,
}

pub(in crate::clones) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    args: &Args,
) -> Result<Report<'a>, String> {
    let mut report = Report {
        schema: "rstyle.mutation-effects/3",
        statement_metric: "rstyle.original-source-statements/1",
        scope: scope::Summary::new(handoff, capture, args.mutation_effects),
        functions: Vec::new(),
        summaries: Vec::new(),
        units: Vec::new(),
        limitations: &[
            "Statement inventory independently counts original nested source statements and bare match arms; unavailable counts do not alter scalar-effect coverage and never authorize a density ratio.",
            "Caller-visible scalar mutation only; no purity, panic freedom, termination, density or rewrite-safety claim.",
            "Original same-unit nongeneric scalar helpers through at most two call edges; deeper or incomplete summaries, unsupported signatures and arguments remain incomplete.",
            "Direct assignment sites and distinct summarized call occurrences are separate; witnesses describe syntax, never runtime frequency.",
            "Checked ordinary branches and primitive loops retain original sites once; unsupported controls and aliases remain incomplete, and formal-use certification is separate.",
        ],
    };
    if args.mutation_effects {
        report.inspect(handoff, args)?;
    }
    report.scope.finish(&report.functions, false);
    Ok(report)
}

impl<'a> Report<'a> {
    fn inspect(&mut self, handoff: &'a Handoff, args: &Args) -> Result<(), String> {
        let mut remaining = args
            .max_analysis_functions
            .unwrap_or(effects::Limits::default().functions);
        let batches = groups(handoff)
            .into_iter()
            .enumerate()
            .map(|(unit, group)| {
                let allowance = remaining.min(group.store.functions().len());
                remaining -= allowance;
                let limits = effects::Limits {
                    functions: allowance,
                    ..effects::Limits::default()
                };
                self.inspect_group(unit, group, limits)
            })
            .collect::<Result<Vec<_>, String>>()?;

        let mut functions: Vec<_> = batches.into_iter().flatten().collect();
        functions.sort_by_key(|(ordinal, _)| *ordinal);
        self.functions = functions
            .into_iter()
            .map(|(_, function)| function)
            .collect();
        Ok(())
    }

    fn inspect_group(
        &mut self,
        unit: usize,
        group: Group<'a>,
        limits: effects::Limits,
    ) -> Result<Vec<(usize, FunctionReport<'a>)>, String> {
        let mut session = effects::Session::new(group.store, limits)?;
        let functions = group
            .functions
            .into_iter()
            .map(|(ordinal, function)| {
                let evidence = session.assess(function.facts.index)?;
                let observed = session.observations(function.facts.index);
                Ok((
                    ordinal,
                    function_report(unit, function, evidence, observed)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;

        self.summaries
            .extend(export_summaries(unit, group.store, &session)?);
        self.units.push(UnitWork {
            unit,
            statistics: session.statistics(),
        });
        Ok(functions)
    }

    pub(in crate::clones) fn permits_success(&self) -> bool {
        self.scope.execution == Execution::NotRequested
            || (self.scope.execution == Execution::Completed
                && self.scope.coverage == Coverage::Complete)
    }

    pub(in crate::clones) fn render(&self) -> String {
        if self.scope.execution == Execution::NotRequested {
            return String::new();
        }
        format!(
            "\nScalar mutation effects: {:?} coverage; {} complete, {} unavailable functions; {} retained scalar summaries.",
            self.scope.coverage,
            self.scope.evaluated_functions,
            self.scope.unavailable_functions,
            self.summaries.len()
        )
    }
}

fn groups(handoff: &Handoff) -> Vec<Group<'_>> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for (ordinal, function) in handoff.assessed.iter().enumerate() {
        let store = function.facts.store.as_ref();
        if let Some(group) = groups
            .iter_mut()
            .find(|group| std::ptr::eq(group.store, store))
        {
            group.functions.push((ordinal, function));
        } else {
            groups.push(Group {
                store,
                functions: vec![(ordinal, function)],
            });
        }
    }
    groups
}

fn function_report<'a>(
    unit: usize,
    function: &'a Function,
    evidence: effects::Assessment,
    observed: Option<&mutations::Observations>,
) -> Result<FunctionReport<'a>, String> {
    let statement_inventory = if evidence.execution == mutations::Execution::BudgetExhausted {
        statement_inventory::Inventory {
            statements: Availability::Unknown {
                reason: "Analysis function budget exhausted".into(),
            },
            execution: InspectionExecution::BudgetExhausted,
        }
    } else {
        statement_inventory::inspect(&function.facts)?
    };
    Ok(FunctionReport {
        key: function.key,
        unit,
        name: &function.facts.name,
        source_file: function
            .facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str),
        fn_span: function.facts.fn_span,
        evidence,
        statement_inventory,
        direct_sites: observed
            .map(|observed| observed.direct.sites.iter().map(copy_site).collect())
            .unwrap_or_default(),
        boundaries: observed
            .map(|observed| {
                observed
                    .direct
                    .boundaries
                    .iter()
                    .map(copy_boundary)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn export_summaries<'a>(
    unit: usize,
    store: &'a UnitStore,
    session: &effects::Session<'_>,
) -> Result<Vec<Summary<'a>>, String> {
    session
        .summaries()
        .iter()
        .enumerate()
        .map(|(summary_index, summary)| {
            let sites = witnesses::sites(summary, session)?;
            let facts = &store.functions()[summary.function_index];
            Ok(Summary {
                unit,
                summary_index,
                function_index: summary.function_index,
                definition_identity: facts
                    .build
                    .frontend_options
                    .get("definition_identity")
                    .and_then(serde_json::Value::as_str),
                name: &facts.name,
                fn_span: facts.fn_span,
                sites,
            })
        })
        .collect()
}

fn copy_site(site: &mutations::WriteSite) -> mutations::WriteSite {
    mutations::WriteSite {
        span: site.span,
        root: site.root.clone(),
        operation: site.operation,
        caller_visible: site.caller_visible,
    }
}

fn copy_boundary(boundary: &mutations::Boundary) -> mutations::Boundary {
    mutations::Boundary {
        availability: boundary.availability,
        span: boundary.span,
        reason: boundary.reason.clone(),
    }
}
