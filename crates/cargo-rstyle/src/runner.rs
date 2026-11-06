//! `rstyle go` and `rstyle rust`: find the frontend, read what it produces, run the selected
//! analyses once in core, and save the run in one piece.

mod frontend;
mod pipe;
mod store;

use crate::cpg::analyze::{self, Unified};
use crate::cpg::{self, input};
use crate::{Cli, Format, MineArgs, PatternDisplayArgs, ignore, workspace};
use clap::Args;
use frontend::{Frontend, Saving};
use rstyle_core::cpg::records::{Demand, Record};
use rstyle_core::cpg::transport::{self, DOCUMENTS_MANIFEST, Encoding};
use rstyle_core::filter::{Analysis, Filter};
use rstyle_core::{patterns_report, unified};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use store::{Destination, Staged};

const RUN_SCHEMA: &str = "rstyle.run/1";
const RUN_FILE: &str = "run.json";
const REPORT_FILE: &str = "report.json.zst";
const DOCUMENTS_FOLDER: &str = "documents";
const FRONTEND_FOLDER: &str = "frontend";

#[derive(Args)]
pub(crate) struct RunArgs {
    /// Run the style rules. Shorthand for `--filter check`.
    #[arg(long)]
    check: bool,
    /// Mine for shared abstractions. Shorthand for `--filter patterns`.
    #[arg(long)]
    patterns: bool,
    /// What to run: check, patterns, bounds, flow, or <frontend>:<analysis>. Comma-separated
    /// or repeated [default: everything].
    #[arg(long, value_delimiter = ',')]
    filter: Vec<String>,
    /// Save the run in this new directory instead of the default location.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Analyze a saved run without running the frontend; saves only with --out.
    #[arg(long)]
    from: Option<PathBuf>,
    /// Exit 1 when any candidate is left after the ignore ledger (for CI).
    #[arg(long)]
    fail_on_candidates: bool,
    /// Report every candidate, ignoring the ignore ledger for this run.
    #[arg(long)]
    no_ignore: bool,
    #[command(flatten)]
    display: PatternDisplayArgs,
    #[command(flatten)]
    mine: MineArgs,
}

#[derive(Args)]
pub(crate) struct GoArgs {
    /// Go source directory [default: the module containing the current directory].
    #[arg(long, conflicts_with = "from")]
    source: Option<PathBuf>,
    #[command(flatten)]
    run: RunArgs,
}

impl RunArgs {
    /// The shorthands and `--filter` together; flags that can't be honored fail here.
    fn selection(&self) -> Result<Filter, String> {
        let shorthand = [
            self.check.then_some(Analysis::Check.name()),
            self.patterns.then_some(Analysis::Patterns.name()),
        ];
        let names: Vec<&str> = shorthand
            .into_iter()
            .flatten()
            .chain(self.filter.iter().map(String::as_str))
            .collect();

        let filter = Filter::parse(&names)?;
        analyze::validate(&filter, self.fail_on_candidates)?;
        Ok(filter)
    }

    fn destination(
        &self,
        runs: impl FnOnce() -> Result<PathBuf, String>,
    ) -> Result<Destination, String> {
        match &self.out {
            Some(out) => Ok(Destination::Exact(out.clone())),
            None => runs().map(Destination::Runs),
        }
    }
}

/// What a saved run says of itself, so it can be read back on its own.
#[derive(Serialize, Deserialize)]
struct RunRecord {
    schema: String,
    language: String,
    /// What the documents' relative paths resolve against.
    source: PathBuf,
    documents: usize,
}

/// What one run has settled before analyzing.
struct Plan<'a> {
    cli: &'a Cli,
    args: &'a RunArgs,
    filter: Filter,
    language: &'static str,
    /// What the documents' paths are relative to.
    source: PathBuf,
    /// Where `rstyle.toml` and the ignore ledger are.
    project: PathBuf,
}

// rstyle-allow(side_effect_density): process boundary: runs the frontend and saves the run
pub(crate) fn go(cli: &Cli, args: &GoArgs) -> Result<(String, u8), String> {
    reject_rust_flags(cli)?;
    let filter = args.run.selection()?;
    if let Some(from) = &args.run.from {
        return saved(cli, &args.run, (filter, "go"), from);
    }
    let source = go_source(args.source.as_deref())?;
    let project = module_root(&source).unwrap_or_else(|| source.clone());

    let frontend = Frontend::go()?;
    let local = filter.frontend_analyses("go", &frontend.analyses)?;
    let staged = Staged::new(args.run.destination(|| store::project_runs(&project))?)?;
    let plan = Plan {
        cli,
        args: &args.run,
        filter,
        language: "go",
        source,
        project,
    };

    let (records, documents) = go_records(&plan, &frontend, &staged)?;
    let mut unified = plan.analyzed(records)?;
    if !local.is_empty() {
        let directory = staged.folder(FRONTEND_FOLDER)?;
        unified.frontend = frontend.run_analyses(&local, &plan.source, &directory);
    }
    let run = plan.save(&unified, staged, documents)?;
    plan.output(&unified, Some(&run))
}

