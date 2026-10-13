//! Opt-in, verified source-to-pair analysis; no legacy miner involvement.

pub mod capture;
mod report;
use crate::Cli;
use clap::Args as ClapArgs;
use rstyle_ccgraph::{CcgraphConfig, Decision, ImportedFunction};
use rstyle_pdg::{EligibilityVerdict, HirFnFacts, classify, producer};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(ClapArgs, Serialize)]
pub struct Args {
    /// Extraction selection: r0, hir-expanded-v1, or verified local hir-helpers-v1.
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
    artifact_prefix: PathBuf,
    outcome: String,
    reasons: Vec<String>,
    facts: HirFnFacts,
    document_manifest: Option<producer::Manifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nominal_unit_binding: Option<rstyle_pdg::nominal_binding::Binding>,
    projection_trace: Option<Value>,
    extraction_evidence: Option<Value>,
}

struct Handoff {
    source_inventory: Vec<Value>,
    assessed: Vec<Function>,
    admitted: Vec<ImportedFunction>,
    failures: Vec<String>,
    keys: BTreeSet<String>,
}

struct FunctionInput<'a> {
    unit: &'a capture::Unit,
    index: usize,
    trace: bool,
}

impl FunctionInput<'_> {
    fn prefix(&self) -> PathBuf {
        self.unit.directory.join(format!("function-{}", self.index))
    }
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

pub fn run(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    let config = configuration(cli, args)?;
    let mut capture = capture::prepare(cli, args)?;
    if let Err(error) = capture::compile(&mut capture, cli, args) {
        capture.failures.push(error);
    }
    let handoff = handoff(&capture, args.trace);
    let matching = rstyle_ccgraph::pipeline::evaluate_with_pair_limit(
        &handoff.admitted,
        &config,
        args.max_pairs,
    );

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
    })?;
    if args.trace {
        report::attach_wl_partitions(&mut report, &handoff)?;
    }
    write_json(&capture.directory.join("report.json"), &report)?;
    let output = report::render(cli.format, &report, accepted, &capture.directory)?;
    let code = if !complete {
        2
    } else {
        u8::from(args.fail_on_accepted_pairs && accepted > 0)
    };

    Ok((output, code))
}

fn handoff(capture: &capture::Capture, trace: bool) -> Handoff {
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
        if let Err(error) = consume_unit(unit, capture, trace, &mut result) {
            result.failures.push(error);
        }
    }
    result.assessed.sort_by(|a, b| a.key.cmp(&b.key));
    result
}

fn consume_unit(
    unit: &capture::Unit,
    capture: &capture::Capture,
    trace: bool,
    result: &mut Handoff,
) -> Result<(), String> {
    if unit.extraction_profile != capture.plan.extraction_profile {
        return Err("Unit extraction profile disagrees with plan".into());
    }
    capture::verify_inventory(unit)?;
    result.source_inventory.extend(source_inventory(unit)?);

    let facts: Vec<HirFnFacts> = read_json(&unit.directory.join("facts.json"))?;
    for function in &facts {
        verify_frontend_invocation(function, unit, capture)?;
    }
    let bindings = nominal_bindings(&facts, unit)?;
    for (index, (function, binding)) in facts.into_iter().zip(bindings).enumerate() {
        let key = function_key(&function, unit, capture)?;
        if !result.keys.insert(key.clone()) {
            return Err(format!("duplicate function key {key}"));
        }
        let mut record = function_record(function, key, unit, index);
        record.nominal_unit_binding = binding;
        let input = FunctionInput { unit, index, trace };
        let admission = consume_function(&mut record, &input);
        result.add(record, admission);
    }
    Ok(())
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

fn source_inventory(unit: &capture::Unit) -> Result<Option<Value>, String> {
    if !unit
        .extraction_profile
        .supports("rust.conditional-source-correspondence")
    {
        return Ok(None);
    }
    let inventory: Vec<Value> = read_json(&unit.directory.join("source-inventory.json"))?;
    Ok(Some(
        json!({"compilation_unit":unit.target, "test":unit.test, "source_functions":inventory}),
    ))
}

fn function_record(
    function: HirFnFacts,
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
        artifact_prefix: unit.directory.join(format!("function-{index}")),
        outcome: "assessed".into(),
        reasons: Vec::new(),
        facts: function,
        document_manifest: None,
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
    let actual: Vec<String> = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    let suffix = actual
        .strip_prefix(unit.invocation.as_slice())
        .ok_or("frontend evidence belongs to another compiler invocation")?;
    let sysroot_only = suffix.len() == 2 && suffix[0] == "--sysroot" && !suffix[1].is_empty();

    if (!suffix.is_empty() && !sysroot_only)
        || facts.build.rustc_version.known_value() != capture.plan.compiler_identity.lines().next()
    {
        return Err("frontend invocation or pinned compiler identity disagrees with run".into());
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
    let assessment: Value = read_json(
        &input
            .unit
            .directory
            .join(format!("function-{}.assessment.json", input.index)),
    )?;
    if assessment["function"] != record.facts.name {
        return Err("assessment function mismatch".into());
    }
    let eligibility = classify(&record.facts);

    if eligibility.status != EligibilityVerdict::Eligible {
        if !assessment["manifest_file"].is_null() {
            return Err("unsupported function has a manifest".into());
        }
        record.exclude(eligibility);
        return Ok(None);
    }
    if assessment["manifest_file"] != format!("function-{}.manifest.json", input.index) {
        return Err(format!(
            "missing or ambiguous assessment manifest; producer errors: {}",
            assessment["extraction_errors"]
        ));
    }
    if !record
        .facts
        .build
        .frontend_options
        .get("source_identifier")
        .and_then(Value::as_str)
        .is_some_and(|name| !name.is_empty())
    {
        return Err("missing captured unqualified source identifier".into());
    }
    admit_serialized(record, input).map(Some)
}

fn admit_serialized(
    record: &mut Function,
    input: &FunctionInput<'_>,
) -> Result<ImportedFunction, String> {
    let serialized = read_serialized(input)?;
    let pair = producer::verify_serialized(&serialized, &record.facts).map_err(|e| e.join("; "))?;
    let mut function =
        rstyle_ccgraph::admit_produced(&pair, &record.facts).map_err(|e| e.join("; "))?;

    record.document_manifest = Some(serialized.manifest);
    function.key = record.key.clone();
    record.projection_trace = input.trace.then(|| projection_trace(&function));
    record_admission(record, &pair)?;

    Ok(function)
}

fn record_admission(record: &mut Function, pair: &producer::Pair) -> Result<(), String> {
    let cfg = serde_json::to_value(&pair.cfg.extraction_evidence).map_err(|e| e.to_string())?;
    let pdg = serde_json::to_value(&pair.pdg.extraction_evidence).map_err(|e| e.to_string())?;
    let evidence = serde_json::Map::from_iter([("cfg".into(), cfg), ("pdg".into(), pdg)]);

    record.outcome = "admitted".into();
    record.extraction_evidence = Some(Value::Object(evidence));
    Ok(())
}

fn read_serialized(input: &FunctionInput<'_>) -> Result<producer::SerializedPair, String> {
    let prefix = input.prefix();

    Ok(producer::SerializedPair {
        pdg_bytes: std::fs::read(prefix.with_extension("pdg.json")).map_err(|e| e.to_string())?,
        cfg_bytes: std::fs::read(prefix.with_extension("cfg.json")).map_err(|e| e.to_string())?,
        manifest: read_json(&prefix.with_extension("manifest.json"))?,
    })
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
