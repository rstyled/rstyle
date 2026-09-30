//! `cargo rstyle`: deterministic Rust style and side-effect analysis.
//!
//! Exit codes: 0 clean, 1 violations found, 2 tool/config/compile error.

mod collect;
mod run;
mod workspace;

use clap::{Parser, ValueEnum};
use rstyle_core::config::Config;
use rstyle_core::report;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "cargo-rstyle", bin_name = "cargo rstyle", version)]
struct Cli {
    /// Only analyze these packages (default: the whole workspace).
    #[arg(short, long = "package")]
    packages: Vec<String>,
    /// Include tests, examples and benches.
    #[arg(long)]
    all_targets: bool,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Path to rstyle.toml (default: <workspace root>/rstyle.toml when present).
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    manifest_path: Option<PathBuf>,
}

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

fn analyze(cli: &Cli) -> Result<report::Report, String> {
    let meta = workspace::load(cli.manifest_path.as_deref())?;
    if let Some(name) = meta.unknown_packages(&cli.packages).first() {
        return Err(format!("package `{name}` is not in this workspace"));
    }
    let config = load_config(cli.config.as_deref(), &meta.workspace_root)?;
    let out_dir = meta.target_directory.join("rstyle").join("facts");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let target_dir = meta.target_directory.join("rstyle").join("build");
    let driver = run::driver_path()?;
    let paths = run::Paths {
        driver: &driver,
        root: &meta.workspace_root,
        target_dir: &target_dir,
        out_dir: &out_dir,
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
    rstyle_core::evaluate(facts, &config)
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
