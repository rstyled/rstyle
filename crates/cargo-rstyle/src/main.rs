//! `cargo rstyle`: deterministic Rust style and side-effect analysis.
//!
//! Exit codes: 0 clean, 1 violations found, 2 tool/config/compile error.

mod clones;
mod collect;
mod diagnostic_process;
mod ignore;
mod run;
mod workspace;

use clap::{Args, Parser, Subcommand, ValueEnum};
use rstyle_core::config::Config;
use rstyle_core::facts::FnFacts;
use rstyle_core::ledger::{self, Opts};
use rstyle_core::normalize::Rules;
use rstyle_core::{cluster, patterns_report, report};
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
    /// Ignore ledger (default: <workspace root>/rstyle-ignore.toml).
    #[arg(long, global = true)]
    ignore_file: Option<PathBuf>,
    /// Print an agent skill (SKILL.md) describing how to use rstyle, then exit.
    #[arg(long)]
    skill: bool,
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

/// Facts directory names under `target/rstyle`. Dependence graphs are large, so only
/// `patterns` asks the driver for them and they live apart from the style facts.
const STYLE_FACTS: &str = "facts";
const SEMANTIC_FACTS: &str = "facts-sem";

/// Agent-facing usage doc, embedded so it always matches the binary.
const SKILL: &str = include_str!("../SKILL.md");

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

// rstyle-allow(side_effect_density): IO boundary: reads rstyle.toml
fn load_config(explicit: Option<&Path>, root: &Path) -> Result<Config, String> {
    let default = root.join("rstyle.toml");
    let path = match explicit {
        Some(path) => path,
        None if default.exists() => &default,
        None => return Ok(Config::default()),
    };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs the driver over the selected crates and reads back their facts.
fn gather(cli: &Cli, facts_dir: &str, pdg: bool) -> Result<(Vec<FnFacts>, PathBuf), String> {
    let meta = workspace::load(cli.manifest_path.as_deref())?;

    if let Some(name) = meta.unknown_packages(&cli.packages).first() {
        return Err(format!("package `{name}` is not in this workspace"));
    }
    let out_dir = meta.target_directory.join("rstyle").join(facts_dir);
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let target_dir = meta.target_directory.join("rstyle").join("build");
    let driver = run::driver_path()?;
    let paths = run::Paths {
        driver: &driver,
        root: &meta.workspace_root,
        target_dir: &target_dir,
        out_dir: &out_dir,
        pdg,
    };
    let selection = run::Selection {
        packages: &cli.packages,
        all_targets: cli.all_targets,
        manifest: cli.manifest_path.as_deref(),
    };
    run::cargo_check(&paths, &selection)?;
    let crates = meta.crate_names(&cli.packages);
    let wanted = collect::Wanted {
        crates: &crates,
        all_targets: cli.all_targets,
    };
    let facts = collect::read_facts(&out_dir, &wanted)?;
    Ok((facts, meta.workspace_root.clone()))
}

fn analyze(cli: &Cli) -> Result<report::Report, String> {
    let (facts, root) = gather(cli, STYLE_FACTS, false)?;
    let config = load_config(cli.config.as_deref(), &root)?;
    rstyle_core::evaluate(facts, &config)
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

/// Exit code 1: the run found something to act on (violations, or candidates under
/// `--fail-on-candidates`).
const FOUND: u8 = 1;

fn patterns(cli: &Cli, args: &PatternsArgs) -> Result<(String, u8), String> {
    let (facts, root) = gather(cli, SEMANTIC_FACTS, true)?;
    let ledger = match args.no_ignore {
        true => None,
        false => Some(ignore::load(&ignore::ledger_path(
            cli.ignore_file.as_deref(),
            &root,
        ))?),
    };
    let options = patterns_report::Options {
        min_score_milli: args.display.min_score_milli(),
        top: args.display.top,
        ledger,
        ..mine_options(&args.mine)
    };
    let report = patterns_report::run_owned(facts, &options);
    let code = u8::from(args.fail_on_candidates && !report.candidates.is_empty()) * FOUND;
    let output = match cli.format {
        Format::Text => patterns_text(&report, args.show_ignored),
        Format::Json => patterns_report::json(&report).map_err(|e| e.to_string())?,
    };

    Ok((output, code))
}

fn patterns_text(report: &patterns_report::PatternsReport, show_ignored: bool) -> String {
    let listing =
        (show_ignored && report.ignored_count > 0).then(|| patterns_report::show_ignored(report));

    std::iter::once(patterns_report::text(report))
        .chain(listing)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What `ignore` works on: the ledger on disk and the current, unfiltered candidates.
struct Session {
    path: PathBuf,
    ledger: ledger::Ledger,
    candidates: Vec<rstyle_core::candidates::Candidate>,
    facts: Vec<FnFacts>,
}

fn session(cli: &Cli, args: &IgnoreArgs) -> Result<Session, String> {
    let (facts, root) = gather(cli, SEMANTIC_FACTS, true)?;
    let path = ignore::ledger_path(cli.ignore_file.as_deref(), &root);
    let ledger = ignore::load(&path)?;
    let candidates = patterns_report::all_candidates(&facts, &mine_options(&args.mine));

    Ok(Session {
        path,
        ledger,
        candidates,
        facts,
    })
}

fn ignore_status(s: &Session, filter: Option<&str>) -> String {
    let mut statuses = ledger::status::status(&s.ledger, &s.candidates, &s.facts);
    statuses.retain(|st| filter.is_none_or(|id| st.id.starts_with(id)));

    ledger::status::text(&statuses)
}

fn ignore_prune(s: Session, dry_run: bool) -> Result<String, String> {
    let (ledger, report) = ignore::prune(s.ledger, &s.candidates, dry_run);
    if !dry_run {
        ignore::save(&s.path, &ledger)?;
    }

    Ok(report)
}

fn ignore_dismiss(s: Session, id: &str, opts: Opts) -> Result<String, String> {
    let (ledger, message) = ignore::dismiss(s.ledger, &s.candidates, id, opts)?;
    ignore::save(&s.path, &ledger)?;

    Ok(message)
}

fn ignore_cmd(cli: &Cli, args: &IgnoreArgs) -> Result<String, String> {
    let s = session(cli, args)?;
    let opts = Opts {
        allow_growth: args.allow_growth,
        reason: args.reason.clone(),
    };

    match (&args.id, args.status, args.prune) {
        (id, true, _) => Ok(ignore_status(&s, id.as_deref())),
        (_, _, true) => ignore_prune(s, args.dry_run),
        (Some(id), _, _) => ignore_dismiss(s, id, opts),
        _ => Err("give a candidate id, --status or --prune".into()),
    }
}

fn render(report: &report::Report, format: Format) -> Result<String, String> {
    match format {
        Format::Text => Ok(report::text(report)),
        Format::Json => report::json(report).map_err(|e| e.to_string()),
    }
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
    let cli = Cli::parse_from(args.map(|(_, a)| a));
    if cli.skill {
        print!("{SKILL}");
        return ExitCode::SUCCESS;
    }
    match &cli.command {
        Some(Command::Clones(args)) => return exit_with(clones::run(&cli, args)),
        Some(Command::Patterns(args)) => return exit_with(patterns(&cli, args)),
        Some(Command::Ignore(args)) => {
            return exit_with(ignore_cmd(&cli, args).map(|output| (output, 0)));
        }
        None => {}
    }
    let result = analyze(&cli).and_then(|report| Ok((render(&report, cli.format)?, report)));

    match result {
        Ok((output, report)) => {
            println!("{output}");
            ExitCode::from(report.exit_code())
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
