//! Opt-in, verified source-to-pair analysis; no legacy miner involvement.

mod analysis;
pub mod capture;
mod identity;
mod linking;
mod planning;
mod report;
mod review;
mod selected_context;
use crate::Cli;
use analysis::Completion;
use clap::Args as ClapArgs;
use identity::FunctionKey;
use rstyle_ccgraph::{CcgraphConfig, Decision, ImportedFunction};
use rstyle_pdg::diagnostics::Stage;
use rstyle_pdg::{EligibilityVerdict, HirFnFacts, classify, producer};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(ClapArgs, Serialize)]
#[command(group(clap::ArgGroup::new("local_analysis").args(["source_rules", "guard_clauses", "enum_dispatch", "capability_proposals", "mutation_rules", "mutation_effects"]).multiple(true)),
    group(clap::ArgGroup::new("review_analysis").args(["helper_proposals", "enum_dispatch", "capability_proposals", "source_rules", "mutation_rules"]).multiple(true)),
    group(clap::ArgGroup::new("dismissal").args(["dismiss_helper_pair", "dismiss_enum_dispatch", "dismiss_capability_pair", "dismiss_parameter", "dismiss_length", "dismiss_spacing", "dismiss_mutation_per_target", "dismiss_mutated_targets"])),
    group(clap::ArgGroup::new("finding_analysis").args(["source_rules", "guard_clauses", "helper_proposals", "enum_dispatch", "capability_proposals", "mutation_rules"]).multiple(true)))]
