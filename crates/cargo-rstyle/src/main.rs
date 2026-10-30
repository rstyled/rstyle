//! `cargo rstyle`: deterministic Rust style and side-effect analysis.
//!
//! Exit codes: 0 clean, 1 violations found, 2 tool/config/compile error.

#[cfg(test)]
mod cli_tests;
mod clones;
mod diagnostic_process;
mod ignore;
mod run;
mod workspace;

use clap::{Args, Parser, Subcommand, ValueEnum};
use rstyle_core::ledger;
use rstyle_core::normalize::Rules;
use rstyle_core::{cluster, patterns_report};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "cargo-rstyle", bin_name = "cargo rstyle", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Only analyze these packages (default: the whole workspace).
    #[arg(short, long = "package", global = true)]
    packages: Vec<String>,
    /// Include tests, examples and benches.
    #[arg(long, global = true)]
    all_targets: bool,
    #[arg(long, value_enum, default_value_t = Format::Text, global = true)]
    format: Format,
    /// Path to rstyle.toml (default: <workspace root>/rstyle.toml when present).
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    manifest_path: Option<PathBuf>,
    #[command(flatten)]
    style_inputs: StyleInputs,
    /// Ignore ledger (default: <workspace root>/rstyle-ignore.toml).
    #[arg(long, global = true)]
    ignore_file: Option<PathBuf>,
    /// Print an agent skill (SKILL.md) describing how to use rstyle, then exit.
    #[arg(long)]
    skill: bool,
}

impl Cli {
    fn validate_style_route(&self) -> Result<(), &'static str> {
        if self.command.is_some() && self.style_inputs.requested() {
            return Err("Root style inputs apply to the default style command only");
        }
        Ok(())
    }
}

#[derive(Args, Default)]
#[group(multiple = true)]
struct StyleInputs {
    /// Reevaluate an explicit saved compiler-style run without compiling.
    #[arg(id = "style_replay_run", long = "replay-run")]
    replay_run: Option<PathBuf>,
    /// Select compiler-style findings with a workspace-relative changed-file JSON manifest.
    #[arg(id = "style_changed_files", long = "changed-files")]
    changed_files: Option<PathBuf>,
    #[command(flatten)]
    baseline: StyleBaselineArgs,
    #[command(flatten)]
    review: StyleReviewArgs,
}

impl StyleInputs {
    fn requested(&self) -> bool {
        self.replay_run.is_some()
            || self.changed_files.is_some()
            || self.baseline.requested()
            || self.review.review_file.is_some()
    }
}

#[derive(Args, Default)]
#[group(multiple = true)]
struct StyleBaselineArgs {
    /// Compare compiler-style findings with a saved whole-scope inventory.
    #[arg(long, conflicts_with = "write_baseline")]
    baseline: Option<PathBuf>,
    /// Export a complete compiler-style inventory to a new file; findings still exit 1.
    #[arg(long, conflicts_with_all = ["baseline", "style_changed_files"])]
    write_baseline: Option<PathBuf>,
    /// Exit 1 for new or changed selected findings, or policy/build-context drift.
    #[arg(long, requires = "baseline")]
    fail_on_new: bool,
}

impl StyleBaselineArgs {
    fn requested(&self) -> bool {
        self.baseline.is_some() || self.write_baseline.is_some()
    }
}

#[derive(Args, Default)]
#[group(multiple = true)]
struct StyleReviewArgs {
    /// Apply individual saved finding reviews; lookup never writes.
    #[arg(long)]
    review_file: Option<PathBuf>,
    /// Inspect retained reviews, including stale and unavailable findings.
    #[arg(long, requires = "review_file")]
    review_status: bool,
    /// Dismiss one diagnostic from an explicit saved run, retaining its reason.
    #[arg(long, num_args = 2, value_names = ["UNIT", "DIAGNOSTIC_INDEX"], requires_all = ["review_file", "review_reason", "style_replay_run"], conflicts_with = "review_status")]
    dismiss_compiler_diagnostic: Vec<String>,
    /// Reason for the selected individual finding dismissal.
    #[arg(long, requires = "dismiss_compiler_diagnostic")]
    review_reason: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Capture HIR R0 evidence and evaluate exhaustive strict similarity pairs.
    Clones(Box<clones::Args>),
    /// Mine the workspace for latent shared abstractions (signature groups).
    Patterns(PatternsArgs),
    /// Dismiss a merge candidate you reviewed, list the ledger, or prune dead entries.
    Ignore(IgnoreArgs),
}

