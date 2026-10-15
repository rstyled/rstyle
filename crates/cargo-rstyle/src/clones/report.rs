//! Reproducible evidence with independent completeness indicators.

mod matching;

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
    pub partitions: rstyle_ccgraph::pipeline::WlPartitions,
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
pub struct Report<'a> {
    scope: Scope<'a>,
    run_directory: &'a Path,
    toolchain: String,
    compiler_identity: &'a str,
    cargo_identity: &'a str,
    contract_revision: &'static str,
    unit_bundle_schema: &'static str,
    function_identity_schema: &'static str,
    unit_contexts: &'a [super::identity::UnitContext],
    verification_policy: &'static str,
    extraction_selection: &'static str,
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
    build_contexts: &'a [super::UnitBuildContexts],
    owned_source_inventory: &'a [Value],
    handoff_failures: &'a [String],
    pair_evaluation_complete: bool,
    pair_enumeration_complete: bool,
    matching: matching::Matching<'a>,
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

// Report the CLI name separately from the frozen frontend evidence token.
fn extraction_selection(args: &Args) -> &'static str {
    if args.extraction_profile == rstyle_pdg::extraction_profile::ExtractionProfile::R0 {
        "r0"
    } else if args.extraction_profile
        == rstyle_pdg::extraction_profile::ExtractionProfile::HelpersV1
    {
        "hir-helpers-v1"
    } else if ctx_profile_is_data(args) {
        args.extraction_profile.token()
    } else {
        "hir-expanded-v1"
    }
}

fn ctx_profile_is_data(args: &Args) -> bool {
    args.extraction_profile.is_data()
}

pub fn make<'a>(ctx: Context<'a>) -> Report<'a> {
    let scope = Scope {
        packages: &ctx.cli.packages,
        all_targets: ctx.cli.all_targets,
        manifest_path: &ctx.cli.manifest_path,
        build_options: ctx.args,
    };

    Report {
        verification_policy: "trusted-generated",
        scope,
        run_directory: &ctx.capture.directory,
        toolchain: crate::run::pinned_toolchain(),
        compiler_identity: &ctx.capture.plan.compiler_identity,
        cargo_identity: &ctx.capture.plan.cargo_identity,
        contract_revision: "04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f",
        unit_bundle_schema: "rstyle.shared-unit/3",
        function_identity_schema: super::identity::SCHEMA,
        unit_contexts: &ctx.handoff.unit_contexts,
        extraction_selection: extraction_selection(ctx.args),
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
        build_contexts: &ctx.handoff.build_contexts,
        owned_source_inventory: &ctx.handoff.source_inventory,
        handoff_failures: &ctx.handoff.failures,
        pair_evaluation_complete: ctx.complete,
        pair_enumeration_complete: enumeration_complete(&ctx),
        matching: matching::Matching::new(ctx.matching, ctx.args.trace, ctx.partitions),
        claim: "Accepted pairs are similarity decisions, not equivalence or safe refactoring.",
    }
}

fn enumeration_complete(ctx: &Context<'_>) -> bool {
    let count = ctx.handoff.admitted.len();
    let expected = count.checked_mul(count.saturating_sub(1)).map(|n| n / 2);

    ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && expected == Some(ctx.matching.pairs.len())
}

pub fn render(
    format: Format,
    report: &Report<'_>,
    accepted: usize,
    directory: &Path,
) -> Result<String, String> {
    match format {
        Format::Json => serde_json::to_string_pretty(report).map_err(|e| e.to_string()),
        Format::Text => Ok(format!(
            "CCGraph strict: {} units; {}; {} pairs, {accepted} accepted; capture={}, assessment={}, enumeration={}, evaluation={}.\nReport: {}\nAccepted pairs indicate similarity, not equivalence or safe refactoring.",
            report.observed_units.len(),
            coverage_summary(report),
            report.matching.report.pairs.len(),
            report.capture_complete,
            report.assessment_complete,
            report.pair_enumeration_complete,
            report.pair_evaluation_complete,
            directory.join("report.json").display()
        )),
    }
}

fn coverage_summary(report: &Report<'_>) -> String {
    let functions = report.functions;
    let counts = ["admitted", "excluded", "unknown", "failed"].map(|outcome| {
        let count = functions
            .iter()
            .filter(|function| function.outcome == outcome)
            .count();
        format!("{count} {outcome}")
    });

    format!("{} assessed ({})", functions.len(), counts.join(", "))
}

pub fn write(path: &Path, report: &Report<'_>) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    write_to(std::io::BufWriter::new(file), report)
        .map_err(|error| format!("{}: {error}", path.display()))
}

impl Report<'_> {
    pub fn publish(&self, format: Format, accepted: usize) -> Result<String, String> {
        rstyle_pdg::diagnostics::Stage::new("cli.report_write", "run", None)
            .observe(|| write(&self.run_directory.join("report.json"), self))?;
        rstyle_pdg::diagnostics::Stage::new("cli.report_render", "run", None)
            .observe(|| render(format, self, accepted, self.run_directory))
    }
}

fn write_to(mut writer: impl std::io::Write, report: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer_pretty(&mut writer, report).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    struct FlushFailure;
    impl std::io::Write for FlushFailure {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::other("report flush failed"))
        }
    }

    #[test]
    fn streamed_report_requires_successful_flush() {
        let error =
            super::write_to(FlushFailure, &serde_json::json!({"complete":true})).unwrap_err();
        assert_eq!(error, "report flush failed");
    }
}
