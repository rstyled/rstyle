//! Opt-in, verified source-to-pair analysis; no legacy miner involvement.

mod advisory;
mod advisory_only;
mod analysis;
pub mod capture;
mod diagnostic_selection;
mod identity;
mod linking;
pub(crate) mod patterns_route;
mod planning;
mod report;
mod review;
mod selected_context;
pub(crate) mod style_route;
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
    path::{Path, PathBuf},
    sync::Arc,
};

/// Requested consumer population; never changes raw capture or strict matching.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum, Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompilerSourceScope {
    #[default]
    Captured,
    Authored,
}

impl CompilerSourceScope {
    fn is_default(&self) -> bool {
        *self == Self::Captured
    }
}

#[derive(ClapArgs, Serialize)]
#[command(group(clap::ArgGroup::new("local_analysis").args(["source_rules", "guard_clauses", "enum_dispatch", "capability_proposals", "mutation_rules", "mutation_effects", "effect_density", "compiler_advisory", "compiler_patterns"]).multiple(true)),
    group(clap::ArgGroup::new("compiler_presentation").args(["min_score", "top"]).requires("compiler_patterns").multiple(true)),
    group(clap::ArgGroup::new("compiler_mining").args(["hide_implements", "threshold", "min_coverage", "pair_coverage", "normalize", "single_call_guard", "max_holes"]).requires("compiler_patterns").multiple(true)),
    group(clap::ArgGroup::new("compiler_inputs").args(["compiler_advisory", "compiler_patterns", "compiler_complexity_top"]).multiple(true)),
    group(clap::ArgGroup::new("advisory_inputs").args(["compiler_advisory", "compiler_patterns", "compiler_complexity_top", "source_rules", "guard_clauses", "mutation_rules", "mutation_effects", "effect_density"]).multiple(true)),
    group(clap::ArgGroup::new("changed_analysis").args(["source_rules", "effect_density", "compiler_advisory", "compiler_patterns", "select_guards"]).multiple(true)),
    group(clap::ArgGroup::new("review_analysis").args(["helper_proposals", "enum_dispatch", "capability_proposals", "source_rules", "mutation_rules", "effect_density", "compiler_patterns", "compiler_advisory", "guard_clauses"]).multiple(true)),
    group(clap::ArgGroup::new("dismissal").args(["dismiss_helper_pair", "dismiss_enum_dispatch", "dismiss_guard", "dismiss_capability_pair", "dismiss_parameter", "dismiss_length", "dismiss_spacing", "dismiss_mutation_per_target", "dismiss_mutated_targets", "dismiss_density", "dismiss_compiler_pattern", "dismiss_compiler_diagnostic", "import_legacy_pattern"])),
    group(clap::ArgGroup::new("finding_analysis").args(["source_rules", "guard_clauses", "helper_proposals", "enum_dispatch", "capability_proposals", "mutation_rules", "effect_density", "compiler_advisory", "compiler_patterns"]).multiple(true)))]