pub struct Args {
    /// Extraction selection: r0, hir-expanded-v1, hir-helpers-v1, hir-data-v1, hir-data-v2 or hir-data-v3.
    #[arg(long, default_value = "r0", value_parser = rstyle_pdg::extraction_profile::ExtractionProfile::select)]
    pub extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    /// Capture an explicitly selected reachable dependency library; does not authorize external calls.
    #[arg(long = "capture-dependency")]
    pub capture_dependencies: Vec<String>,
    /// Strictly reevaluate an exact saved standalone run or dependency closure without compiling.
    #[arg(long, conflicts_with = "capture_dependencies")]
    replay_run: Option<PathBuf>,
    /// JSON matching overrides, separate from legacy rstyle.toml.
    #[arg(long)]
    matching_config: Option<PathBuf>,
    /// Include conformance evidence (projection traces and WL round histograms).
    #[arg(long)]
    trace: bool,
    /// Inline full facts, or keep compact identities and references to complete unit bundles.
    #[arg(long, value_enum, default_value = "full")]
    report_detail: report::Detail,
    /// Exit 1 if any pair is accepted; similarity does not prove equivalence.
    #[arg(long)]
    fail_on_accepted_pairs: bool,
    /// Explain ordered native scalar differences in accepted pairs; incomplete explanations exit 2.
    #[arg(long)]
    explain_pairs: bool,
    /// Propose bounded u32 expression helpers; includes native pair explanations. Advisory only.
    #[arg(long)]
    helper_proposals: bool,
    /// Plan required body edits for helper-with-wrapper intents; insertion and naming remain unknown.
    #[arg(long, requires = "helper_proposals")]
    edit_plans: bool,
    /// Plan signature and witnessed caller edits for visible parameter-count findings.
    #[arg(long, requires = "source_rules")]
    parameter_plans: bool,
    /// Plan the original tail-if region of advisory guard-clause findings.
    #[arg(long, requires = "guard_clauses")]
    guard_plans: bool,
    /// Evaluate source length and signature arity independently of graph admission.
    #[arg(long)]
    source_rules: bool,
    /// JSON workspace-relative changed-file selection for source-rule presentation and finding exits.
    #[arg(long, requires = "source_rules")]
    changed_files: Option<PathBuf>,
    /// Suggest bounded guard-clause shapes from original source; advisory only.
    #[arg(long)]
    guard_clauses: bool,
    /// Suggest bounded local enum dispatch abstractions from original source and signatures.
    #[arg(long)]
    enum_dispatch: bool,
    /// Propose local capabilities from parallel ordered method usage.
    #[arg(long)]
    capability_proposals: bool,
    /// Maximum distinct-owner wrapper pairs evaluated across selected units.
    #[arg(long, default_value_t = 16384, requires = "capability_proposals")]
    max_capability_pairs: usize,
    /// Evaluate direct scalar assignment budgets with explicit unknown remainder.
    #[arg(long)]
    mutation_rules: bool,
    /// Summarize caller-visible scalar mutation through original same-unit leaf helpers.
    #[arg(long)]
    mutation_effects: bool,
    /// TOML mutation_budget and workspace exclude policy; root granularity only.
    #[arg(long, requires = "mutation_rules")]
    mutation_config: Option<PathBuf>,
    /// TOML source-rule policy (fn_length/fn_params/fn_spacing); distinct from legacy configuration.
    #[arg(long, requires = "source_rules")]
    analysis_config: Option<PathBuf>,
    /// Exit 1 when requested analyses find opportunities; incomplete analysis exits 2.
    #[arg(long, requires = "finding_analysis")]
    fail_on_findings: bool,
    /// Apply versioned reviews for requested abstraction families; lookup never writes.
    #[arg(long, requires = "review_analysis")]
    review_file: Option<PathBuf>,
    /// Inspect every retained review; omitted or unavailable assessments remain incomplete.
    #[arg(long, requires = "review_file", conflicts_with = "dismissal")]
    review_status: bool,
    /// Remove only completed negative reviews from an explicit saved run; implies status.
    #[arg(long, requires_all = ["review_file", "replay_run"], conflicts_with = "dismissal")]
    review_prune: bool,
    /// Preview review pruning without writing or creating a review file.
    #[arg(long, requires = "review_prune")]
    review_prune_dry_run: bool,
    /// Dismiss these exact saved-run helper endpoints; requires an explicit reason.
    #[arg(long, num_args = 2, requires_all = ["review_file", "review_reason", "replay_run", "helper_proposals"])]
    dismiss_helper_pair: Vec<String>,
    /// Dismiss an exact saved-run enum dispatch finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "enum_dispatch"])]
    dismiss_enum_dispatch: Option<String>,
    /// Dismiss one exact saved-run visible parameter-count finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "source_rules"])]
    dismiss_parameter: Option<String>,
    /// Dismiss one exact saved-run visible function-length finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "source_rules"])]
    dismiss_length: Option<String>,
    /// Dismiss one exact saved-run visible function-spacing finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "source_rules"])]
    dismiss_spacing: Option<String>,
    /// Dismiss one completed visible saved-run per-root mutation finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "mutation_rules"])]
    dismiss_mutation_per_target: Option<String>,
    /// Dismiss one completed visible saved-run mutated-root count finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "mutation_rules"])]
    dismiss_mutated_targets: Option<String>,
    /// Dismiss an exact saved-run ordered capability pair.
    #[arg(long, num_args = 2, requires_all = ["review_file", "review_reason", "replay_run", "capability_proposals"])]
    dismiss_capability_pair: Vec<String>,
    /// Reason retained with an explicitly dismissed finding.
    #[arg(long, requires = "dismissal")]
    review_reason: Option<String>,
    /// Retain review for future proven membership growth with unchanged original members.
    #[arg(long, requires = "dismiss_helper_pair")]
    review_allow_growth: bool,
    /// Bound local rule work; remaining functions are explicitly not evaluated.
    #[arg(long, requires = "local_analysis")]
    max_analysis_functions: Option<usize>,
    /// Execution budget: fail incomplete before enumeration if it requires more pairs.
    #[arg(long)]
    max_pairs: Option<usize>,
    #[arg(long)]
    pub release: bool,
    #[arg(long)]
    pub locked: bool,
    #[arg(long)]
    pub offline: bool,
    #[arg(long)]
    pub features: Option<String>,
    #[arg(long)]
    pub all_features: bool,
    #[arg(long)]
    pub no_default_features: bool,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(short, long)]
    pub jobs: Option<usize>,
}

