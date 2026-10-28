//! Reproducible evidence with independent completeness indicators.

mod consumers;
pub(super) use consumers::Consumers;
mod functions;
mod locations;
mod matching;
mod source_reviews;
pub(in crate::clones) use source_reviews::SourceReviews;

use super::{Args, Handoff, capture};
use crate::{Cli, Format};
use rstyle_ccgraph::RunReport;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Copy, clap::ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Detail {
    Full,
    Compact,
}

pub struct Context<'a> {
    pub cli: &'a Cli,
    pub args: &'a Args,
    pub capture: &'a capture::Capture,
    pub handoff: &'a Handoff,
    pub matching: &'a RunReport,
    pub partitions: rstyle_ccgraph::pipeline::WlPartitions,
    pub complete: bool,
    pub mutation_config: Option<&'a super::analysis::mutation_reports::Settings>,
    pub effect_config: Option<&'a super::analysis::effect_reports::Settings>,
    pub analysis_config: Option<&'a super::analysis::Config>,
    pub matching_config: &'a rstyle_ccgraph::CcgraphConfig,
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
    pub review: super::review::workflow::Report,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_status: Option<super::review::status::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_prune: Option<super::review::prune::Report>,
    pub abstraction_review: super::review::local_workflow::Report,
    #[serde(flatten)]
    pub source_reviews: SourceReviews,
    pub edit_planning: super::planning::Report,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_planning: Option<super::planning::parameters::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard_planning: Option<super::planning::guards::Report>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_coordination: Option<super::planning::coordination::Report>,
    #[serde(flatten)]
    pub numeric_reviews: consumers::NumericReviews,
    pub mutation_analysis: super::analysis::mutation_reports::Report<'a>,
    pub mutation_effects: super::analysis::effect_reports::Report<'a>,
    pub enum_dispatch: super::analysis::dispatch_reports::Report<'a>,
    pub capability_proposals: super::analysis::capability_reports::Report<'a>,
    #[serde(flatten)]
    pub pair_reports: super::analysis::pair_reports::Reports<'a>,
    pub analysis: super::analysis::Report<'a>,
    report_detail: Detail,
    scope: Scope<'a>,
    run_directory: &'a Path,
    toolchain: String,
    compiler_identity: &'a str,
    #[serde(flatten)]
    producer: ProducerMetadata,
    cargo_identity: &'a str,
    unit_contexts: &'a [super::identity::UnitContext],
    verification_policy: &'static str,
    extraction_selection: &'static str,
    extraction_policy: Policy,
    extraction_capabilities: &'static [&'static str],
    call_summaries: &'static [&'static str],
    expected_targets: &'a [capture::Target],
    expected_compilation_units: &'a Value,
    intended_primary_units: &'a [Value],
    intended_dependency_units: &'a [Value],
    observed_units: &'a [capture::Unit],
    capture_complete: bool,
    capture_failures: &'a [String],
    assessment_complete: bool,
    functions: functions::Functions<'a>,
    build_contexts: &'a [super::UnitBuildContexts],
    owned_source_inventory: &'a [Value],
    handoff_failures: &'a [String],
    pair_evaluation_complete: bool,
    pair_enumeration_complete: bool,
    matching: matching::Matching<'a>,
    accepted_pairs: Vec<locations::AcceptedPair<'a>>,
    claim: &'static str,
}

#[derive(Serialize)]
struct ProducerMetadata {
    contract_revision: &'static str,
    unit_bundle_schema: &'static str,
    function_identity_schema: &'static str,
    schema_versions: SchemaVersions,
    producer_name: &'static str,
    producer_version: &'static str,
}