pub struct Args {
    /// Internal root-style payload selection; full clones capture remains independent.
    #[arg(skip)]
    style_inputs_only: bool,
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
    /// Reuse compiler style collectors, effect propagation and fix plans per compilation unit.
    #[arg(long)]
    compiler_advisory: bool,
    /// Select core numeric rules by ID; omitted means all rules. Invalid suppressions always surface.
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        requires = "compiler_advisory"
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    compiler_diagnostic_rules: Vec<diagnostic_selection::Rule>,
    /// Diagnostic population; authored lists generated missing owners outside the requested scope.
    #[arg(long, value_enum, default_value_t = CompilerSourceScope::Captured, requires = "compiler_advisory")]
    #[serde(skip_serializing_if = "CompilerSourceScope::is_default")]
    compiler_diagnostic_scope: CompilerSourceScope,
    /// Rank up to N compiler functions per unit by existing COG then CC scores, including clean functions.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_complexity_top: Option<std::num::NonZeroUsize>,
    /// Rank captured inputs or authored functions, retaining generated owners outside the scope.
    #[arg(long, value_enum, default_value_t = CompilerSourceScope::Captured, requires = "compiler_complexity_top")]
    #[serde(skip_serializing_if = "CompilerSourceScope::is_default")]
    compiler_complexity_scope: CompilerSourceScope,
    /// Capture compiler advice without strict helper certification, admission or matching.
    #[arg(long, requires = "advisory_inputs", conflicts_with_all = ["matching_config", "trace", "fail_on_accepted_pairs", "explain_pairs", "helper_proposals", "enum_dispatch", "capability_proposals", "max_pairs"])]
    compiler_advisory_only: bool,
    /// Reuse compiler graphs and core proposals independently of numeric style diagnostics.
    #[arg(long)]
    compiler_patterns: bool,
    /// Select captured inputs or authored functions, explicitly excluding expansion signatures.
    #[arg(long, value_enum, default_value_t = CompilerSourceScope::Captured, requires = "compiler_patterns")]
    #[serde(skip_serializing_if = "CompilerSourceScope::is_default")]
    compiler_pattern_scope: CompilerSourceScope,
    /// Preview legacy ledger matches against raw scoped compiler proposals; never edits the ledger.
    #[arg(long, requires = "compiler_patterns")]
    #[serde(skip_serializing_if = "Option::is_none")]
    legacy_pattern_ledger: Option<PathBuf>,
    /// Import one explicit legacy match as a current exact compiler review.
    #[arg(long, num_args = 3, value_names = ["LEGACY_ID", "UNIT", "CANDIDATE_ID"], requires_all = ["legacy_pattern_ledger", "review_file", "replay_run"])]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    import_legacy_pattern: Vec<String>,
    /// Explicitly convert a growth dismissal to the current exact membership only.
    #[arg(long, requires = "import_legacy_pattern")]
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    legacy_current_members_only: bool,
    #[command(flatten, next_help_heading = "Compiler pattern mining")]
    #[serde(skip_serializing_if = "crate::MineArgs::is_default")]
    mine: crate::MineArgs,
    #[command(flatten, next_help_heading = "Compiler pattern presentation")]
    #[serde(skip_serializing_if = "crate::PatternDisplayArgs::is_default")]
    pattern_display: crate::PatternDisplayArgs,
    /// Select compiler proposals touching changed files at any usage site or definition.
    #[arg(long, requires_all = ["compiler_patterns", "changed_files"])]
    select_compiler_patterns: bool,
    /// Reuse core fix groups and parameter plans across captured dependency units; advisory only.
    #[arg(long, requires = "compiler_advisory")]
    compiler_plans: bool,
    /// TOML policy for compiler metrics, proposal effects and exclusions; independent of strict clones.
    #[arg(long, requires = "compiler_inputs")]
    compiler_advisory_config: Option<PathBuf>,
    /// JSON workspace-relative changed-file selection for source, density and compiler diagnostic presentation and finding exits.
    #[arg(long, requires = "changed_analysis")]
    changed_files: Option<PathBuf>,
    /// Suggest bounded guard-clause shapes from original source; advisory only.
    #[arg(long)]
    guard_clauses: bool,
    /// Select guard advice in changed files for presentation and finding exits; coverage stays global.
    #[arg(long, requires_all = ["guard_clauses", "changed_files"])]
    select_guards: bool,
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
    /// Evaluate bounded static effect density; unknown events prevent a ratio.
    #[arg(long)]
    effect_density: bool,
    /// Compare completed numeric findings with an explicit saved density baseline.
    #[arg(
        long,
        requires = "effect_density",
        conflicts_with = "write_density_baseline"
    )]
    density_baseline: Option<PathBuf>,
    /// Export a complete whole-scope density baseline to a new file.
    #[arg(long, requires = "effect_density", conflicts_with_all = ["changed_files", "density_baseline"])]
    write_density_baseline: Option<PathBuf>,
    /// Fail on new density findings or baseline policy/build-context drift.
    #[arg(long, requires = "density_baseline")]
    fail_on_new_density: bool,
    /// Compare completed source-rule findings with a saved whole-scope inventory.
    #[arg(
        long,
        requires = "source_rules",
        conflicts_with = "write_source_baseline"
    )]
    source_baseline: Option<PathBuf>,
    /// Publish a new complete source-rule inventory without overwriting a file.
    #[arg(long, requires = "source_rules", conflicts_with_all = ["changed_files", "source_baseline"])]
    write_source_baseline: Option<PathBuf>,
    /// Fail for new or changed selected source findings, or baseline header drift.
    #[arg(long, requires = "source_baseline")]
    fail_on_new_source: bool,
    /// Compare completed compiler diagnostics with an explicit saved inventory.
    #[arg(
        long,
        requires = "compiler_advisory",
        conflicts_with = "write_compiler_baseline"
    )]
    compiler_baseline: Option<PathBuf>,
    /// Export a complete compiler diagnostic inventory to a new file.
    #[arg(long, requires = "compiler_advisory", conflicts_with_all = ["changed_files", "compiler_baseline"])]
    write_compiler_baseline: Option<PathBuf>,
    /// Fail for new or changed compiler diagnostics, or policy/build scope drift.
    #[arg(long, requires = "compiler_baseline")]
    fail_on_new_compiler: bool,
    /// Compare compiler proposals with an explicit saved inventory.
    #[arg(
        long,
        requires = "compiler_patterns",
        conflicts_with = "write_compiler_pattern_baseline"
    )]
    compiler_pattern_baseline: Option<PathBuf>,
    /// Export a complete compiler proposal inventory to a new file.
    #[arg(long, requires = "compiler_patterns", conflicts_with_all = ["changed_files", "compiler_pattern_baseline"])]
    write_compiler_pattern_baseline: Option<PathBuf>,
    /// Fail for new or changed compiler proposals, or policy/build scope drift.
    #[arg(long, requires = "compiler_pattern_baseline")]
    fail_on_new_compiler_patterns: bool,
    /// TOML side_effects and workspace exclude policy for static density.
    #[arg(long, requires = "effect_density")]
    effect_config: Option<PathBuf>,
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
    /// Dismiss one current guard opportunity by saved-run function key and opportunity index.
    #[arg(long, num_args = 2, value_names = ["FUNCTION", "OPPORTUNITY_INDEX"], requires_all = ["review_file", "review_reason", "replay_run", "guard_clauses"])]
    dismiss_guard: Vec<String>,
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
    /// Dismiss one completed visible saved-run static density finding.
    #[arg(long, requires_all = ["review_file", "review_reason", "replay_run", "effect_density"])]
    dismiss_density: Option<String>,
    /// Dismiss a saved-run compiler diagnostic by numeric unit and zero-based index.
    #[arg(long, num_args = 2, value_names = ["UNIT", "DIAGNOSTIC_INDEX"], requires_all = ["review_file", "review_reason", "replay_run", "compiler_advisory"])]
    dismiss_compiler_diagnostic: Vec<String>,
    /// Dismiss one saved-run compiler proposal selected by numeric unit and candidate ID.
    #[arg(long, num_args = 2, value_names = ["UNIT", "CANDIDATE_ID"], requires_all = ["review_file", "review_reason", "replay_run", "compiler_patterns"])]
    dismiss_compiler_pattern: Vec<String>,
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
    compiler_advisory: Option<rstyle_core::config::Config>,
    matching: CcgraphConfig,
    analysis: Option<analysis::Config>,
    mutation: Option<analysis::mutation_reports::Settings>,
    effects: Option<analysis::effect_reports::Settings>,
    compiler_baseline: Option<review::compiler_baseline::Baseline>,
    compiler_pattern_baseline: Option<review::compiler_pattern_baseline::Baseline>,
    density_baseline: Option<review::density_baseline::Baseline>,
    source_baseline: Option<review::source_baseline::Baseline>,
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
        compiler_advisory: advisory::configuration(args)?,
        compiler_pattern_baseline: args
            .compiler_pattern_baseline
            .as_deref()
            .map(review::compiler_pattern_baseline::load)
            .transpose()?,
        compiler_baseline: args
            .compiler_baseline
            .as_deref()
            .map(review::compiler_baseline::load)
            .transpose()?,
        matching: rstyle_ccgraph::parse_config(&value).map_err(|e| e.message)?,
        analysis: analysis::configuration(args)?,
        mutation: analysis::mutation_reports::configuration(args)?,
        effects: analysis::effect_reports::configuration(args)?,
        source_baseline: args
            .source_baseline
            .as_deref()
            .map(review::source_baseline::load)
            .transpose()?,
        density_baseline: args
            .density_baseline
            .as_deref()
            .map(review::density_baseline::load)
            .transpose()?,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<rstyle_pdg::unit_store::UnitNamespace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact_kind: Option<&'static str>,
    bundle: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependency_closure: Option<PathBuf>,
    function: u32,
}