#[derive(Args)]
struct PatternsArgs {
    #[command(flatten)]
    route: PatternRouteArgs,
    #[command(flatten)]
    display: PatternDisplayArgs,
    /// Report every candidate, ignoring the ignore ledger for this run.
    #[arg(long)]
    no_ignore: bool,
    /// List the candidates the ignore ledger hid.
    #[arg(long)]
    show_ignored: bool,
    /// Exit 1 when any candidate is left after the ignore ledger (for CI).
    #[arg(long)]
    fail_on_candidates: bool,
    #[command(flatten)]
    mine: MineArgs,
}

/// Shared discovery selection for pattern listing and repository review commands.
#[derive(Args, Default)]
struct PatternInputArgs {
    /// Compiler population; authored excludes explicitly generated signatures.
    #[arg(long, value_enum)]
    compiler_pattern_scope: Option<clones::CompilerSourceScope>,
    /// Reevaluate an explicit saved compiler run without compiling.
    #[arg(long)]
    replay_run: Option<PathBuf>,
    /// Require the existing lockfile during compiler capture.
    #[arg(long)]
    locked: bool,
}

#[derive(Args, Default)]
struct PatternRouteArgs {
    #[command(flatten)]
    input: PatternInputArgs,
    /// Record an exact review of a saved compiler candidate without changing raw proposals.
    #[arg(long, num_args = 2, value_names = ["UNIT", "CANDIDATE_ID"], requires_all = ["review_file", "review_reason", "replay_run"], conflicts_with = "review_status")]
    dismiss_compiler_pattern: Vec<String>,
    /// Explain a direct compiler proposal review.
    #[arg(long, requires = "dismiss_compiler_pattern")]
    review_reason: Option<String>,
    /// Scoped compiler review store.
    #[arg(long)]
    review_file: Option<PathBuf>,
    /// Include scoped compiler review status.
    #[arg(long, requires = "review_file")]
    review_status: bool,
}

/// Presentation over ranked proposals; does not change mining or review identity.
#[derive(Args, Default, PartialEq, serde::Serialize)]
struct PatternDisplayArgs {
    /// Hide groups scoring below this (natural-log units, e.g. 0.5).
    #[arg(long)]
    min_score: Option<f64>,
    /// Show only the best N groups and candidates.
    #[arg(long)]
    top: Option<usize>,
}

impl PatternDisplayArgs {
    fn min_score_milli(&self) -> u32 {
        (self.min_score.unwrap_or_default().max(0.0) * 1000.0).round() as u32
    }

    fn is_default(&self) -> bool {
        self.min_score_milli() == 0 && self.top.is_none()
    }
}

#[derive(Args)]
struct IgnoreArgs {
    #[command(flatten)]
    route: PatternInputArgs,
    /// Candidate id (or a unique prefix of at least 4 digits) from `cargo rstyle patterns`.
    id: Option<String>,
    /// Keep hiding the candidate when more functions join it.
    #[arg(long, requires = "id", conflicts_with_all = ["status", "prune"])]
    allow_growth: bool,
    /// Why it was dismissed; shown when the candidate is listed or the entry is pruned.
    #[arg(long, requires = "id", conflicts_with_all = ["status", "prune"])]
    reason: Option<String>,
    /// List ledger entries with the current location and state of each function.
    #[arg(long, conflicts_with = "prune")]
    status: bool,
    /// Remove entries that match no current candidate.
    #[arg(long, conflicts_with = "id")]
    prune: bool,
    /// With --prune: report what would be removed without writing.
    #[arg(long, requires = "prune")]
    dry_run: bool,
    #[command(flatten)]
    mine: MineArgs,
}

