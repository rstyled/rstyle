//! `cargo rstyle cpg`: the style rules and the pattern miner over `compat.cpg` documents
//! from any language frontend, without compiling anything.

mod ingest;
mod input;
mod receipt;

use rstyle_core::config::Config;
use rstyle_core::cpg::records::{Demand, Record};
use rstyle_core::{cpg, patterns_report, report};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::{Cli, CpgInputArgs, CpgPatternsArgs, Format};

/// Style report of a run over any number of units, and its exit code.
pub fn check(
    args: &CpgInputArgs,
    config: Option<&Path>,
    format: Format,
) -> Result<(String, u8), String> {
    let received = match input::read(args, Demand::default()) {
        Ok(received) => received,
        Err(error) => return error.output(format),
    };
    let config = load_config(config)?;
    let inputs = cpg::run_inputs(received.records, &config)?;

    let report = rstyle_core::evaluate_with_effects(
        inputs.facts,
        &config,
        rstyle_core::Retention::Report,
        inputs.effects,
    )?
    .report;
    let output = match format {
        Format::Text => receipt::text(report::text(&report), &received.receipt),
        Format::Json => receipt::json(&report, &received.receipt)?,
    };
    Ok((output, report.exit_code()))
}

/// Abstraction candidates of a run, with effects propagated across all of its units
/// and, when asked for, each function's runtime bounds.
pub fn patterns(cli: &Cli, args: &CpgPatternsArgs) -> Result<(String, u8), String> {
    let demand = Demand {
        graphs: true,
        bounds: args.complexity,
    };
    let received = match input::read(&args.input, demand) {
        Ok(received) => received,
        Err(error) => return error.output(cli.format),
    };
    let records = received.records;
    let bounds = args
        .complexity
        .then(|| crate::clones::document_bounds(&records, args.display.top));
    let evaluation = mined(records, args, &load_config(cli.config.as_deref())?)?;
    let code = u8::from(args.fail_on_candidates && !evaluation.candidates.is_empty());

    let output = match cli.format {
        Format::Text => {
            let bounds = bounds.as_ref().map(|report| report.text());
            let mined = patterns_report::text(&evaluation);
            let text = [mined]
                .into_iter()
                .chain(bounds)
                .collect::<Vec<_>>()
                .join("\n");
            receipt::text(text, &received.receipt)
        }
        Format::Json => {
            let report = json!({"evaluation": evaluation, "runtime_bounds": bounds});
            receipt::json(&report, &received.receipt)?
        }
    };
    Ok((output, code))
}

fn mined(
    records: Vec<Record>,
    args: &CpgPatternsArgs,
    config: &Config,
) -> Result<patterns_report::PatternsReport, String> {
    let inputs = cpg::run_inputs(records, config)?;
    let observed = rstyle_core::effect_observations(inputs.facts, config, Some(inputs.effects))?;
    let options = patterns_report::Options {
        min_score_milli: args.display.min_score_milli(),
        top: args.display.top,
        ..crate::mine_options(&args.mine)
    };

    let (mut evaluation, _) =
        patterns_report::run_distinct_owned_and_retain(observed.facts, &options);
    rstyle_core::candidates::annotate_completed_effects(
        &mut evaluation.candidates,
        &observed.effects,
    )?;
    Ok(evaluation)
}

// rstyle-allow(side_effect_density): IO boundary: reads one input document
fn read(path: &Path) -> Result<serde_json::Value, String> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let manifest = directory.join(cpg::transport::DOCUMENTS_MANIFEST);
    if manifest.exists() {
        let bytes = std::fs::read(&manifest).map_err(|e| e.to_string())?;
        let manifest = cpg::transport::DocumentManifest::read(&bytes)?;
        let seal = manifest
            .documents
            .iter()
            .find(|seal| Some(std::ffi::OsStr::new(&seal.file)) == path.file_name())
            .ok_or_else(|| format!("{} is not listed in its document manifest", path.display()))?;
        cpg::transport::read_sealed(directory, seal)
    } else {
        cpg::transport::read_json(path)
    }
}

/// Documents name no Cargo workspace, so only an explicit `--config` applies.
// rstyle-allow(side_effect_density): IO boundary: reads the explicit config file
fn load_config(path: Option<&Path>) -> Result<Config, String> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Config::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// What `emit` compiles: the workspace or selected packages, optionally all targets.
pub struct Selection<'a> {
    pub packages: &'a [String],
    pub encoding: cpg::transport::Encoding,
    pub all_targets: bool,
    pub manifest: Option<&'a Path>,
}