struct ReportFacts {
    owner: FactOwner,
    index: usize,
}

#[derive(Clone)]
enum FactOwner {
    Strict(Arc<rstyle_pdg::unit_store::UnitStore>),
    Advisory(Arc<Vec<HirFnFacts>>),
    Numeric(Arc<Vec<HirFnFacts<rstyle_pdg::HirRef>>>),
    Style(Arc<Vec<rstyle_pdg::StyleFnFacts>>),
}

/// Borrowed navigation and compiler observations shared by full and numeric inputs.
struct FactView<'a> {
    name: &'a str,
    fn_span: rstyle_pdg::ByteSpan,
    source: &'a rstyle_pdg::SourceFacts,
    signature: &'a rstyle_pdg::SignatureFacts,
    build: &'a rstyle_pdg::BuildFacts,
    source_diagnostics: &'a [rstyle_pdg::HirDiagnostic],
}

impl<'a, B, L> From<&'a HirFnFacts<B, L>> for FactView<'a> {
    fn from(facts: &'a HirFnFacts<B, L>) -> Self {
        Self {
            name: &facts.name,
            fn_span: facts.fn_span,
            source: &facts.source,
            signature: &facts.signature,
            build: &facts.build,
            source_diagnostics: &facts.source_diagnostics,
        }
    }
}