// rstyle-allow(side_effect_density): process boundary: reads the frontend's stream
fn go_records(
    plan: &Plan<'_>,
    frontend: &Frontend,
    staged: &Staged,
) -> Result<(Vec<Record>, usize), String> {
    if !plan.filter.needs_documents() {
        return Ok((Vec::new(), 0));
    }

    let directory = staged.folder(DOCUMENTS_FOLDER)?;
    let saving = Saving {
        directory: &directory,
        encoding: plan.cli.cpg_compression,
    };
    let streamed = frontend.stream(&plan.source, plan.filter.demand(), &saving)?;
    Ok((streamed.records, streamed.documents))
}

// rstyle-allow(side_effect_density): process boundary: runs the compiler and saves the run
pub(crate) fn rust(cli: &Cli, args: &RunArgs) -> Result<(String, u8), String> {
    let filter = args.selection()?;
    filter.frontend_analyses("rust", &[])?;
    if let Some(from) = &args.from {
        return saved(cli, args, (filter, "rust"), from);
    }

    let metadata: workspace::Metadata = workspace::load_with_toolchain(
        cli.manifest_path.as_deref(),
        Some(&crate::run::pinned_toolchain()),
    )?;
    let root = metadata
        .workspace_root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", metadata.workspace_root.display()))?;
    // Everything stays under `target/`, so `cargo clean` clears it.
    let rstyle = metadata.target_directory.join("rstyle");
    let staged = Staged::new(args.destination(|| Ok(rstyle.join("runs")))?)?;
    let plan = Plan {
        cli,
        args,
        filter,
        language: "rust",
        source: root.clone(),
        project: root,
    };

    let (records, documents) = rust_records(&plan, &staged, &rstyle.join("build"))?;
    let unified = plan.analyzed(records)?;
    let run = plan.save(&unified, staged, documents)?;
    plan.output(&unified, Some(&run))
}

/// The driver writes one document per compilation. Dependencies stay built between runs in
/// the shared `target`; the workspace's own crates are always compiled afresh.
// rstyle-allow(side_effect_density): process boundary: runs the compiler and reads what it wrote
fn rust_records(
    plan: &Plan<'_>,
    staged: &Staged,
    target: &Path,
) -> Result<(Vec<Record>, usize), String> {
    if !plan.filter.needs_documents() {
        return Ok((Vec::new(), 0));
    }

    let directory = staged.folder(DOCUMENTS_FOLDER)?;
    let selection = cpg::Selection {
        packages: &plan.cli.packages,
        encoding: plan.cli.cpg_compression,
        all_targets: plan.cli.all_targets,
        manifest: plan.cli.manifest_path.as_deref(),
    };
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    cpg::emit_documents(&selection, &directory, target)?;
    let paths = cpg::documents(&directory)?;
    let records = input::file_records(&paths, plan.filter.demand())?;
    Ok((records, paths.len()))
}

