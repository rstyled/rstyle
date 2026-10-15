//! Opt-in, verified source-to-pair analysis; no legacy miner involvement.

pub mod capture;
mod report;
use crate::Cli;
use clap::Args as ClapArgs;
use rstyle_ccgraph::{CcgraphConfig, Decision, ImportedFunction};
use rstyle_pdg::diagnostics::Stage;
use rstyle_pdg::{EligibilityVerdict, HirFnFacts, classify, producer};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
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
    /// JSON matching overrides, separate from legacy rstyle.toml.
    #[arg(long)]
    matching_config: Option<PathBuf>,
    /// Include conformance evidence (projection traces and WL round histograms).
    #[arg(long)]
    trace: bool,
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
    key: String,
    compilation_unit: String,
    unit_evidence: Value,
    outcome: String,
    reasons: Vec<String>,
    facts: ReportFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    nominal_unit_binding: Option<rstyle_pdg::nominal_binding::Binding>,
    projection_trace: Option<Value>,
    extraction_evidence: Option<Value>,
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

struct Handoff {
    source_inventory: Vec<Value>,
    assessed: Vec<Function>,
    admitted: Vec<ImportedFunction>,
    failures: Vec<String>,
    keys: BTreeSet<String>,
}

struct UnitRequest<'a> {
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
    rstyle_pdg::verification_policy::generated(|| run_generated(cli, args))
}

fn run_generated(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    let config = configuration(cli, args)?;
    let mut capture =
        Stage::new("cli.prepare", "run", None).observe(|| capture::prepare(cli, args))?;
    if let Err(error) = capture::compile(&mut capture, cli, args) {
        capture.failures.push(error);
    }
    let handoff = Stage::new("cli.handoff", "run", None)
        .observe(|| Ok::<_, String>(handoff(&capture, args)))?;
    let matching_stage = Stage::new("cli.matching", "run", None).enter();
    let matching = rstyle_ccgraph::pipeline::evaluate_selected(
        &handoff.admitted,
        &config,
        args.max_pairs,
        matching_profile(args),
    );
    matching_stage.finish(Ok::<_, String>(()))?;

    let accepted = matching
        .pairs
        .iter()
        .filter(|p| p.decision == Decision::Accepted)
        .count();
    let complete =
        capture.failures.is_empty() && handoff.failures.is_empty() && !matching.incomplete;
    let mut report = report::make(report::Context {
        cli,
        args,
        capture: &capture,
        handoff: &handoff,
        matching: &matching,
        complete,
    });
    if args.trace {
        Stage::new("cli.trace_partitions", "run", None)
            .observe(|| report::attach_wl_partitions(&mut report, &handoff))?;
    }
    Stage::new("cli.report_write", "run", None)
        .observe(|| report::write(&capture.directory.join("report.json"), &report))?;
    let output = Stage::new("cli.report_render", "run", None)
        .observe(|| report::render(cli.format, &report, accepted, &capture.directory))?;
    let code = if !complete {
        2
    } else {
        u8::from(args.fail_on_accepted_pairs && accepted > 0)
    };

    Ok((output, code))
}

fn handoff(capture: &capture::Capture, args: &Args) -> Handoff {
    let mut result = Handoff {
        source_inventory: Vec::new(),
        assessed: Vec::new(),
        admitted: Vec::new(),
        failures: Vec::new(),
        keys: BTreeSet::new(),
    };
    if !capture.failures.is_empty() {
        return result;
    }

    for unit in &capture.units {
        if let Err(error) = consume_unit(unit, capture, args, &mut result) {
            result.failures.push(error);
        }
    }
    result.assessed.sort_by(|a, b| a.key.cmp(&b.key));
    result
}