/// Flags that change what `patterns` mines; `ignore` takes them too so ids line up.
#[derive(Args, Default, PartialEq, serde::Serialize)]
struct MineArgs {
    /// Evaluation oracle: ignore trait-impl links whose trait method id contains this text
    /// (repeatable; `*` hides all), so existing traits look like undiscovered abstractions.
    #[arg(long = "hide-implements")]
    hide_implements: Vec<String>,
    /// Minimum WL similarity (0..=1000) for two functions to be clustered [default: 600].
    #[arg(long)]
    threshold: Option<u32>,
    /// Minimum alignment coverage (0..=1000) for a member to join a cluster [default: 600].
    #[arg(long)]
    min_coverage: Option<u32>,
    /// Experiment: also cluster pairs whose alignment coverage (0..=1000) reaches this, even
    /// below the WL threshold [default: off].
    #[arg(long)]
    pair_coverage: Option<u32>,
    /// Experiment: canonicalize dependence graphs before comparing them. Comma-separated rules
    /// (`ctor`, `pred`, `case`, `exit`, `counter`, `cmp`, `try`) or `all` [default: none].
    #[arg(long, value_parser = Rules::parse)]
    normalize: Option<Rules>,
    /// Experiment: a body with a single call counts as substantial unless that call only
    /// forwards its parameters [default: off, bodies need 2 calls].
    #[arg(long)]
    single_call_guard: bool,
    /// Most differing parts a `parameterize` suggestion may have [default: 3].
    #[arg(long)]
    max_holes: Option<usize>,
}

impl MineArgs {
    fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// Agent-facing usage doc, embedded so it always matches the binary.
const SKILL: &str = include_str!("../SKILL.md");

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

fn config_path(explicit: Option<PathBuf>, root: &Path) -> Option<PathBuf> {
    explicit.or_else(|| {
        let default = root.join("rstyle.toml");
        default.exists().then_some(default)
    })
}

fn cluster_params(args: &MineArgs) -> cluster::Params {
    let default = cluster::Params::default();
    cluster::Params {
        threshold_milli: args.threshold.unwrap_or(default.threshold_milli),
        min_coverage_milli: args.min_coverage.unwrap_or(default.min_coverage_milli),
        pair_coverage_milli: args.pair_coverage.or(default.pair_coverage_milli),
        normalize: args.normalize.unwrap_or(default.normalize),
        single_call_guard: args.single_call_guard || default.single_call_guard,
        max_holes: args.max_holes.unwrap_or(default.max_holes),
    }
}

fn mine_options(mine: &MineArgs) -> patterns_report::Options {
    patterns_report::Options {
        hide_implements: mine.hide_implements.clone(),
        params: cluster_params(mine),
        ..patterns_report::Options::default()
    }
}

fn status_text(mut statuses: Vec<ledger::status::EntryStatus>, filter: Option<&str>) -> String {
    statuses.retain(|st| filter.is_none_or(|id| st.id.starts_with(id)));
    ledger::status::text(&statuses)
}

// rstyle-allow(side_effect_density): process boundary: prints the output and sets the exit code
fn exit_with(result: Result<(String, u8), String>) -> ExitCode {
    match result {
        Ok((output, code)) => {
            println!("{output}");
            ExitCode::from(code)
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

// rstyle-allow(side_effect_density): process boundary: prints the report and sets the exit code
fn main() -> ExitCode {
    if let Some(plan) = std::env::var_os("RSTYLE_CAPTURE_PLAN") {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().is_some_and(|a| a.ends_with("rustc")) {
            return match clones::capture::wrapper(Path::new(&plan), args) {
                Ok(code) => ExitCode::from(code),
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(2)
                }
            };
        }
    }

    // `cargo rstyle ...` invokes `cargo-rstyle rstyle ...`; drop that echo.
    let args = std::env::args()
        .enumerate()
        .filter(|(i, a)| !(*i == 1 && a == "rstyle"));
    let mut cli = Cli::parse_from(args.map(|(_, a)| a));
    if cli.skill {
        print!("{SKILL}");
        return ExitCode::SUCCESS;
    }
    if let Err(error) = cli.validate_style_route() {
        eprintln!("{error}");
        return ExitCode::from(2);
    }

    match cli.command.take() {
        Some(Command::Clones(args)) => return exit_with(clones::run(&cli, &args)),
        Some(Command::Patterns(args)) => return exit_with(clones::patterns_route::run(cli, args)),
        Some(Command::Ignore(args)) => {
            return exit_with(clones::patterns_route::ignore(cli, args));
        }
        None => {}
    }
    exit_with(clones::style_route::run(cli))
}