/// Reads a saved run back. A frontend analysis needs a frontend, so naming one is an error.
// rstyle-allow(side_effect_density): IO boundary: reads a saved run
fn saved(
    cli: &Cli,
    args: &RunArgs,
    (filter, language): (Filter, &'static str),
    from: &Path,
) -> Result<(String, u8), String> {
    if let Some(named) = filter.named_frontend_analyses().first() {
        return Err(format!(
            "--from reads saved documents, and a saved run has no frontend to run `{named}`"
        ));
    }

    let record = read_record(from, language)?;
    let source = match &record {
        Some(record) => record.source.clone(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    let plan = Plan {
        cli,
        args,
        filter,
        language,
        source: source.clone(),
        project: source,
    };

    let records = saved_records(from, record.as_ref(), plan.filter.demand())?;
    let unified = plan.analyzed(records)?;
    match &args.out {
        Some(out) => {
            let staged = Staged::new(Destination::Exact(out.clone()))?;
            // The documents stay with the run they came from.
            let run = plan.save(&unified, staged, 0)?;
            plan.output(&unified, Some(&run))
        }
        None => plan.output(&unified, None),
    }
}

// rstyle-allow(side_effect_density): IO boundary: reads the run's record
fn read_record(from: &Path, language: &str) -> Result<Option<RunRecord>, String> {
    let path = from.join(RUN_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let record: RunRecord =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    if record.schema != RUN_SCHEMA {
        return Err(format!(
            "{} is {}, not {RUN_SCHEMA}",
            path.display(),
            record.schema
        ));
    }
    if record.language != language {
        return Err(format!(
            "{} is a {} run, not {language}",
            from.display(),
            record.language
        ));
    }
    Ok(Some(record))
}

/// A sealed capture vouches for itself. Unsealed documents are checked against the count the
/// run recorded, so a saved run missing a file can't read as a smaller, cleaner one.
// rstyle-allow(side_effect_density): IO boundary: reads a saved run's documents
fn saved_records(
    from: &Path,
    record: Option<&RunRecord>,
    demand: Demand,
) -> Result<Vec<Record>, String> {
    let folder = from.join(DOCUMENTS_FOLDER);
    let directory = if folder.is_dir() {
        folder
    } else {
        from.to_path_buf()
    };
    if directory.join(DOCUMENTS_MANIFEST).exists() {
        return input::capture_records(&directory, demand);
    }

    let paths = cpg::documents(&directory)?;
    if let Some(record) = record.filter(|record| record.documents != paths.len()) {
        return Err(format!(
            "{} recorded {} documents but holds {}",
            from.display(),
            record.documents,
            paths.len()
        ));
    }
    if paths.is_empty() {
        return Err(format!("{} holds no documents", from.display()));
    }
    input::file_records(&paths, demand)
}

impl Plan<'_> {
    // rstyle-allow(side_effect_density): IO boundary: loads the configuration and ledger
    fn analyzed(&self, records: Vec<Record>) -> Result<Unified, String> {
        let config = cpg::load_config(
            crate::config_path(self.cli.config.clone(), &self.project).as_deref(),
        )?;
        let options = self.pattern_options()?;
        unified::analyze(records, &self.filter, &config, &options).map(Unified::from)
    }

    fn pattern_options(&self) -> Result<patterns_report::Options, String> {
        let wants_ledger = self.filter.core(Analysis::Patterns) && !self.args.no_ignore;
        let ledger = wants_ledger
            .then(|| {
                ignore::load(&ignore::ledger_path(
                    self.cli.ignore_file.as_deref(),
                    &self.project,
                ))
            })
            .transpose()?;
        Ok(patterns_report::Options {
            min_score_milli: self.args.display.min_score_milli(),
            top: self.args.display.top,
            ledger,
            ..crate::mine_options(&self.args.mine)
        })
    }

    /// Writes the report and SARIF, then publishes the whole run. The report is compressed
    /// here and printed plain, so `--format json | jq` keeps working.
    fn save(&self, unified: &Unified, staged: Staged, documents: usize) -> Result<PathBuf, String> {
        unified.write_sarif(staged.path(), &self.source)?;
        transport::write_json(&staged.path().join(REPORT_FILE), Encoding::Zstd, unified)?;

        let record = RunRecord {
            schema: RUN_SCHEMA.into(),
            language: self.language.into(),
            source: self.source.clone(),
            documents,
        };
        transport::write_json(&staged.path().join(RUN_FILE), Encoding::Json, &record)?;
        staged.publish()
    }

    fn output(&self, unified: &Unified, run: Option<&Path>) -> Result<(String, u8), String> {
        let code = unified.exit_code(self.args.fail_on_candidates);
        let output = match self.cli.format {
            Format::Text => with_location(unified.text(), run),
            Format::Json => json_output(unified, run)?,
        };
        Ok((output, code))
    }
}

fn with_location(text: String, run: Option<&Path>) -> String {
    match run {
        Some(run) => format!("{text}\nrstyle: run saved in {}", run.display()),
        None => text,
    }
}

/// A saved report says nothing of where it ran; what's printed also names the run.
#[derive(Serialize)]
struct Printed<'a> {
    #[serde(flatten)]
    report: &'a Unified,
    #[serde(skip_serializing_if = "Option::is_none")]
    run: Option<&'a Path>,
}

fn json_output(report: &Unified, run: Option<&Path>) -> Result<String, String> {
    serde_json::to_string_pretty(&Printed { report, run }).map_err(|error| error.to_string())
}

fn reject_rust_flags(cli: &Cli) -> Result<(), String> {
    if !cli.packages.is_empty() {
        return Err("--package applies to Rust; for Go, choose a directory with --source".into());
    }
    if cli.all_targets {
        return Err("--all-targets applies to Rust; the Go frontend always reads tests".into());
    }
    if cli.manifest_path.is_some() {
        return Err("--manifest-path applies to Rust; for Go, use --source".into());
    }
    Ok(())
}

// rstyle-allow(side_effect_density): IO boundary: resolves the source directory
fn go_source(explicit: Option<&Path>) -> Result<PathBuf, String> {
    match explicit {
        Some(source) => source
            .canonicalize()
            .map_err(|error| format!("{}: {error}", source.display())),
        None => {
            let current = std::env::current_dir().map_err(|error| error.to_string())?;
            Ok(module_root(&current).unwrap_or(current))
        }
    }
}

/// The nearest directory at or above `start` holding a `go.mod`.
fn module_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|directory| directory.join("go.mod").is_file())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_module_root_is_the_nearest_with_a_go_mod() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(module_root(&nested), None);
        std::fs::write(dir.path().join("go.mod"), "module m\n").unwrap();
        assert_eq!(module_root(&nested), Some(dir.path().to_path_buf()));
        std::fs::write(dir.path().join("a/go.mod"), "module m/a\n").unwrap();
        assert_eq!(module_root(&nested), Some(dir.path().join("a")));
    }
}
