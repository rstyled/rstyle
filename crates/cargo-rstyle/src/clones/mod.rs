//! Opt-in, verified source-to-pair analysis; no legacy miner involvement.

pub mod capture;
mod identity;
mod linking;
mod report;
mod selected_context;
use crate::Cli;
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
pub struct Args {
    /// Extraction selection: r0, hir-expanded-v1, hir-helpers-v1, hir-data-v1, hir-data-v2 or hir-data-v3.
    #[arg(long, default_value = "r0", value_parser = rstyle_pdg::extraction_profile::ExtractionProfile::select)]
    pub extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    /// Capture an explicitly selected reachable dependency library; does not authorize external calls.
    #[arg(long = "capture-dependency")]
    pub capture_dependencies: Vec<String>,
    /// Strictly replay a saved dependency closure and its owning unit bundles.
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

fn configuration(cli: &Cli, args: &Args) -> Result<CcgraphConfig, String> {
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
    rstyle_ccgraph::parse_config(&value).map_err(|e| e.message)
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
    let capture = if let Some(directory) = &args.replay_run {
        linking::load_capture(directory, args)?
    } else {
        let mut capture =
            Stage::new("cli.prepare", "run", None).observe(|| capture::prepare(cli, args))?;
        if let Err(error) = capture::compile(&mut capture, cli, args) {
            capture.failures.push(error);
        }
        capture
    };
    let handoff = Handoff::capture(&capture, args)?;
    let evaluation = evaluate_matching(&handoff, &config, args)?;
    let matching = evaluation.report;

    let accepted = matching
        .pairs
        .iter()
        .filter(|p| p.decision == Decision::Accepted)
        .count();
    let capture_complete = capture.failures.is_empty();
    let assessment_complete = handoff.failures.is_empty();
    let complete = capture_complete && assessment_complete && !matching.incomplete;
    let report = report::make(report::Context {
        cli,
        args,
        capture: &capture,
        handoff: &handoff,
        matching: &matching,
        partitions: evaluation.partitions,
        complete,
    });
    let output = report.publish(cli.format, accepted)?;
    let code = if !complete {
        2
    } else {
        u8::from(args.fail_on_accepted_pairs && accepted > 0)
    };

    Ok((output, code))
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
            verify_receiver_inventory(function, unit.extraction_profile)?;
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