impl Args {
    fn review_status_requested(&self) -> bool {
        self.review_status || self.review_prune
    }
}

#[cfg(test)]
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

struct Configuration {
    matching: CcgraphConfig,
    analysis: Option<analysis::Config>,
    mutation: Option<analysis::mutation_reports::Settings>,
}

fn configuration(cli: &Cli, args: &Args) -> Result<Configuration, String> {
    if cli.config.is_some() || cli.ignore_file.is_some() {
        return Err(
            "clones does not consume legacy --config or --ignore-file; use --matching-config"
                .into(),
        );
    }
    let value = args
        .matching_config
        .as_deref()
        .map(read_json::<Value>)
        .transpose()?
        .unwrap_or_else(|| json!({}));
    Ok(Configuration {
        matching: rstyle_ccgraph::parse_config(&value).map_err(|e| e.message)?,
        analysis: analysis::configuration(args)?,
        mutation: analysis::mutation_reports::configuration(args)?,
    })
}

#[derive(Serialize)]
struct Function {
    key: FunctionKey,
    compilation_unit: String,
    unit_evidence: UnitEvidence,
    outcome: String,
    reasons: Vec<String>,
    facts: ReportFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    nominal_unit_binding: Option<rstyle_pdg::nominal_binding::Binding>,
    projection_trace: Option<Value>,
    extraction_evidence: Option<Value>,
}

#[derive(Serialize)]
struct UnitEvidence {
    report_unit: identity::UnitId,
    namespace: rstyle_pdg::unit_store::UnitNamespace,
    bundle: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependency_closure: Option<PathBuf>,
    function: u32,
}

struct ReportFacts {
    store: Arc<rstyle_pdg::unit_store::UnitStore>,
    index: usize,
}

impl Deref for ReportFacts {
    type Target = HirFnFacts;
    fn deref(&self) -> &HirFnFacts {
        &self.store.functions()[self.index]
    }
}

impl Serialize for ReportFacts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

#[derive(Serialize)]
struct UnitBuildContexts {
    unit_bundle: PathBuf,
    builds: rstyle_pdg::nominal_binding::BuildContexts,
}

struct Handoff {
    build_contexts: Vec<UnitBuildContexts>,
    source_inventory: Vec<Value>,
    assessed: Vec<Function>,
    admitted: Vec<ImportedFunction>,
    failures: Vec<String>,
    keys: BTreeSet<FunctionKey>,
    unit_contexts: Vec<identity::UnitContext>,
}

struct UnitRequest<'a> {
    id: identity::UnitId,
    unit: &'a capture::Unit,
    capture: &'a capture::Capture,
    args: &'a Args,
}

struct FunctionInput<'a> {
    index: usize,
    trace: bool,
    analysis: &'a rstyle_pdg::unit_store::UnitAnalysis<'a>,
}

impl Function {
    fn exclude(&mut self, eligibility: rstyle_pdg::HirEligibility) {
        self.outcome = match eligibility.status {
            EligibilityVerdict::Unknown => "unknown",
            _ => "excluded",
        }
        .into();
        self.reasons = eligibility
            .diagnostics
            .into_iter()
            .map(|d| d.reason)
            .collect();
    }
}

