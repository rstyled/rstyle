//! Reproducible evidence with independent completeness indicators.

mod consumers;
pub(super) use consumers::Consumers;
mod functions;
mod style;

use super::{Args, Handoff, capture};
use crate::{Cli, Format};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

/// One capture's inputs, and the policy and baselines reports are evaluated against.
pub struct Context<'a> {
    pub pattern_ledger: Option<&'a rstyle_core::ledger::Ledger>,
    pub compiler_advisory_config: Option<&'a rstyle_core::config::Config>,
    pub cli: &'a Cli,
    pub args: &'a Args,
    pub capture: &'a capture::Capture,
    pub handoff: &'a Handoff,
    pub complete: bool,
    pub compiler_pattern_baseline: Option<&'a super::review::compiler_pattern_baseline::Baseline>,
    pub compiler_baseline: Option<&'a super::review::compiler_baseline::Baseline>,
    pub analysis_config: Option<&'a super::analysis::Config>,
}

#[derive(Serialize)]
struct Scope<'a> {
    selection_origin: &'static str,
    packages: Option<&'a [String]>,
    all_targets: Option<bool>,
    manifest_path: Option<&'a Path>,
    build_options_origin: &'static str,
    build_options: &'a Args,
}

#[derive(Serialize)]
pub struct Report<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_advisory: Option<super::advisory::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_status: Option<super::review::status::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_prune: Option<super::review::prune::Report>,
    scope: Scope<'a>,
    run_directory: &'a Path,
    toolchain: String,
    compiler_identity: &'a str,
    function_identity_schema: &'static str,
    producer_name: &'static str,
    producer_version: &'static str,
    cargo_identity: &'a str,
    unit_contexts: &'a [super::identity::UnitContext],
    expected_targets: &'a [capture::Target],
    expected_compilation_units: &'a Value,
    intended_primary_units: &'a [Value],
    observed_units: &'a [capture::Unit],
    #[serde(flatten)]
    capture_status: CaptureStatus<'a>,
    functions: functions::Functions<'a>,
    build_contexts: &'a [super::UnitBuildContexts],
}

#[derive(Serialize)]
struct CaptureStatus<'a> {
    capture_complete: bool,
    capture_failures: &'a [String],
    assessment_complete: bool,
    handoff_failures: &'a [String],
}

impl<'a> CaptureStatus<'a> {
    fn new(ctx: &Context<'a>) -> Self {
        Self {
            capture_complete: ctx.capture.failures.is_empty(),
            capture_failures: &ctx.capture.failures,
            assessment_complete: ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty(),
            handoff_failures: &ctx.handoff.failures,
        }
    }
}

pub(super) fn make<'a>(ctx: Context<'a>, consumers: Consumers) -> Report<'a> {
    Report {
        compiler_advisory: consumers.compiler_advisory,
        review_status: consumers.review_status,
        review_prune: consumers.review_prune,
        scope: selected_scope(&ctx),
        run_directory: &ctx.capture.directory,
        toolchain: crate::run::pinned_toolchain(),
        compiler_identity: &ctx.capture.plan.compiler_identity,
        function_identity_schema: super::identity::SCHEMA,
        producer_name: "rstyle-driver",
        producer_version: env!("CARGO_PKG_VERSION"),
        cargo_identity: &ctx.capture.plan.cargo_identity,
        unit_contexts: &ctx.handoff.unit_contexts,
        expected_targets: &ctx.capture.plan.targets,
        expected_compilation_units: &ctx.capture.expected_unit_graph,
        intended_primary_units: &ctx.capture.plan.primary_units,
        observed_units: &ctx.capture.units,
        capture_status: CaptureStatus::new(&ctx),
        functions: functions::Functions::new(&ctx.handoff.assessed),
        build_contexts: &ctx.handoff.build_contexts,
    }
}

fn selected_scope<'a>(ctx: &Context<'a>) -> Scope<'a> {
    let replay = ctx.args.replay_run.is_some();
    Scope {
        selection_origin: if replay { "saved_run" } else { "fresh_capture" },
        packages: (!replay).then_some(ctx.cli.packages.as_slice()),
        all_targets: (!replay).then_some(ctx.cli.all_targets),
        manifest_path: if replay {
            None
        } else {
            ctx.cli.manifest_path.as_deref()
        },
        build_options_origin: if replay {
            "current consumer request; original effective build contexts are recorded in expected_compilation_units and build_contexts"
        } else {
            "fresh capture request"
        },
        build_options: ctx.args,
    }
}

pub fn write(path: &Path, report: &Report<'_>) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
    write_to(std::io::BufWriter::new(file), report)
        .map_err(|error| format!("{}: {error}", path.display()))
}

impl Report<'_> {
    pub fn publish(&self, format: Format) -> Result<String, String> {
        self.publish_with(|| self.render_published(format))
    }

    fn publish_with(
        &self,
        render: impl FnOnce() -> Result<String, String>,
    ) -> Result<String, String> {
        self.publish_archive()?;
        rstyle_capture::diagnostics::run_stage("cli.report_render", render)
    }

    fn publish_archive(&self) -> Result<(), String> {
        rstyle_capture::diagnostics::run_stage("cli.report_write", || {
            write(&self.run_directory.join("report.json"), self)
        })
    }

    fn render_published(&self, format: Format) -> Result<String, String> {
        match format {
            Format::Json => std::fs::read_to_string(self.run_directory.join("report.json"))
                .map_err(|error| error.to_string()),
            Format::Text => Ok(render_text(self, self.run_directory)),
        }
    }
}

fn render_text(report: &Report<'_>, directory: &Path) -> String {
    format!(
        "Compiler reports: {} units; capture={}, assessment={}.\nReport: {}{}{}{}",
        report.observed_units.len(),
        report.capture_status.capture_complete,
        report.capture_status.assessment_complete,
        directory.join("report.json").display(),
        report
            .compiler_advisory
            .as_ref()
            .map_or_else(String::new, |report| report.render()),
        report
            .review_status
            .as_ref()
            .map_or_else(String::new, |status| status.render()),
        report
            .review_prune
            .as_ref()
            .map_or_else(String::new, |pruning| pruning.render()),
    )
}

fn write_to(mut writer: impl std::io::Write, report: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut writer, report).map_err(|error| error.to_string())?;
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
