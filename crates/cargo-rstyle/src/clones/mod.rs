//! Opt-in, verified source-to-pair analysis; no legacy miner involvement.

pub mod capture;
use crate::{Cli, Format};
use clap::Args as ClapArgs;
use rstyle_ccgraph::{CcgraphConfig, Decision, ImportedFunction, RunReport};
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
    /// JSON matching overrides, separate from legacy rstyle.toml.
    #[arg(long)]
    matching_config: Option<PathBuf>,
    /// Include conformance evidence (projection traces and WL round histograms).
    #[arg(long)]
    trace: bool,
    /// Exit 1 if any pair is accepted; similarity does not prove equivalence.
    #[arg(long)]
    fail_on_accepted_pairs: bool,
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
    outcome: String,
    reasons: Vec<String>,
    facts: HirFnFacts,
    document_manifest: Option<producer::Manifest>,
    projection_trace: Option<Value>,
}

struct Handoff {
    assessed: Vec<Function>,
    admitted: Vec<ImportedFunction>,
    failures: Vec<String>,
    keys: BTreeSet<String>,
}

pub fn run(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    let config = configuration(cli, args)?;
    let mut capture = capture::prepare(cli)?;
    capture::compile(&mut capture, cli, args)?;
    let handoff = handoff(&capture, args.trace);
    let matching = rstyle_ccgraph::evaluate_all(&handoff.admitted, &config);
    let accepted = matching
        .pairs
        .iter()
        .filter(|p| p.decision == Decision::Accepted)
        .count();
    let complete =
        capture.failures.is_empty() && handoff.failures.is_empty() && !matching.incomplete;
    let report = report(cli, args, &capture, &handoff, &matching, complete)?;
    write_json(&capture.directory.join("report.json"), &report)?;
    let output = render(cli.format, &report, accepted, &capture.directory)?;
    let code = if !complete {
        2
    } else {
        u8::from(args.fail_on_accepted_pairs && accepted > 0)
    };
    Ok((output, code))
}

fn handoff(capture: &capture::Capture, trace: bool) -> Handoff {
    let mut result = Handoff {
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
    capture::verify_inventory(unit)?;
    let facts: Vec<HirFnFacts> = read_json(&unit.directory.join("facts.json"))?;
    for (index, function) in facts.into_iter().enumerate() {
        let key = function_key(&function, unit, capture)?;
        if !result.keys.insert(key.clone()) {
            return Err(format!("duplicate function key {key}"));
        }
        let mut record = Function {
            key,
            compilation_unit: format!(
                "{}:{}:test={}",
                unit.target.package, unit.target.name, unit.test
            ),
            outcome: "assessed".into(),
            reasons: Vec::new(),
            facts: function,
            document_manifest: None,
            projection_trace: None,
        };
        if let Err(error) = consume_function(&mut record, unit, index, trace, &mut result.admitted)
        {
            record.outcome = "failed".into();
            record.reasons.push(error.clone());
            result.failures.push(error);
        }
        result.assessed.push(record);
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
    unit: &capture::Unit,
    index: usize,
    trace: bool,
    admitted: &mut Vec<ImportedFunction>,
) -> Result<(), String> {
    let assessment: Value = read_json(
        &unit
            .directory
            .join(format!("function-{index}.assessment.json")),
    )?;
    if assessment["function"] != record.facts.name {
        return Err("assessment function mismatch".into());
    }
    let eligibility = classify(&record.facts);
    if eligibility.status != EligibilityVerdict::Eligible {
        if !assessment["manifest_file"].is_null() {
            return Err("unsupported function has a manifest".into());
        }
        record.outcome = match eligibility.status {
            EligibilityVerdict::Unknown => "unknown",
            _ => "excluded",
        }
        .into();
        record.reasons = eligibility
            .diagnostics
            .iter()
            .map(|d| d.reason.clone())
            .collect();
        return Ok(());
    }
    admit_serialized(record, unit, index, trace, admitted)
}

fn admit_serialized(
    record: &mut Function,
    unit: &capture::Unit,
    index: usize,
    trace: bool,
    admitted: &mut Vec<ImportedFunction>,
) -> Result<(), String> {
    let prefix = unit.directory.join(format!("function-{index}"));
    let manifest: producer::Manifest = read_json(&prefix.with_extension("manifest.json"))?;
    let serialized = producer::SerializedPair {
        pdg_bytes: std::fs::read(prefix.with_extension("pdg.json")).map_err(|e| e.to_string())?,
        cfg_bytes: std::fs::read(prefix.with_extension("cfg.json")).map_err(|e| e.to_string())?,
        manifest: manifest.clone(),
    };
    let pair = producer::verify_serialized(&serialized, &record.facts).map_err(|e| e.join("; "))?;
    let mut function =
        rstyle_ccgraph::admit_produced(&pair, &record.facts).map_err(|e| e.join("; "))?;
    record.document_manifest = Some(manifest);
    function.key = record.key.clone();
    if trace {
        record.projection_trace = Some(projection_trace(&function));
    }
    record.outcome = "admitted".into();
    admitted.push(function);
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

fn report(
    cli: &Cli,
    args: &Args,
    capture: &capture::Capture,
    handoff: &Handoff,
    matching: &RunReport,
    complete: bool,
) -> Result<Value, String> {
    let mut matching = serde_json::to_value(matching).map_err(|e| e.to_string())?;
    if !args.trace {
        strip_histograms(&mut matching);
    }
    Ok(json!({
        "scope":{"packages":cli.packages,"all_targets":cli.all_targets,"manifest_path":cli.manifest_path,"build_options":args},
        "run_directory":capture.directory,"toolchain":crate::run::pinned_toolchain(),
        "contract_revision":"04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f",
        "extraction_policy":{"id":"compat.rust.hir-r0","version":"0.1.0","semantics":"bounded normal flow; exceptional transfers and cleanup omitted"},
        "expected_targets":capture.plan.targets,"observed_units":capture.units,
        "capture_complete":capture.failures.is_empty(),"capture_failures":capture.failures,
        "assessment_complete":capture.failures.is_empty() && handoff.failures.is_empty(),
        "functions":handoff.assessed,"handoff_failures":handoff.failures,
        "pair_evaluation_complete":complete,"matching":matching,
        "claim":"Accepted pairs are similarity decisions, not equivalence or safe refactoring."
    }))
}

fn strip_histograms(report: &mut Value) {
    if let Some(pairs) = report["pairs"].as_array_mut() {
        for pair in pairs {
            if let Some(stages) = pair["stages"].as_array_mut() {
                for stage in stages {
                    if let Some(detail) = stage["detail"].as_object_mut() {
                        detail.retain(|key, _| !key.starts_with("round_"));
                    }
                }
            }
        }
    }
}

fn render(
    format: Format,
    report: &Value,
    accepted: usize,
    directory: &Path,
) -> Result<String, String> {
    match format {
        Format::Json => serde_json::to_string_pretty(report).map_err(|e| e.to_string()),
        Format::Text => Ok(format!(
            "CCGraph strict: {} assessed functions, {} pairs, {accepted} accepted; capture={}, assessment={}, pairs={}.\nReport: {}\nAccepted pairs indicate similarity, not equivalence or safe refactoring.",
            report["functions"].as_array().map_or(0, Vec::len),
            report["matching"]["pairs"].as_array().map_or(0, Vec::len),
            report["capture_complete"],
            report["assessment_complete"],
            report["pair_evaluation_complete"],
            directory.join("report.json").display()
        )),
    }
}