impl ReportFacts {
    fn functions(&self) -> Result<&[HirFnFacts], String> {
        match &self.owner {
            FactOwner::Strict(store) => Ok(store.functions()),
            FactOwner::Advisory(facts) => Ok(facts),
            FactOwner::Numeric(_) | FactOwner::Style(_) => {
                Err("Original HIR bodies were not requested".into())
            }
        }
    }

    fn full(&self) -> Result<&HirFnFacts, String> {
        Ok(&self.functions()?[self.index])
    }

    fn view(&self) -> FactView<'_> {
        match &self.owner {
            FactOwner::Strict(store) => (&store.functions()[self.index]).into(),
            FactOwner::Advisory(facts) => (&facts[self.index]).into(),
            FactOwner::Numeric(facts) => (&facts[self.index]).into(),
            FactOwner::Style(facts) => (&facts[self.index]).into(),
        }
    }

    fn namespace(&self) -> Option<rstyle_pdg::unit_store::UnitNamespace> {
        match &self.owner {
            FactOwner::Strict(store) => Some(*store.namespace()),
            FactOwner::Advisory(_) | FactOwner::Numeric(_) | FactOwner::Style(_) => None,
        }
    }
}

impl Serialize for ReportFacts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.full()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
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
    with_capture_verification(args.replay_run.is_some(), || run_generated(cli, args))
}

fn with_capture_verification<T>(replay: bool, execute: impl FnOnce() -> T) -> T {
    if replay {
        rstyle_pdg::verification_policy::strict(execute)
    } else {
        rstyle_pdg::verification_policy::generated(execute)
    }
}

fn run_generated(cli: &Cli, args: &Args) -> Result<(String, u8), String> {
    let config = configuration(cli, args)?;
    let capture = capture_input(cli, args)?;
    run_captured(cli, args, config, capture)
}

