//! Reproducible evidence with independent completeness indicators.

use super::{Args, Handoff, capture};
use crate::{Cli, Format};
use rstyle_ccgraph::RunReport;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

pub struct Context<'a> {
    pub cli: &'a Cli,
    pub args: &'a Args,
    pub capture: &'a capture::Capture,
    pub handoff: &'a Handoff,
    pub matching: &'a RunReport,
    pub complete: bool,
}

#[derive(Serialize)]
struct Scope<'a> {
    packages: &'a [String],
    all_targets: bool,
    manifest_path: &'a Option<std::path::PathBuf>,
    build_options: &'a Args,
}

#[derive(Serialize)]
struct Report<'a> {
    scope: Scope<'a>,
    run_directory: &'a Path,
    toolchain: String,
    producer_binary_sha256: &'a str,
    orchestration_binary_sha256: &'a str,
    compiler_identity: &'a str,
    cargo_identity: &'a str,
    contract_revision: &'static str,
    extraction_policy: Policy,
    extraction_capabilities: &'static [&'static str],
    call_summaries: &'static [&'static str],
    schema_versions: SchemaVersions,
    producer_name: &'static str,
    producer_version: &'static str,
    expected_targets: &'a [capture::Target],
    expected_compilation_units: &'a Value,
    intended_primary_units: &'a [Value],
    observed_units: &'a [capture::Unit],
    capture_complete: bool,
    capture_failures: &'a [String],
    assessment_complete: bool,
    functions: &'a [super::Function],
    owned_source_inventory: &'a [Value],
    handoff_failures: &'a [String],
    pair_evaluation_complete: bool,
    pair_enumeration_complete: bool,
    matching: Value,
    claim: &'static str,
}

#[derive(Serialize)]
struct Policy {
    id: String,
    version: String,
    semantics: &'static str,
}

#[derive(Serialize)]
struct SchemaVersions {
    pdg: &'static str,
    cfg: &'static str,
}

pub fn make(ctx: Context<'_>) -> Result<Value, String> {
    let matching = matching_report(ctx.matching, ctx.args.trace)?;
    let scope = Scope {
        packages: &ctx.cli.packages,
        all_targets: ctx.cli.all_targets,
        manifest_path: &ctx.cli.manifest_path,
        build_options: ctx.args,
    };

    let report = Report {
        scope,
        run_directory: &ctx.capture.directory,
        toolchain: crate::run::pinned_toolchain(),
        producer_binary_sha256: &ctx.capture.plan.driver_sha256,
        orchestration_binary_sha256: &ctx.capture.plan.orchestrator_sha256,
        compiler_identity: &ctx.capture.plan.compiler_identity,
        cargo_identity: &ctx.capture.plan.cargo_identity,
        contract_revision: "04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f",
        extraction_policy: Policy {
            id: ctx.args.extraction_profile.policy().id,
            version: ctx.args.extraction_profile.policy().version,
            semantics: "bounded normal flow; exceptional transfers and cleanup omitted",
        },
        extraction_capabilities: ctx.args.extraction_profile.capabilities(),
        call_summaries: ctx.args.extraction_profile.summaries(),
        schema_versions: SchemaVersions {
            pdg: "0.1.0",
            cfg: "0.2.0",
        },
        producer_name: "rstyle-hir-extraction",
        producer_version: env!("CARGO_PKG_VERSION"),
        expected_targets: &ctx.capture.plan.targets,
        expected_compilation_units: &ctx.capture.expected_unit_graph,
        intended_primary_units: &ctx.capture.plan.primary_units,
        observed_units: &ctx.capture.units,
        capture_complete: ctx.capture.failures.is_empty(),
        capture_failures: &ctx.capture.failures,
        assessment_complete: ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty(),
        functions: &ctx.handoff.assessed,
        owned_source_inventory: &ctx.handoff.source_inventory,
        handoff_failures: &ctx.handoff.failures,
        pair_evaluation_complete: ctx.complete,
        pair_enumeration_complete: enumeration_complete(&ctx),
        matching,
        claim: "Accepted pairs are similarity decisions, not equivalence or safe refactoring.",
    };
    serde_json::to_value(report).map_err(|e| e.to_string())
}

