//! `cargo rstyle cpg check`: the style rules over `compat.cpg` documents from any
//! language frontend, without compiling anything.

use rstyle_core::config::Config;
use rstyle_core::report;
use std::path::{Path, PathBuf};

use crate::Format;

/// Style report of one bundled document or a split run, and its exit code.
pub fn check(
    documents: &[PathBuf],
    config: Option<&Path>,
    format: Format,
) -> Result<(String, u8), String> {
    let values = documents
        .iter()
        .map(|path| read(path))
        .collect::<Result<Vec<_>, _>>()?;
    let facts = rstyle_core::cpg::run_facts(values)?;

    let report = rstyle_core::evaluate(facts, &load_config(config)?)?;
    let output = match format {
        Format::Text => report::text(&report),
        Format::Json => report::json(&report).map_err(|error| error.to_string())?,
    };
    Ok((output, report.exit_code()))
}

// rstyle-allow(side_effect_density): IO boundary: reads one input document
fn read(path: &Path) -> Result<serde_json::Value, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
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
    pub all_targets: bool,
    pub manifest: Option<&'a Path>,
}

/// Writes one `compat.cpg` document per compiled workspace target, beside the
/// facts the rustc frontend derives itself, by checking the workspace afresh.
// rstyle-allow(side_effect_density): process boundary: runs cargo with the driver
pub fn emit(selection: &Selection<'_>, out: &Path) -> Result<(String, u8), String> {
    let metadata: crate::workspace::Metadata = crate::workspace::load_with_toolchain(
        selection.manifest,
        Some(&crate::run::pinned_toolchain()),
    )?;
    std::fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    let out = out.canonicalize().map_err(|error| error.to_string())?;
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
        .env("RSTYLE_CPG_ROOT", &metadata.workspace_root);
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

/// Bundled documents in `dir`, each with its reference facts.
// rstyle-allow(side_effect_density): IO boundary: lists the output directory
fn documents(dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let mut pairs = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        let Some(stem) = path
            .file_name()
            .and_then(|name| name.to_str()?.strip_suffix(".cpg.json"))
        else {
            continue;
        };
        let facts = dir.join(format!("{stem}.facts.json"));
        pairs.push((path, facts));
    }
    pairs.sort();

    Ok(pairs)
}

// rstyle-allow(side_effect_density): IO boundary: removes documents of an earlier run
fn remove_documents(dir: &Path) -> Result<(), String> {
    for (document, facts) in documents(dir)? {
        for path in [document, facts] {
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|error| format!("{}: {error}", path.display()))?;
            }
        }
    }
    Ok(())
}

/// Checks each document's derived rule facts against the frontend's own facts,
/// field by field and by the rule diagnostics each set produces.
pub fn compare(dir: &Path, config: Option<&Path>, format: Format) -> Result<(String, u8), String> {
    let config = load_config(config)?;
    let mut comparison = rstyle_core::cpg::compare::Comparison::default();
    let mut diagnostics = Vec::new();
    let pairs = documents(dir)?;
    if pairs.is_empty() {
        return Err(format!("{} has no compat.cpg documents", dir.display()));
    }

    for (document, facts) in &pairs {
        let derived = rstyle_core::cpg::run_facts(vec![read(document)?])
            .map_err(|error| format!("{}: {error}", document.display()))?;
        let reference: Vec<rstyle_core::facts::FnFacts> = serde_json::from_value(read(facts)?)
            .map_err(|error| format!("{}: {error}", facts.display()))?;
        comparison.extend(rstyle_core::cpg::compare::compare(&reference, &derived));
        diagnostics.extend(diagnostic_differences(&reference, &derived, &config)?);
    }
    let failed = !comparison.differences.is_empty() || !diagnostics.is_empty();
    let output = match format {
        Format::Text => comparison_text(&comparison, &diagnostics, pairs.len()),
        Format::Json => serde_json::to_string_pretty(&serde_json::json!({
            "documents": pairs.len(), "functions": comparison.functions,
            "not_compared": rstyle_core::cpg::compare::NOT_DERIVED, "by_field": comparison.by_field(),
            "differences": comparison.differences, "diagnostic_differences": diagnostics,
        }))
        .map_err(|error| error.to_string())?,
    };
    Ok((output, u8::from(failed)))
}

/// Diagnostics one side reports and the other does not.
fn diagnostic_differences(
    reference: &[rstyle_core::facts::FnFacts],
    derived: &[rstyle_core::facts::FnFacts],
    config: &Config,
) -> Result<Vec<String>, String> {
    let lines = |facts: &[rstyle_core::facts::FnFacts]| -> Result<Vec<String>, String> {
        let report = rstyle_core::evaluate(facts.to_vec(), config)?;
        Ok(report
            .diagnostics
            .iter()
            .map(report::diagnostic_line)
            .collect())
    };
    let (expected, actual) = (lines(reference)?, lines(derived)?);
    let only = |side: &str, from: &[String], other: &[String]| -> Vec<String> {
        from.iter()
            .filter(|line| !other.contains(line))
            .map(|line| format!("{side}: {line}"))
            .collect()
    };
    let mut differences = only("rustc only", &expected, &actual);
    differences.extend(only("compat.cpg only", &actual, &expected));

    Ok(differences)
}

fn comparison_text(
    comparison: &rstyle_core::cpg::compare::Comparison,
    diagnostics: &[String],
    documents: usize,
) -> String {
    let mut lines = vec![format!(
        "{documents} documents, {} functions, {} differing facts, {} differing diagnostics; not derived yet: {}",
        comparison.functions,
        comparison.differences.len(),
        diagnostics.len(),
        rstyle_core::cpg::compare::NOT_DERIVED.join(", ")
    )];
    lines.extend(
        comparison
            .by_field()
            .into_iter()
            .map(|(field, count)| format!("  {field}: {count} functions")),
    );
    for difference in comparison.differences.iter().take(20) {
        lines.push(format!(
            "{} {}:\n    rustc:     {}\n    compat.cpg: {}",
            difference.function,
            difference.field,
            difference.reference.replace('\n', " | "),
            difference.derived.replace('\n', " | ")
        ));
    }
    lines.extend(diagnostics.iter().cloned());

    lines.join("\n")
}