/// Writes the `compat.cpg` documents of every compiled workspace target, by checking the
/// workspace afresh. They hold every function the compiler does, as a capture's do.
// rstyle-allow(side_effect_density): process boundary: runs cargo with the driver
pub fn emit(selection: &Selection<'_>, out: &Path) -> Result<(String, u8), String> {
    let metadata: crate::workspace::Metadata = crate::workspace::load_with_toolchain(
        selection.manifest,
        Some(&crate::run::pinned_toolchain()),
    )?;
    std::fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    let out = out.canonicalize().map_err(|error| error.to_string())?;
    // rustc resolves source paths against its working directory, which the OS
    // reports with symlinks resolved; a root reached through a symlink (e.g.
    // `--manifest-path` under macOS's `/var`) would match no source file.
    let root = metadata
        .workspace_root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", metadata.workspace_root.display()))?;
    remove_documents(&out)?;
    let target = out.join("target");

    let members: Vec<&str> = metadata
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    let mut clean = cargo("clean", selection.manifest);
    clean.arg("--target-dir").arg(&target);
    clean.args(members.iter().flat_map(|name| ["-p", name]));
    status(clean, "cargo clean")?;

    let mut check = cargo("check", selection.manifest);
    check.arg("--target-dir").arg(&target);
    check.args(packages(selection));
    check
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTC_WORKSPACE_WRAPPER", crate::run::driver_path()?)
        .env("RSTYLE_CPG_DIR", &out)
        .env("RSTYLE_CPG_ROOT", &root)
        .env("RSTYLE_CPG_COMPRESSION", selection.encoding.cli_name());
    status(check, "cargo check")?;
    let written = documents(&out)?.len();

    Ok((
        format!(
            "{written} compat.cpg documents written to {}",
            out.display()
        ),
        0,
    ))
}

fn cargo(subcommand: &str, manifest: Option<&Path>) -> std::process::Command {
    let mut command = std::process::Command::new("cargo");
    crate::run::clear_rstyle_environment(&mut command);
    command
        .arg(subcommand)
        .env("RUSTUP_TOOLCHAIN", crate::run::pinned_toolchain())
        .env("RUSTC_WRAPPER", "")
        .env_remove("RUSTC");
    if let Some(manifest) = manifest {
        command.arg("--manifest-path").arg(manifest);
    }

    command
}

fn packages(selection: &Selection<'_>) -> Vec<String> {
    let mut args: Vec<String> = match selection.packages {
        [] => vec!["--workspace".into()],
        names => names
            .iter()
            .flat_map(|name| ["-p".to_string(), name.clone()])
            .collect(),
    };
    if selection.all_targets {
        args.push("--all-targets".into());
    }
    args
}

// rstyle-allow(side_effect_density): process boundary: runs one cargo command
fn status(mut command: std::process::Command, what: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("{what}: {error}"))?;
    match status.success() {
        true => Ok(()),
        false => Err(format!("{what} failed with {status}")),
    }
}

/// The documents an emit wrote to `dir`, in name order.
// rstyle-allow(side_effect_density): IO boundary: lists the output directory
fn documents(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let mut documents = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.to_string_lossy().ends_with(".cpg.json")
            || path.to_string_lossy().ends_with(".cpg.json.zst")
        {
            documents.push(path);
        }
    }
    documents.sort();

    Ok(documents)
}

// rstyle-allow(side_effect_density): IO boundary: removes documents of an earlier run
fn remove_documents(dir: &Path) -> Result<(), String> {
    // Standalone exports have no manifest. Remove a prior sealed capture's
    // manifest before refreshing its documents so readers cannot use stale seals.
    let manifest = dir.join(cpg::transport::DOCUMENTS_MANIFEST);
    match std::fs::remove_file(&manifest) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}: {error}", manifest.display())),
    }
    for path in documents(dir)? {
        std::fs::remove_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

/// Store a frontend's streaming JSON text sequence as a sealed capture.
pub fn ingest(out: &Path, encoding: cpg::transport::Encoding) -> Result<(String, u8), String> {
    ingest::run(&mut std::io::stdin().lock(), out, encoding)
}

/// Verify all documents of a completed capture and return a structured receipt.
pub fn receive(directory: &Path, format: Format) -> Result<(String, u8), String> {
    input::receive(directory, format)
}