fn enumeration_complete(ctx: &Context<'_>) -> bool {
    let count = ctx.handoff.admitted.len();
    let expected = count.checked_mul(count.saturating_sub(1)).map(|n| n / 2);

    ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && expected == Some(ctx.matching.pairs.len())
}

fn matching_report(report: &RunReport, trace: bool) -> Result<Value, String> {
    let mut value = serde_json::to_value(report).map_err(|e| e.to_string())?;
    let pairs = value["pairs"]
        .as_array_mut()
        .ok_or("missing serialized pair list")?;
    for pair in pairs {
        format_pair(pair, trace)?;
    }

    Ok(value)
}

fn format_pair(pair: &mut Value, trace: bool) -> Result<(), String> {
    let stages = pair["stages"].as_array_mut().ok_or("missing stages")?;
    let status = evaluation_status(stages);
    if !trace {
        for stage in stages {
            strip_histograms(stage);
        }
    }

    pair["evaluation_status"] = Value::String(status.into());
    if status == "failed" || status == "incompatible" {
        pair["decision"] = Value::Null;
    }
    Ok(())
}

fn evaluation_status(stages: &[Value]) -> &'static str {
    if stages.iter().any(|s| s["outcome"].get("Error").is_some()) {
        return "failed";
    }
    if stages
        .iter()
        .any(|s| s["outcome"].get("Ineligible").is_some())
    {
        return "incompatible";
    }
    "evaluated"
}

fn strip_histograms(stage: &mut Value) {
    if let Some(detail) = stage["detail"].as_object_mut() {
        detail.retain(|key, _| !key.starts_with("round_"));
    }
}

pub fn render(
    format: Format,
    report: &Value,
    accepted: usize,
    directory: &Path,
) -> Result<String, String> {
    match format {
        Format::Json => serde_json::to_string_pretty(report).map_err(|e| e.to_string()),
        Format::Text => Ok(format!(
            "CCGraph strict: {} units; {}; {} pairs, {accepted} accepted; capture={}, assessment={}, enumeration={}, evaluation={}.\nReport: {}\nAccepted pairs indicate similarity, not equivalence or safe refactoring.",
            report["observed_units"].as_array().map_or(0, Vec::len),
            coverage_summary(report),
            report["matching"]["pairs"].as_array().map_or(0, Vec::len),
            report["capture_complete"],
            report["assessment_complete"],
            report["pair_enumeration_complete"],
            report["pair_evaluation_complete"],
            directory.join("report.json").display()
        )),
    }
}

fn coverage_summary(report: &Value) -> String {
    let functions = report["functions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let counts = ["admitted", "excluded", "unknown", "failed"].map(|outcome| {
        let count = functions.iter().filter(|f| f["outcome"] == outcome).count();
        format!("{count} {outcome}")
    });

    format!("{} assessed ({})", functions.len(), counts.join(", "))
}

pub fn attach_wl_partitions(report: &mut Value, handoff: &Handoff) -> Result<(), String> {
    let pairs = report["matching"]["pairs"]
        .as_array_mut()
        .ok_or("missing pair report")?;
    for pair in pairs {
        attach_pair_partition(pair, handoff)?;
    }

    Ok(())
}

fn attach_pair_partition(pair: &mut Value, handoff: &Handoff) -> Result<(), String> {
    let Some(depth) = pair["stages"][6]["detail"]["depth"].as_str() else {
        return Ok(());
    };
    let depth: usize = depth.parse().map_err(|_| "invalid WL depth")?;
    let first = projected_function(handoff, &pair["first"])?;
    let second = projected_function(handoff, &pair["second"])?;
    let partitions = rstyle_ccgraph::wl::trace_partitions(&first, &second, depth);

    pair["wl_label_partitions"] = serde_json::to_value(partitions).map_err(|e| e.to_string())?;
    Ok(())
}

fn projected_function(
    handoff: &Handoff,
    key: &Value,
) -> Result<rstyle_ccgraph::ProjectedGraph, String> {
    let function = handoff
        .admitted
        .iter()
        .find(|f| key == &f.key)
        .ok_or("missing verified function for WL trace")?;
    rstyle_ccgraph::project(function)
        .map(|p| p.graph)
        .map_err(|e| format!("{e:?}"))
}