fn run_captured(
    cli: &Cli,
    args: &Args,
    config: Configuration,
    capture: capture::Capture,
) -> Result<(String, u8), String> {
    run_captured_with(
        cli,
        args,
        CapturedRun { config, capture },
        |report, accepted| report.publish(cli.format, accepted),
    )
}

struct CapturedRun {
    config: Configuration,
    capture: capture::Capture,
}

fn run_captured_with(
    cli: &Cli,
    args: &Args,
    captured: CapturedRun,
    publish: impl FnOnce(&report::Report<'_>, usize) -> Result<String, String>,
) -> Result<(String, u8), String> {
    let CapturedRun { config, capture } = captured;
    let handoff = Handoff::capture(&capture, args)?;
    let mut evaluation = (!args.compiler_advisory_only)
        .then(|| evaluate_matching(&handoff, &config.matching, args))
        .transpose()?;
    let partitions = matching_partitions(&mut evaluation);
    let matching = evaluation.as_ref().map(|value| &value.report);

    let accepted = accepted_count(matching);
    let capture_complete = capture.failures.is_empty();
    let assessment_complete = handoff.failures.is_empty();
    let complete =
        capture_complete && assessment_complete && matching.is_none_or(|report| !report.incomplete);
    let context = report::Context {
        cli,
        args,
        capture: &capture,
        handoff: &handoff,
        matching,
        partitions,
        complete,
        compiler_advisory_config: config.compiler_advisory.as_ref(),
        analysis_config: config.analysis.as_ref(),
        mutation_config: config.mutation.as_ref(),
        effect_config: config.effects.as_ref(),
        density_baseline: config.density_baseline.as_ref(),
        source_baseline: config.source_baseline.as_ref(),
        compiler_baseline: config.compiler_baseline.as_ref(),
        compiler_pattern_baseline: config.compiler_pattern_baseline.as_ref(),
        matching_config: &config.matching,
    };
    let consumers = report::Consumers::new(&context)?;
    let report = report::make(context, consumers);
    publish_baselines(&report, args, complete && analyses_complete(&report))?;
    publish_compiler_pattern_baseline(&report, args, complete && analyses_complete(&report))?;
    let output = publish(&report, accepted)?;
    let code = if complete && analyses_complete(&report) {
        u8::from(finding_exit_requested(&report, args, accepted))
    } else {
        2
    };

    Ok((output, code))
}

fn accepted_count(matching: Option<&rstyle_ccgraph::RunReport>) -> usize {
    matching
        .into_iter()
        .flat_map(|report| &report.pairs)
        .filter(|p| p.decision == Decision::Accepted)
        .count()
}

fn matching_partitions(
    evaluation: &mut Option<rstyle_ccgraph::pipeline::Evaluation>,
) -> rstyle_ccgraph::pipeline::WlPartitions {
    evaluation
        .as_mut()
        .map(|value| std::mem::take(&mut value.partitions))
        .unwrap_or_default()
}

fn publish_baselines(
    report: &report::Report<'_>,
    args: &Args,
    complete: bool,
) -> Result<(), String> {
    review::baseline_common::publish_optional(
        report.numeric_reviews.density_review.baseline.as_ref(),
        args.write_density_baseline.as_deref(),
        complete,
    )?;
    review::baseline_common::publish_optional(
        report.source_reviews.source_baseline.as_ref(),
        args.write_source_baseline.as_deref(),
        complete,
    )?;
    review::baseline_common::publish_optional(
        report
            .compiler_advisory
            .as_ref()
            .and_then(advisory::Report::baseline),
        args.write_compiler_baseline.as_deref(),
        complete,
    )?;
    Ok(())
}

fn publish_compiler_pattern_baseline(
    report: &report::Report<'_>,
    args: &Args,
    complete: bool,
) -> Result<(), String> {
    review::baseline_common::publish_optional(
        report
            .compiler_advisory
            .as_ref()
            .and_then(advisory::Report::pattern_baseline),
        args.write_compiler_pattern_baseline.as_deref(),
        complete,
    )
}

fn analyses_complete(report: &report::Report<'_>) -> bool {
    report
        .compiler_advisory
        .as_ref()
        .is_none_or(analysis::Completion::permits_success)
        && report.analysis.permits_success()
        && report.enum_dispatch.permits_success()
        && report.capability_proposals.permits_success()
        && report.mutation_analysis.permits_success()
        && report.mutation_effects.permits_success()
        && report.numeric_reviews.mutation_review.permits_success()
        && report.numeric_reviews.density_review.permits_success()
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
        && report.guard_review.permits_success()
        && report.source_reviews.permits_success()
        && report
            .review_status
            .as_ref()
            .is_none_or(analysis::Completion::permits_success)
}

fn finding_exit_requested(report: &report::Report<'_>, args: &Args, accepted: usize) -> bool {
    (args.fail_on_new_compiler_patterns
        && report
            .compiler_advisory
            .as_ref()
            .and_then(advisory::Report::pattern_baseline)
            .is_some_and(review::compiler_pattern_baseline::Report::gate_fails))
        || (args.fail_on_new_compiler
            && report
                .compiler_advisory
                .as_ref()
                .and_then(advisory::Report::baseline)
                .is_some_and(review::compiler_baseline::Report::gate_fails))
        || (args.fail_on_new_source && report.source_reviews.baseline_gate_fails())
        || (args.fail_on_new_density && report.numeric_reviews.density_review.baseline_gate_fails())
        || (args.fail_on_accepted_pairs && accepted > 0)
        || (args.fail_on_findings
            && (report
                .compiler_advisory
                .as_ref()
                .is_some_and(|report| report.finding_exit_count() > 0)
                || source_finding_exit_count(report) > 0
                || report
                    .abstraction_review
                    .dispatch_visible_or(report.enum_dispatch.findings)
                    > 0
                || report
                    .abstraction_review
                    .capability_visible_or(report.capability_proposals.findings)
                    > 0
                || report.mutation_analysis.visible_findings > 0
                || report.mutation_effects.finding_exit_count() > 0
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
    let capture = prepare_capture(cli, args)?;
    Ok(compile_capture(capture, cli, args))
}

fn prepare_capture(cli: &Cli, args: &Args) -> Result<capture::Capture, String> {
    Stage::new("cli.prepare", "run", None).observe(|| capture::prepare(cli, args))
}

fn compile_capture(mut capture: capture::Capture, cli: &Cli, args: &Args) -> capture::Capture {
    if let Err(error) = capture::compile(&mut capture, cli, args) {
        capture.failures.push(error);
    }
    capture
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
    if !args.compiler_advisory_only
        && !capture.plan.dependency_units.is_empty()
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
    if request.args.compiler_advisory_only {
        return advisory_only::consume(request, result);
    }
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
            owner: FactOwner::Strict(Arc::clone(&store)),
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
            namespace: function.namespace(),
            artifact_kind: function
                .namespace()
                .is_none()
                .then_some("advisory-raw-facts"),
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

fn verify_frontend_invocation<B, L>(
    facts: &HirFnFacts<B, L>,
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

fn verify_data_compiler_commit<B, L>(
    facts: &HirFnFacts<B, L>,
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

enum CompilerBaseline {
    Diagnostics,
    Patterns,
}

impl Args {
    fn compiler_inputs_requested(&self) -> bool {
        self.compiler_effects_requested() || self.compiler_complexity_top.is_some()
    }

    fn compiler_effects_requested(&self) -> bool {
        self.compiler_advisory || self.compiler_patterns
    }

    fn compiler_baseline_requested(&self, family: CompilerBaseline) -> bool {
        let (import, export) = match family {
            CompilerBaseline::Diagnostics => {
                (&self.compiler_baseline, &self.write_compiler_baseline)
            }
            CompilerBaseline::Patterns => (
                &self.compiler_pattern_baseline,
                &self.write_compiler_pattern_baseline,
            ),
        };
        import.is_some() || export.is_some()
    }
}