impl ProducerMetadata {
    fn current() -> Self {
        Self {
            contract_revision: "04d9aa0db8b1f8e4a3fcad16622e2a4e3bc3e53f",
            unit_bundle_schema: rstyle_pdg::unit_store::BUNDLE_SCHEMA,
            function_identity_schema: super::identity::SCHEMA,
            schema_versions: SchemaVersions::current(),
            producer_name: "rstyle-hir-extraction",
            producer_version: env!("CARGO_PKG_VERSION"),
        }
    }
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

impl SchemaVersions {
    fn current() -> Self {
        Self {
            pdg: "0.1.0",
            cfg: "0.2.0",
        }
    }
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

pub(super) fn make<'a>(ctx: Context<'a>, consumers: Consumers<'a>) -> Report<'a> {
    Report {
        edit_planning: consumers.edit_planning,
        parameter_planning: consumers.parameter_planning,
        guard_planning: consumers.guard_planning,
        edit_coordination: consumers.edit_coordination,
        pair_reports: consumers.pairs,
        review: consumers.review,
        abstraction_review: consumers.abstraction_review,
        source_reviews: consumers.source_reviews,
        review_status: consumers.review_status,
        review_prune: consumers.review_prune,
        analysis: consumers.analysis,
        enum_dispatch: consumers.dispatch,
        capability_proposals: consumers.capabilities,
        mutation_analysis: consumers.mutation,
        numeric_reviews: consumers.numeric_reviews,
        mutation_effects: consumers.effects,
        report_detail: ctx.args.report_detail,
        verification_policy: ctx.verification_policy(),
        scope: selected_scope(&ctx),
        run_directory: &ctx.capture.directory,
        toolchain: crate::run::pinned_toolchain(),
        compiler_identity: &ctx.capture.plan.compiler_identity,
        cargo_identity: &ctx.capture.plan.cargo_identity,
        producer: ProducerMetadata::current(),
        unit_contexts: &ctx.handoff.unit_contexts,
        extraction_selection: extraction_selection(ctx.args),
        extraction_policy: extraction_policy(ctx.args),
        extraction_capabilities: ctx.args.extraction_profile.capabilities(),
        call_summaries: ctx.args.extraction_profile.summaries(),
        expected_targets: &ctx.capture.plan.targets,
        expected_compilation_units: &ctx.capture.expected_unit_graph,
        intended_primary_units: &ctx.capture.plan.primary_units,
        intended_dependency_units: &ctx.capture.plan.dependency_units,
        observed_units: &ctx.capture.units,
        capture_complete: ctx.capture.failures.is_empty(),
        capture_failures: &ctx.capture.failures,
        assessment_complete: ctx.capture.failures.is_empty() && ctx.handoff.failures.is_empty(),
        functions: functions::Functions::new(&ctx.handoff.assessed, ctx.args.report_detail),
        build_contexts: &ctx.handoff.build_contexts,
        owned_source_inventory: &ctx.handoff.source_inventory,
        handoff_failures: &ctx.handoff.failures,
        pair_evaluation_complete: ctx.complete,
        pair_enumeration_complete: enumeration_complete(&ctx),
        matching: matching::Matching::new(ctx.matching, ctx.args.trace, ctx.partitions),
        accepted_pairs: consumers.accepted,
        claim: "Accepted pairs are similarity decisions, not equivalence or safe refactoring.",
    }
}

impl<'a> Context<'a> {
    fn edit_planning(
        &self,
        helpers: &super::analysis::helper_reports::Report<'_>,
    ) -> Result<super::planning::Report, String> {
        super::planning::evaluate(helpers, self)
    }

    fn declaration_indices(&self) -> Result<super::analysis::declaration_indices::Indices, String> {
        super::analysis::declaration_indices::Indices::new(self.handoff, self.args)
    }

    fn dispatch_report(
        &self,
        indices: &super::analysis::declaration_indices::Indices,
    ) -> Result<super::analysis::dispatch_reports::Report<'a>, String> {
        super::analysis::dispatch_reports::evaluate(self.handoff, self.capture, self.args, indices)
    }

    fn capability_report(
        &self,
        indices: &super::analysis::declaration_indices::Indices,
    ) -> Result<super::analysis::capability_reports::Report<'a>, String> {
        super::analysis::capability_reports::evaluate(
            self.handoff,
            self.capture,
            self.args,
            indices,
        )
    }

