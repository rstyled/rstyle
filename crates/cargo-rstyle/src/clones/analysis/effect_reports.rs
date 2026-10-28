//! Opt-in caller-visible scalar mutation observations, separate from rule budgets.
mod density;
pub(in crate::clones) use density::{Settings, configuration};
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
    density_metric: &'static str,
    scalar_mutations_requested: bool,
    density_requested: bool,
    density_config: Option<&'a Settings>,
    pub findings: usize,
    pub visible_findings: usize,
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
    density: Option<density::Report>,
    #[serde(skip)]
    scalar_requested: bool,
    direct_sites: Vec<mutations::WriteSite>,
    boundaries: Vec<mutations::Boundary>,
}

impl super::local_scope::FunctionStatus for FunctionReport<'_> {
    fn status(&self) -> super::local_scope::Status {
        let evidence = &self.evidence;
        let assessed = evidence.execution != mutations::Execution::BudgetExhausted
            && (self.density.is_none()
                || self.statement_inventory.execution != InspectionExecution::BudgetExhausted);
        super::local_scope::Status {
            assessed,
            evaluated: assessed
                && (!self.scalar_requested
                    || (evidence.coverage == mutations::Coverage::Complete
                        && matches!(evidence.availability, Availability::Known(()))))
                && self.density.as_ref().is_none_or(|density| density.complete),
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
    density_summaries_classified: usize,
}

#[derive(Clone, Copy)]
struct Request<'a> {
    scalar: bool,
    settings: Option<&'a Settings>,
    root: &'a std::path::Path,
}

pub(in crate::clones) fn evaluate<'a>(
    handoff: &'a Handoff,
    capture: &capture::Capture,
    args: &Args,
    settings: Option<&'a Settings>,
) -> Result<Report<'a>, String> {
    let mut report = Report {
        schema: "rstyle.mutation-effects/4",
        scalar_mutations_requested: args.mutation_effects,
        density_requested: args.effect_density,
        density_config: settings,
        findings: 0,
        visible_findings: 0,
        statement_metric: "rstyle.original-source-statements/1",
        density_metric: "rstyle.static-effect-events/1",
        scope: scope::Summary::new(
            handoff,
            capture,
            args.mutation_effects || args.effect_density,
        ),
        functions: Vec::new(),
        summaries: Vec::new(),
        units: Vec::new(),
        limitations: &[
            "Statement inventory independently counts original nested source statements and bare match arms; unavailable counts prevent density ratios.",
            "Scalar mutation proofs and configured static event density do not establish runtime effect frequency, purity, panic freedom, termination or rewrite safety.",
            "Density requires complete checked scalar syntax or classified safe external free calls with owned-u32 arguments and scalar/unit/never results; unknown calls and opaque operations prevent ratios, even at zero weights.",
            "Implicit arithmetic panics are outside the explicit static event vocabulary; global, unsafe and Drop-bearing forms remain unsupported rather than zero.",
            "Original same-unit nongeneric scalar helpers through at most two call edges; deeper or incomplete summaries, unsupported signatures and arguments remain incomplete.",
            "Direct assignment sites and distinct summarized call occurrences are separate; witnesses describe syntax, never runtime frequency.",
            "Checked ordinary branches and primitive loops retain original sites once; unsupported controls and aliases remain incomplete, and formal-use certification is separate.",
        ],
    };
    if args.mutation_effects || args.effect_density {
        report.inspect(
            handoff,
            args,
            Request {
                scalar: args.mutation_effects,
                settings,
                root: &capture.plan.root,
            },
        )?;
    }

    report.count_findings();
    report.scope.finish(&report.functions, false);
    Ok(report)
}

impl<'a> Report<'a> {
    fn count_findings(&mut self) {
        self.findings = self
            .functions
            .iter()
            .filter_map(|f| f.density.as_ref())
            .map(|d| d.findings)
            .sum();
        self.visible_findings = self
            .functions
            .iter()
            .filter_map(|f| f.density.as_ref())
            .map(|d| d.visible_findings)
            .sum();
    }

