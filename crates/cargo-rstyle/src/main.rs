//! `cargo rstyle`: deterministic Rust style and side-effect analysis.
//!
//! Exit codes: 0 clean, 1 violations found, 2 tool/config/compile error.

mod collect;
mod run;
mod workspace;

use clap::{Args, Parser, Subcommand, ValueEnum};
use rstyle_core::config::Config;
use rstyle_core::facts::FnFacts;
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
    /// Print an agent skill (SKILL.md) describing how to use rstyle, then exit.
    #[arg(long)]
    skill: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Mine the workspace for latent shared abstractions (signature groups).
    Patterns(PatternsArgs),
}

#[derive(Args)]
struct PatternsArgs {
    /// Hide groups scoring below this (natural-log units, e.g. 0.5).
    #[arg(long, default_value_t = 0.0)]
    min_score: f64,
    /// Show only the best N groups and candidates.
    #[arg(long)]
    top: Option<usize>,
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

fn cluster_params(args: &PatternsArgs) -> cluster::Params {
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

fn patterns(cli: &Cli, args: &PatternsArgs) -> Result<String, String> {
    let (facts, _) = gather(cli, SEMANTIC_FACTS, true)?;
    let options = patterns_report::Options {
        min_score_milli: (args.min_score.max(0.0) * 1000.0).round() as u32,
        top: args.top,
        hide_implements: args.hide_implements.clone(),
        params: cluster_params(args),
    };
    let report = patterns_report::run(&facts, &options);
    match cli.format {
        Format::Text => Ok(patterns_report::text(&report)),
        Format::Json => patterns_report::json(&report).map_err(|e| e.to_string()),
    }
}

fn render(report: &report::Report, format: Format) -> Result<String, String> {
    match format {
        Format::Text => Ok(report::text(report)),
        Format::Json => report::json(report).map_err(|e| e.to_string()),
    }
}

// rstyle-allow(side_effect_density): process boundary: prints the report and sets the exit code
fn main() -> ExitCode {
    // `cargo rstyle ...` invokes `cargo-rstyle rstyle ...`; drop that echo.
    let args = std::env::args()
        .enumerate()
        .filter(|(i, a)| !(*i == 1 && a == "rstyle"));
    let cli = Cli::parse_from(args.map(|(_, a)| a));
    if cli.skill {
        print!("{SKILL}");
        return ExitCode::SUCCESS;
    }
    if let Some(Command::Patterns(args)) = &cli.command {
        return match patterns(&cli, args) {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(2)
            }
        };
    }
    let result = analyze(&cli).and_then(|report| Ok((render(&report, cli.format)?, report)));
    match result {
        Ok((output, report)) => {
            println!("{output}");
            ExitCode::from(u8::from(!report.diagnostics.is_empty()))
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