fn consume_unit(
    unit: &capture::Unit,
    capture: &capture::Capture,
    args: &Args,
    result: &mut Handoff,
) -> Result<(), String> {
    if unit.extraction_profile != capture.plan.extraction_profile {
        return Err("Unit extraction profile disagrees with plan".into());
    }
    let snapshot = capture::load_inventory(unit)?;
    let manifest = snapshot.json("unit.evidence.manifest.json")?;
    let mut bindings = Vec::new();

    let store = rstyle_pdg::unit_store::UnitStore::from_current_run(
        snapshot.bytes("unit.evidence.json")?,
        &manifest,
        |facts| {
            for function in facts {
                verify_frontend_invocation(function, unit, capture)?;
                verify_receiver_inventory(function, unit.extraction_profile)?;
                rstyle_pdg::data_library::verify_evidence(function)?;
            }
            bindings = nominal_bindings(facts, unit)?;
            Ok(())
        },
    )?;
    result
        .source_inventory
        .extend(source_inventory(unit, &snapshot)?);
    consume_store(
        Arc::new(store),
        bindings,
        &UnitRequest {
            unit,
            capture,
            args,
        },
        result,
    )
}

fn consume_store(
    store: Arc<rstyle_pdg::unit_store::UnitStore>,
    bindings: Vec<Option<rstyle_pdg::nominal_binding::Binding>>,
    request: &UnitRequest,
    result: &mut Handoff,
) -> Result<(), String> {
    let analysis = store.analysis();

    for (index, binding) in bindings.into_iter().enumerate() {
        let function = &store.functions()[index];
        let key = function_key(function, request.unit, request.capture)?;
        if !result.keys.insert(key.clone()) {
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
    rstyle_pdg::data_receivers::verify(facts).map(|_| ())
}

fn nominal_bindings(
    facts: &[HirFnFacts],
    unit: &capture::Unit,
) -> Result<Vec<Option<rstyle_pdg::nominal_binding::Binding>>, String> {
    if !unit
        .extraction_profile
        .supports("rust.verified-helper-calls")
    {
        return Ok(vec![None; facts.len()]);
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
    key: String,
    unit: &capture::Unit,
    index: usize,
) -> Function {
    Function {
        key,
        compilation_unit: format!(
            "{}:{}:test={}",
            unit.target.package, unit.target.name, unit.test
        ),
        unit_evidence: json!({"identity":function.store.identity(), "bundle": unit.directory.join("unit.evidence.json"), "function": u32::try_from(index).expect("checked unit function table")}),
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
    verify_selected_unit(facts, unit)?;
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

fn verify_selected_unit(facts: &HirFnFacts, unit: &capture::Unit) -> Result<(), String> {
    use rstyle_pdg::nominal_boundary::{CONTEXT_KEY, UnitContext};
    let context = facts.build.frontend_options.get(CONTEXT_KEY);
    if !unit.extraction_profile.is_data() {
        return if context.is_none() {
            Ok(())
        } else {
            Err("Unexpected selected-unit data context".into())
        };
    }
    let actual: UnitContext =
        Deserialize::deserialize(context.ok_or("Missing selected-unit data context")?)
            .map_err(|e| e.to_string())?;

    actual.transport()?;
    if actual != capture::selected_unit_context(&unit.target, &unit.invocation)? {
        return Err("Selected-unit context disagrees with frozen Cargo plan or invocation".into());
    }
    Ok(())
}

fn function_key(
    facts: &HirFnFacts,
    unit: &capture::Unit,
    capture: &capture::Capture,
) -> Result<String, String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(Value::as_str)
        .ok_or("missing captured source-file navigation")?;
    let mut build = serde_json::to_value(&facts.build).map_err(|e| e.to_string())?;

    build["frontend_options"]
        .as_object_mut()
        .ok_or("missing frontend options")?
        .remove("invocation");
    let invocation: Vec<_> = unit
        .invocation
        .iter()
        .map(|a| a.replace(capture.directory.to_string_lossy().as_ref(), "<run>"))
        .collect();
    let identity = json!([
        unit.target.package,
        unit.target.name,
        unit.target.kind,
        unit.test,
        file,
        facts.fn_span,
        facts.name,
        build,
        invocation
    ]);
    Ok(format!(
        "function:{}",
        digest(&serde_json::to_vec(&identity).map_err(|e| e.to_string())?)
    ))
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
    function.key = record.key.clone();
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