    fn inspect(
        &mut self,
        handoff: &'a Handoff,
        args: &Args,
        request: Request<'_>,
    ) -> Result<(), String> {
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
                self.inspect_group(
                    group,
                    GroupRequest {
                        unit,
                        limits,
                        options: request,
                    },
                )
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
        group: Group<'a>,
        context: GroupRequest<'_>,
    ) -> Result<Vec<(usize, FunctionReport<'a>)>, String> {
        let GroupRequest {
            unit,
            limits,
            options: request,
        } = context;
        let mut session = effects::Session::new(group.store, limits)?;
        let mut density_cache = density::Cache::new();
        let functions = group
            .functions
            .into_iter()
            .map(|(ordinal, function)| {
                let evidence = session.assess(function.facts.index)?;
                Ok((
                    ordinal,
                    function_report(
                        FunctionRequest {
                            unit,
                            options: request,
                            function,
                        },
                        evidence,
                        &session,
                        &mut density_cache,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;

        self.summaries
            .extend(export_summaries(unit, group.store, &session)?);
        self.units.push(UnitWork {
            unit,
            statistics: session.statistics(),
            density_summaries_classified: density_cache.len(),
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
        let mut output = format!(
            "\nRequested scalar mutation/density observations: {:?} coverage; {} complete, {} unavailable functions; {} retained scalar summaries.",
            self.scope.coverage,
            self.scope.evaluated_functions,
            self.scope.unavailable_functions,
            self.summaries.len()
        );
        for function in &self.functions {
            if let Some(density) = &function.density {
                output.push_str(&density.render(
                    function.name,
                    function.source_file,
                    function.fn_span,
                ));
            }
        }
        output
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

struct FunctionRequest<'a, 'r> {
    unit: usize,
    options: Request<'r>,
    function: &'a Function,
}

struct GroupRequest<'a> {
    unit: usize,
    limits: effects::Limits,
    options: Request<'a>,
}

fn function_report<'a>(
    context: FunctionRequest<'a, '_>,
    evidence: effects::Assessment,
    session: &effects::Session<'_>,
    cache: &mut density::Cache,
) -> Result<FunctionReport<'a>, String> {
    let FunctionRequest {
        unit,
        options: request,
        function,
    } = context;
    let statement_inventory = inspect_statements(function, &evidence)?;
    let observed = session.observations(function.facts.index);
    let density = request
        .settings
        .map(|settings| {
            density::evaluate(
                density::Inputs {
                    function,
                    cache,
                    assessment: &evidence,
                    session,
                    statements: &statement_inventory,
                },
                settings,
                request.root,
            )
        })
        .transpose()?;

    let direct = copy_direct(observed);
    Ok(FunctionReport {
        scalar_requested: request.scalar,
        density,
        key: function.key,
        unit,
        name: &function.facts.name,
        source_file: super::source_file(function),
        fn_span: function.facts.fn_span,
        evidence,
        statement_inventory,
        direct_sites: direct.sites,
        boundaries: direct.boundaries,
    })
}

#[derive(Default)]
struct CopiedDirect {
    sites: Vec<mutations::WriteSite>,
    boundaries: Vec<mutations::Boundary>,
}

fn copy_direct(observed: Option<&mutations::Observations>) -> CopiedDirect {
    let Some(observed) = observed else {
        return CopiedDirect::default();
    };

    CopiedDirect {
        sites: observed.direct.sites.iter().map(copy_site).collect(),
        boundaries: observed
            .direct
            .boundaries
            .iter()
            .map(copy_boundary)
            .collect(),
    }
}

fn inspect_statements(
    function: &Function,
    evidence: &effects::Assessment,
) -> Result<statement_inventory::Inventory, String> {
    Ok(
        if evidence.execution == mutations::Execution::BudgetExhausted {
            statement_inventory::Inventory {
                statements: Availability::Unknown {
                    reason: "Analysis function budget exhausted".into(),
                },
                execution: InspectionExecution::BudgetExhausted,
            }
        } else {
            statement_inventory::inspect(&function.facts)?
        },
    )
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