    fn effect_report(&self) -> Result<super::analysis::effect_reports::Report<'a>, String> {
        super::analysis::effect_reports::evaluate(
            self.handoff,
            self.capture,
            self.args,
            self.effect_config,
        )
    }

    fn verification_policy(&self) -> &'static str {
        if self.args.replay_run.is_some() {
            "strict-replay"
        } else {
            "trusted-generated"
        }
    }

    fn mutation_report(&self) -> Result<super::analysis::mutation_reports::Report<'a>, String> {
        super::analysis::mutation_reports::evaluate(
            self.handoff,
            self.capture,
            self.args,
            self.mutation_config,
        )
    }

    fn local_analysis(&self) -> super::analysis::Report<'a> {
        super::analysis::evaluate(self.handoff, self.capture, self.analysis_config, self.args)
    }

    fn pair_reports(&self) -> Result<super::analysis::pair_reports::Reports<'a>, String> {
        super::analysis::pair_reports::evaluate(
            self.handoff,
            self.matching,
            super::analysis::pair_reports::Requests {
                explain: self.args.explain_pairs,
                helpers: self.args.helper_proposals,
                matching_complete: self.complete,
            },
        )
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

fn enumeration_complete(ctx: &Context<'_>) -> bool {
    let count = ctx.handoff.admitted.len();
    let expected = count.checked_mul(count.saturating_sub(1)).map(|n| n / 2);

    ctx.capture.failures.is_empty()
        && ctx.handoff.failures.is_empty()
        && expected == Some(ctx.matching.pairs.len())
}

fn render_text(report: &Report<'_>, accepted: usize, directory: &Path) -> String {
    let summary = format!(
        "CCGraph strict: {} units; {}; {} pairs, {accepted} accepted; capture={}, assessment={}, enumeration={}, evaluation={}.\nReport: {}\nAccepted pairs indicate similarity, not equivalence or safe refactoring.",
        report.observed_units.len(),
        coverage_summary(report),
        report.matching.report.pairs.len(),
        report.capture_complete,
        report.assessment_complete,
        report.pair_enumeration_complete,
        report.pair_evaluation_complete,
        directory.join("report.json").display()
    );
    let pairs = report
        .accepted_pairs
        .iter()
        .map(|pair| {
            format!(
                "\nAccepted pair: {} <> {} (score {}).",
                pair.first,
                pair.second,
                pair.displayed_score.unwrap_or("unavailable")
            )
        })
        .collect::<String>();

    format!(
        "{summary}{pairs}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        report.analysis.render(),
        report.pair_reports.render(),
        report.review.render(),
        report.abstraction_review.render(),
        report.source_reviews.render(),
        report
            .review_status
            .as_ref()
            .map_or_else(String::new, |status| status.render()),
        report
            .review_prune
            .as_ref()
            .map_or_else(String::new, |pruning| pruning.render()),
        report.enum_dispatch.render(),
        report.capability_proposals.render(),
        report.mutation_analysis.render(),
        report.numeric_reviews.mutation_review.render(),
        report.numeric_reviews.density_review.render(),
        report.mutation_effects.render(),
        planning_summary(report)
    )
}

fn planning_summary(report: &Report<'_>) -> String {
    format!(
        "{}{}{}{}",
        report.edit_planning.render(),
        report
            .parameter_planning
            .as_ref()
            .map_or_else(String::new, |planning| planning.render()),
        report
            .guard_planning
            .as_ref()
            .map_or_else(String::new, |planning| planning.render()),
        report
            .edit_coordination
            .as_ref()
            .map_or_else(String::new, |planning| planning.render())
    )
}

fn coverage_summary(report: &Report<'_>) -> String {
    let functions = report.functions.items;
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
            .observe(|| self.render_published(format, accepted))
    }

    fn render_published(&self, format: Format, accepted: usize) -> Result<String, String> {
        match format {
            Format::Json => std::fs::read_to_string(self.run_directory.join("report.json"))
                .map_err(|error| error.to_string()),
            Format::Text => Ok(render_text(self, accepted, self.run_directory)),
        }
    }
}

fn write_to(mut writer: impl std::io::Write, report: &impl Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut writer, report).map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

fn extraction_policy(args: &super::Args) -> Policy {
    Policy {
        id: args.extraction_profile.policy().id,
        version: args.extraction_profile.policy().version,
        semantics: if args
            .extraction_profile
            .supports("rust.mutable-scalar-formals")
        {
            "bounded flow; explicit checked failures for supported mutable scalar formals; general cleanup omitted"
        } else {
            "bounded normal flow; exceptional transfers and cleanup omitted"
        },
    }
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