impl Handoff {
    fn new(assignments: Option<&[identity::Assignment<'_>]>) -> Self {
        Self {
            build_contexts: Vec::new(),
            source_inventory: Vec::new(),
            assessed: Vec::new(),
            admitted: Vec::new(),
            failures: Vec::new(),
            keys: BTreeSet::new(),
            unit_contexts: assignments
                .map(|units| {
                    units
                        .iter()
                        .map(|unit| identity::UnitContext {
                            id: unit.context.id,
                            observed_unit: unit.context.observed_unit,
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn sorted(mut self) -> Self {
        self.assessed.sort_by_key(|function| function.key);
        self
    }

    fn capture(capture: &capture::Capture, args: &Args) -> Result<Self, String> {
        Stage::new("cli.handoff", "run", None).observe(|| Ok(handoff(capture, args)))
    }

    fn add(&mut self, mut record: Function, result: Result<Option<ImportedFunction>, String>) {
        match result {
            Ok(Some(function)) => self.admitted.push(function),
            Ok(None) => {}
            Err(error) => {
                record.outcome = "failed".into();
                record.reasons.push(error.clone());
                self.failures.push(error);
            }
        }

        self.assessed.push(record);
    }
}

fn matching_profile(args: &Args) -> rstyle_ccgraph::MatchingProfile {
    match args.extraction_profile {
        rstyle_pdg::extraction_profile::ExtractionProfile::DataV1 => {
            rstyle_ccgraph::MatchingProfile::DataV1
        }
        rstyle_pdg::extraction_profile::ExtractionProfile::DataV2
        | rstyle_pdg::extraction_profile::ExtractionProfile::DataV3 => {
            rstyle_ccgraph::MatchingProfile::DataCallsV1
        }
        _ => rstyle_ccgraph::MatchingProfile::ScalarV1,
    }
}

pub fn run(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    if args.replay_run.is_some() {
        rstyle_pdg::verification_policy::strict(|| run_generated(cli, args))
    } else {
        rstyle_pdg::verification_policy::generated(|| run_generated(cli, args))
    }
}

fn run_generated(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    let config = configuration(cli, args)?;
    let capture = capture_input(cli, args)?;
    let handoff = Handoff::capture(&capture, args)?;
    let evaluation = evaluate_matching(&handoff, &config.matching, args)?;
    let matching = evaluation.report;

    let accepted = matching
        .pairs
        .iter()
        .filter(|p| p.decision == Decision::Accepted)
        .count();
    let capture_complete = capture.failures.is_empty();
    let assessment_complete = handoff.failures.is_empty();
    let complete = capture_complete && assessment_complete && !matching.incomplete;
    let context = report::Context {
        cli,
        args,
        capture: &capture,
        handoff: &handoff,
        matching: &matching,
        partitions: evaluation.partitions,
        complete,
        analysis_config: config.analysis.as_ref(),
        mutation_config: config.mutation.as_ref(),
        matching_config: &config.matching,
    };
    let consumers = report::Consumers::new(&context)?;
    let report = report::make(context, consumers);
    let output = report.publish(cli.format, accepted)?;
    let code = if complete && analyses_complete(&report) {
        u8::from(finding_exit_requested(&report, args, accepted))
    } else {
        2
    };

    Ok((output, code))
}

fn analyses_complete(report: &report::Report<'_>) -> bool {
    report.analysis.permits_success()
        && report.enum_dispatch.permits_success()
        && report.capability_proposals.permits_success()
        && report.mutation_analysis.permits_success()
        && report.mutation_effects.permits_success()
        && report.mutation_review.permits_success()
        && report.edit_planning.permits_success()
        && report
            .parameter_planning
            .as_ref()
            .is_none_or(analysis::Completion::permits_success)
        && report
            .guard_planning
            .as_ref()
            .is_none_or(analysis::Completion::permits_success)
        && report
            .edit_coordination
            .as_ref()
            .is_none_or(analysis::Completion::permits_success)
        && report.pair_reports.permits_success()
        && report.review.permits_success()
        && report.abstraction_review.permits_success()
        && report.source_reviews.permits_success()
        && report
            .review_status
            .as_ref()
            .is_none_or(analysis::Completion::permits_success)
}

fn finding_exit_requested(report: &report::Report<'_>, args: &Args, accepted: usize) -> bool {
    (args.fail_on_accepted_pairs && accepted > 0)
        || (args.fail_on_findings
            && (source_finding_exit_count(report) > 0
                || report
                    .abstraction_review
                    .dispatch_visible_or(report.enum_dispatch.findings)
                    > 0
                || report
                    .abstraction_review
                    .capability_visible_or(report.capability_proposals.findings)
                    > 0
                || report.mutation_analysis.visible_findings > 0
                || report
                    .review
                    .visible_or(report.pair_reports.helper_proposals.findings)
                    > 0))
}

fn source_finding_exit_count(report: &report::Report<'_>) -> usize {
    report
        .analysis
        .finding_exit_count()
        .saturating_sub(report.source_reviews.hidden_selected_findings())
}

fn capture_input(cli: &Cli, args: &Args) -> Result<capture::Capture, String> {
    if let Some(directory) = &args.replay_run {
        if cli.manifest_path.is_some() || !cli.packages.is_empty() || cli.all_targets {
            return Err("Saved-run replay uses its original selection; new manifest, package and all-targets flags are not accepted".into());
        }
        return linking::load_capture(directory, args);
    }
    capture_fresh(cli, args)
}

fn capture_fresh(cli: &Cli, args: &Args) -> Result<capture::Capture, String> {
    let mut capture =
        Stage::new("cli.prepare", "run", None).observe(|| capture::prepare(cli, args))?;
    if let Err(error) = capture::compile(&mut capture, cli, args) {
        capture.failures.push(error);
    }
    Ok(capture)
}

fn evaluate_matching(
    handoff: &Handoff,
    config: &CcgraphConfig,
    args: &Args,
) -> Result<rstyle_ccgraph::pipeline::Evaluation, String> {
    Stage::new("cli.matching", "run", None).observe(|| {
        Ok(rstyle_ccgraph::pipeline::evaluate_selected_with_options(
            &handoff.admitted,
            config,
            rstyle_ccgraph::pipeline::EvaluationOptions {
                maximum_pairs: args.max_pairs,
                profile: matching_profile(args),
                trace: args.trace,
            },
        ))
    })
}

fn handoff(capture: &capture::Capture, args: &Args) -> Handoff {
    let assignments = identity::assign(&capture.units);
    let mut result = Handoff::new(assignments.as_deref().ok());
    if !capture.failures.is_empty() {
        return result;
    }

    let assignments = match assignments {
        Ok(assignments) => assignments,
        Err(error) => {
            return Handoff {
                failures: vec![error],
                ..result
            };
        }
    };
    if !capture.plan.dependency_units.is_empty()
        && args
            .extraction_profile
            .supports("rust.verified-cross-unit-scalar-helpers")
    {
        if let Err(error) = linking::consume(capture, args, &assignments, &mut result) {
            return Handoff {
                failures: result.failures.into_iter().chain([error]).collect(),
                ..result
            }
            .sorted();
        }
    } else {
        for assignment in assignments {
            let request = UnitRequest {
                id: assignment.context.id,
                unit: assignment.unit,
                capture,
                args,
            };
            if let Err(error) = consume_unit(&request, &mut result) {
                result.failures.push(error);
            }
        }
    }
    result.sorted()
}

fn consume_unit(request: &UnitRequest<'_>, result: &mut Handoff) -> Result<(), String> {
    let (store, bindings) = load_unit(request, result)?;
    let store = store.with_slice_length_instances()?;
    consume_store(Arc::new(store), bindings, request, result)
}

fn load_unit(
    request: &UnitRequest<'_>,
    result: &mut Handoff,
) -> Result<
    (
        rstyle_pdg::unit_store::UnitStore,
        rstyle_pdg::nominal_binding::UnitBindings,
    ),
    String,
> {
    let unit = request.unit;
    let capture = request.capture;
    if unit.extraction_profile != capture.plan.extraction_profile {
        return Err("Unit extraction profile disagrees with plan".into());
    }
    let snapshot = capture::load_inventory(unit)?;
    let selected_context = selected_context::SelectedContext::new(unit)?;
    let mut bindings = rstyle_pdg::nominal_binding::UnitBindings::empty(0);

    let inspect = |facts: &[HirFnFacts]| {
        for function in facts {
            verify_frontend_invocation(function, unit, capture, &selected_context)?;
            verify_receiver_inventory(function, unit.extraction_profile)
                .map_err(|error| format!("Receiver capture for {}: {error}", function.name))?;
            rstyle_pdg::data_library::verify_evidence(function)?;
        }
        bindings = nominal_bindings(facts, unit)?;
        Ok(())
    };
    let store = if request.args.replay_run.is_some() {
        rstyle_pdg::unit_store::UnitStore::from_bundle_with(snapshot.bundle.bytes(), inspect)?
    } else {
        rstyle_pdg::unit_store::UnitStore::from_verified_current_run(&snapshot.bundle, inspect)?
    };
    result
        .source_inventory
        .extend(source_inventory(unit, &snapshot)?);
    Ok((store, bindings))
}

fn consume_store(
    store: Arc<rstyle_pdg::unit_store::UnitStore>,
    bindings: rstyle_pdg::nominal_binding::UnitBindings,
    request: &UnitRequest,
    result: &mut Handoff,
) -> Result<(), String> {
    let analysis = store.analysis();
    result.build_contexts.push(UnitBuildContexts {
        unit_bundle: request.unit.directory.join("unit.evidence.json"),
        builds: bindings.builds,
    });

    let keys = identity::functions(request.id, store.functions())?;
    for (index, (binding, key)) in bindings.bindings.into_iter().zip(keys).enumerate() {
        if !result.keys.insert(key) {
            return Err(format!("duplicate function key {key}"));
        }
        let facts = ReportFacts {
            store: Arc::clone(&store),
            index,
        };
        let mut record = function_record(facts, key, request.unit, index);
        record.nominal_unit_binding = binding;
        let input = FunctionInput {
            index,
            trace: request.args.trace,
            analysis: &analysis,
        };
        let admission = consume_function(&mut record, &input);
        result.add(record, admission);
    }
    Ok(())
}

fn verify_receiver_inventory(
    facts: &HirFnFacts,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<(), String> {
    if profile.is_data()
        && !facts
            .build
            .frontend_options
            .contains_key(rstyle_pdg::data_receivers::KEY)
    {
        return Err("Missing required Data receiver capture inventory".into());
    }
    rstyle_pdg::data_receivers::verify_capture(facts).map(|_| ())
}

fn nominal_bindings(
    facts: &[HirFnFacts],
    unit: &capture::Unit,
) -> Result<rstyle_pdg::nominal_binding::UnitBindings, String> {
    if !unit
        .extraction_profile
        .supports("rust.verified-helper-calls")
    {
        return Ok(rstyle_pdg::nominal_binding::UnitBindings::empty(
            facts.len(),
        ));
    }
    let context = unit
        .target
        .package_context
        .as_ref()
        .ok_or("Missing selected Cargo package snapshot")?;
    if context.package_id != unit.target.package
        || context.manifest_path.parent() != Some(unit.target.manifest_dir.as_path())
    {
        return Err("Cargo package snapshot disagrees with selected target".into());
    }

    rstyle_pdg::nominal_binding::bind_unit(facts, context)
}

fn source_inventory(
    unit: &capture::Unit,
    snapshot: &capture::Snapshot,
) -> Result<Option<Value>, String> {
    if !unit
        .extraction_profile
        .supports("rust.conditional-source-correspondence")
    {
        return Ok(None);
    }
    let inventory: Vec<Value> = snapshot.json("source-inventory.json")?;
    Ok(Some(
        json!({"compilation_unit":unit.target, "test":unit.test, "source_functions":inventory}),
    ))
}

fn function_record(
    function: ReportFacts,
    key: FunctionKey,
    unit: &capture::Unit,
    index: usize,
) -> Function {
    Function {
        key,
        compilation_unit: format!(
            "{}:{}:test={}",
            unit.target.package, unit.target.name, unit.test
        ),
        unit_evidence: UnitEvidence {
            report_unit: key.unit(),
            namespace: *function.store.namespace(),
            bundle: unit.directory.join("unit.evidence.json"),
            dependency_closure: None,
            function: u32::try_from(index).expect("checked unit function table"),
        },
        outcome: "assessed".into(),
        reasons: Vec::new(),
        facts: function,
        nominal_unit_binding: None,
        projection_trace: None,
        extraction_evidence: None,
    }
}

fn verify_frontend_invocation(
    facts: &HirFnFacts,
    unit: &capture::Unit,
    capture: &capture::Capture,
    selected_context: &selected_context::SelectedContext,
) -> Result<(), String> {
    if rstyle_pdg::extraction_profile::ExtractionProfile::from_facts(facts)?
        != capture.plan.extraction_profile
    {
        return Err("frontend extraction selection disagrees with run".into());
    }
    let value = facts
        .build
        .frontend_options
        .get("invocation")
        .ok_or("missing captured invocation")?;
    let actual: Vec<String> = Deserialize::deserialize(value).map_err(|e| e.to_string())?;
    let suffix = actual
        .strip_prefix(unit.invocation.as_slice())
        .ok_or("frontend evidence belongs to another compiler invocation")?;
    let sysroot_only = suffix.len() == 2 && suffix[0] == "--sysroot" && !suffix[1].is_empty();

    if (!suffix.is_empty() && !sysroot_only)
        || facts.build.rustc_version.known_value() != capture.plan.compiler_identity.lines().next()
    {
        return Err("frontend invocation or pinned compiler identity disagrees with run".into());
    }
    selected_context.check(
        facts
            .build
            .frontend_options
            .get(rstyle_pdg::nominal_boundary::CONTEXT_KEY),
    )?;
    verify_data_compiler_commit(facts, capture)
}

fn verify_data_compiler_commit(
    facts: &HirFnFacts,
    capture: &capture::Capture,
) -> Result<(), String> {
    if !capture.plan.extraction_profile.is_data() {
        return Ok(());
    }
    let expected = capture
        .plan
        .compiler_identity
        .lines()
        .find_map(|line| line.strip_prefix("commit-hash: "))
        .ok_or("Missing planned compiler commit identity")?;
    if facts.build.rustc_commit.known_value() != Some(expected) {
        return Err("Data compiler commit disagrees with frozen compiler identity".into());
    }
    Ok(())
}

fn consume_function(
    record: &mut Function,
    input: &FunctionInput<'_>,
) -> Result<Option<ImportedFunction>, String> {
    let facts = input
        .analysis
        .function(input.index)
        .ok_or("Missing unit frontend")?;
    let eligibility = classify(&facts);
    if eligibility.status != EligibilityVerdict::Eligible {
        if input.analysis.pair(input.index).is_some() {
            return Err("unsupported function has a graph root".into());
        }
        record.exclude(eligibility);
        return Ok(None);
    }
    if !facts
        .build
        .frontend_options
        .get("source_identifier")
        .and_then(Value::as_str)
        .is_some_and(|name| !name.is_empty())
    {
        return Err("missing captured unqualified source identifier".into());
    }

    let pair = input
        .analysis
        .pair(input.index)
        .ok_or("Missing eligible unit graph root")?;
    let mut function =
        rstyle_ccgraph::admit_produced(pair, &facts).map_err(|errors| errors.join("; "))?;
    function.key = record.key.to_string();
    record.projection_trace = input.trace.then(|| projection_trace(&function));
    record_admission(record, pair)?;
    Ok(Some(function))
}

fn record_admission(record: &mut Function, pair: &producer::Pair) -> Result<(), String> {
    let cfg = serde_json::to_value(&pair.cfg.extraction_evidence).map_err(|e| e.to_string())?;
    let pdg = serde_json::to_value(&pair.pdg.extraction_evidence).map_err(|e| e.to_string())?;
    let evidence = serde_json::Map::from_iter([("cfg".into(), cfg), ("pdg".into(), pdg)]);

    record.outcome = "admitted".into();
    record.extraction_evidence = Some(Value::Object(evidence));
    Ok(())
}

fn projection_trace(function: &ImportedFunction) -> Value {
    match rstyle_ccgraph::project(function) {
        Err(error) => json!({"error":format!("{error:?}")}),
        Ok(projection) => json!({
            "nodes": projection.trace.nodes.iter().map(|(id, disposition)| json!({"id":id,"disposition":format!("{disposition:?}")})).collect::<Vec<_>>(),
            "edges": projection.trace.edges.iter().map(|e| json!({"origin":e.origin,"source":e.source,"target":e.target,"family":e.family,"disposition":format!("{:?}",e.disposition)})).collect::<Vec<_>>()
        }),
    }
}
