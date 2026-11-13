//! `rstyle go`, `rstyle cxx`, `rstyle py` and `rstyle rust`: find the frontend, read what it produces, run the
//! selected analyses once in core, and save the run in one piece.

mod frontend;
mod pipe;
mod risk;
mod store;

use crate::cpg::analyze::{self, FrontendResult, Unified};
use crate::cpg::{self, input};
use crate::{Cli, Format, MineArgs, PatternDisplayArgs, compiler, ignore, workspace};
use clap::Args;
use frontend::{AnalysisPaths, Frontend, Saving};
pub(crate) use risk::{RiskArgs, risk};
use rstyle_core::config::{Config, Excludes};
use rstyle_core::cpg::OnRecords;
use rstyle_core::cpg::records::{Demand, Record};
use rstyle_core::cpg::transport::{self, DOCUMENTS_MANIFEST, Encoding};
use rstyle_core::exclusions;
use rstyle_core::filter::{Analysis, Filter};
use rstyle_core::unified::Early;
use rstyle_core::{patterns_report, unified};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use store::{Destination, Staged};

const RUN_SCHEMA: &str = "rstyle.run/1";
const RUN_FILE: &str = "run.json";
const POLICY_FILE: &str = "rstyle.toml";
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
    /// What to run: check, patterns, bounds, flow, defects, or <frontend>:<analysis>. Comma-separated
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
    /// A path or glob relative to the source directory to leave out, as `exclude` in rstyle.toml
    /// does, for this run. Python's frontend doesn't read it. Can be repeated.
    #[arg(long = "exclude", value_name = "GLOB")]
    excludes: Vec<String>,
    #[command(flatten)]
    display: PatternDisplayArgs,
    #[command(flatten)]
    mine: MineArgs,
}

/// What a frontend that's a program of its own hears beyond what rstyle tells it.
#[derive(Args, Default)]
pub(crate) struct FrontendArgs {
    /// Arguments for the frontend itself, after `--`, passed on as they are: for instance
    /// `rstyle cxx -- --require-database`. The frontend's own usage lists them.
    #[arg(last = true, value_name = "FRONTEND_ARGS", conflicts_with = "from")]
    arguments: Vec<OsString>,
}

#[derive(Args)]
pub(crate) struct GoArgs {
    /// Go source directory [default: the module containing the current directory].
    #[arg(long, conflicts_with = "from")]
    source: Option<PathBuf>,
    #[command(flatten)]
    run: RunArgs,
    #[command(flatten)]
    frontend: FrontendArgs,
}

#[derive(Args)]
pub(crate) struct CxxArgs {
    /// C or C++ source directory [default: the project holding the current directory].
    #[arg(long, conflicts_with = "from")]
    source: Option<PathBuf>,
    /// compile_commands.json, or a directory holding one [default: the one the frontend finds or
    /// builds].
    #[arg(long, conflicts_with = "from")]
    compilation_database: Option<PathBuf>,
    /// Another include directory. Can be repeated.
    #[arg(long = "include", value_name = "DIR", conflicts_with = "from")]
    includes: Vec<PathBuf>,
    /// A preprocessor definition, NAME or NAME=VALUE. Can be repeated.
    #[arg(long = "define", value_name = "NAME[=VALUE]", conflicts_with = "from")]
    defines: Vec<String>,
    #[command(flatten)]
    run: RunArgs,
    #[command(flatten)]
    frontend: FrontendArgs,
}

impl FrontendCommand for CxxArgs {
    const LANGUAGE: Language = CXX;

    fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    fn run(&self) -> &RunArgs {
        &self.run
    }

    /// What the frontend needs to hear about the build, after the arguments every frontend gets.
    fn arguments(&self) -> Vec<OsString> {
        let database = self.compilation_database.iter().flat_map(|path| {
            [
                OsString::from("--compilation-database"),
                path.as_os_str().to_os_string(),
            ]
        });

        let includes = self
            .includes
            .iter()
            .flat_map(|path| [OsString::from("--include"), path.clone().into_os_string()]);
        let defines = self
            .defines
            .iter()
            .flat_map(|name| [OsString::from("--define"), OsString::from(name)]);
        database.chain(includes).chain(defines).collect()
    }

    fn passed_on(&self) -> &[OsString] {
        &self.frontend.arguments
    }
}

#[derive(Args)]
pub(crate) struct PyArgs {
    /// Python source directory [default: the project holding the current directory].
    #[arg(long, conflicts_with = "from")]
    source: Option<PathBuf>,
    /// The interpreter whose packages the project uses, which the frontend runs [default: an
    /// environment in the project, read without running it, or else the active one].
    #[arg(long, conflicts_with = "from")]
    python: Option<PathBuf>,
    #[command(flatten)]
    run: RunArgs,
    #[command(flatten)]
    frontend: FrontendArgs,
}

impl FrontendCommand for PyArgs {
    const LANGUAGE: Language = PY;

    fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    fn run(&self) -> &RunArgs {
        &self.run
    }

    /// What the frontend needs to hear, after the arguments every frontend gets.
    fn arguments(&self) -> Vec<OsString> {
        self.python
            .iter()
            .flat_map(|path| [OsString::from("--python"), path.clone().into_os_string()])
            .collect()
    }

    fn passed_on(&self) -> &[OsString] {
        &self.frontend.arguments
    }
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

    fn destination(&self, project: &Path, language: &'static str) -> Result<Destination, String> {
        match &self.out {
            Some(out) => Ok(Destination::Exact(out.clone())),
            None => store::project_runs(project, language).map(Destination::Runs),
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
    /// What the run left out, from `exclude` and `--exclude`, so a replay leaves it out too. A run
    /// saved before runs recorded it doesn't say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exclude: Option<Vec<String>>,
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
    /// `rstyle.toml`, with `--exclude` added to what it excludes.
    config: Config,
}

/// What a run needs to know of a frontend's language: what its errors call it, the frontend's name,
/// where its source directory is by default, and where its project is from there.
pub(crate) struct Language {
    called: &'static str,
    frontend: &'static str,
    source_root: fn(&Path) -> Option<PathBuf>,
    project_root: fn(&Path) -> Option<PathBuf>,
}

const GO: Language = Language {
    called: "Go",
    frontend: "go",
    source_root: module_root,
    project_root: module_root,
};
// The frontend finds or builds the compilation database: that's C++ knowledge, not core's.
const CXX: Language = Language {
    called: "C++",
    frontend: "cxx",
    source_root: cxx_project_root,
    project_root: |_| None,
};
const PY: Language = Language {
    called: "Python",
    frontend: "py",
    source_root: py_project_root,
    project_root: py_project_root,
};

/// A command for a frontend that's a program of its own: its language, and what it hands the frontend.
pub(crate) trait FrontendCommand {
    const LANGUAGE: Language;
    fn source(&self) -> Option<&Path>;
    fn run(&self) -> &RunArgs;
    /// What the frontend hears of its language, after what every frontend gets.
    fn arguments(&self) -> Vec<OsString>;
    /// What followed `--`, which the frontend hears last, after what the run excludes, unchanged.
    fn passed_on(&self) -> &[OsString];
}

impl FrontendCommand for GoArgs {
    const LANGUAGE: Language = GO;

    fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    fn run(&self) -> &RunArgs {
        &self.run
    }

    fn arguments(&self) -> Vec<OsString> {
        Vec::new()
    }

    fn passed_on(&self) -> &[OsString] {
        &self.frontend.arguments
    }
}

/// A run of the command's frontend over its source, or a saved run read back.
// rstyle-allow(side_effect_density): process boundary: runs the frontend and saves the run
pub(crate) fn frontend<C: FrontendCommand>(cli: &Cli, command: &C) -> Result<(String, u8), String> {
    let language = &C::LANGUAGE;
    let run = command.run();
    reject_rust_flags(cli, language.called)?;
    let filter = run.selection()?;
    filter.names_only_frontend(language.frontend)?;
    if let Some(from) = &run.from {
        return saved(cli, run, (filter, language.frontend), from);
    }

    let source = chosen_source(command.source(), language.source_root)?;
    let project = (language.project_root)(&source).unwrap_or_else(|| source.clone());
    let frontend = Frontend::located(language.frontend)?
        .streaming(command.arguments(), command.passed_on().to_vec());
    from_frontend(cli, run, (filter, source, project), frontend)
}

/// A run of a frontend that's a program of its own, over its documents and its own analyses.
// rstyle-allow(side_effect_density): process boundary: runs the frontend and saves the run
fn from_frontend(
    cli: &Cli,
    args: &RunArgs,
    (filter, source, project): (Filter, PathBuf, PathBuf),
    frontend: Frontend,
) -> Result<(String, u8), String> {
    let language = frontend.language;
    let local = filter.frontend_analyses(language, &frontend.analyses())?;
    let plan = Plan::new((cli, args), (filter, language), (source, project))?;
    let staged = Staged::new(args.destination(&plan.project, language)?)?;
    let frontend = frontend.excluding(&plan.config.exclude.0);
    let early = Early::new(&plan.filter, &plan.config)?;

    let (records, documents) =
        match plan.filter.needs_documents() || frontend.reads_documents(&local) {
            true => frontend_records(&plan, &frontend, &staged, &early)?,
            false => (Vec::new(), 0),
        };
    let mut unified = plan.analyzed(records, Some(early))?;
    if !local.is_empty() {
        let paths = AnalysisPaths {
            source: &plan.source,
            results: &staged.folder(FRONTEND_FOLDER)?,
            documents: &staged.path().join(DOCUMENTS_FOLDER),
        };
        unified.frontend = frontend.run_analyses(&local, &paths);
    }
    let run = plan.save(&unified, staged, documents)?;
    plan.output(&unified, Some(&run))
}

// rstyle-allow(side_effect_density): process boundary: reads the frontend's stream
fn frontend_records(
    plan: &Plan<'_>,
    frontend: &Frontend,
    staged: &Staged,
    early: &Early,
) -> Result<(Vec<Record>, usize), String> {
    let directory = staged.folder(DOCUMENTS_FOLDER)?;
    let saving = Saving {
        directory: &directory,
        encoding: plan.cli.cpg_compression,
    };
    let take = |records: Vec<Record>| early.take(records);
    let streamed = frontend.stream(&plan.source, (plan.filter.demand(), &take), &saving)?;
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
    let root = crate::run::canonical(&metadata.workspace_root)
        .map_err(|error| format!("{}: {error}", metadata.workspace_root.display()))?;
    // The driver compiles every file, and core leaves out what's excluded.
    let plan = Plan::new((cli, args), (filter, "rust"), (root.clone(), root))?;
    let staged = Staged::new(args.destination(&plan.project, "rust")?)?;
    let early = Early::new(&plan.filter, &plan.config)?;

    let (records, documents) = rust_records(&plan, &staged, &early)?;
    let unified = plan.analyzed(records, Some(early))?;
    let run = plan.save(&unified, staged, documents)?;
    plan.output(&unified, Some(&run))
}

/// Reads the capture `cargo rstyle` and `patterns` share, so only what changed compiles, and
/// keeps its documents with the run.
// rstyle-allow(side_effect_density): process boundary: runs the compiler and reads what it wrote
fn rust_records(
    plan: &Plan<'_>,
    staged: &Staged,
    early: &Early,
) -> Result<(Vec<Record>, usize), String> {
    if !plan.filter.needs_documents() {
        return Ok((Vec::new(), 0));
    }

    let units = compiler::captured_documents(plan.cli)?;
    let documents = store::keep_documents(&units, &staged.folder(DOCUMENTS_FOLDER)?)?;
    let seals = units.iter().flat_map(|unit| {
        unit.documents
            .iter()
            .map(|seal| (unit.directory.as_path(), seal))
    });
    let records =
        input::sealed_records(seals, plan.filter.demand(), &|records| early.take(records))?;
    Ok((records, documents))
}

/// Reads a saved run back, with no frontend unless the filter names one of its analyses.
// rstyle-allow(side_effect_density): IO boundary: reads a saved run
fn saved(
    cli: &Cli,
    args: &RunArgs,
    (filter, language): (Filter, &'static str),
    from: &Path,
) -> Result<(String, u8), String> {
    let record = read_record(from, language)?;
    let replay = replayed_analyses(&filter, language)?;
    let source = match &record {
        Some(record) => record.source.clone(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    let mut plan = Plan::new((cli, args), (filter, language), (source.clone(), source))?;
    let recorded = record.as_ref().and_then(|record| record.exclude.as_deref());
    plan.config.exclude_also(recorded.unwrap_or_default());
    let early = Early::new(&plan.filter, &plan.config)?;

    let take = |records: Vec<Record>| early.take(records);
    let records = saved_records(from, record.as_ref(), (plan.filter.demand(), &take))?;
    let mut unified = plan.analyzed(records, Some(early))?;
    let documents = documents_folder(from);
    match &args.out {
        Some(out) => {
            let staged = Staged::new(Destination::Exact(out.clone()))?;
            if let Some((frontend, names)) = &replay {
                let paths = AnalysisPaths {
                    source: &plan.source,
                    results: &staged.folder(FRONTEND_FOLDER)?,
                    documents: &documents,
                };
                unified.frontend = frontend.run_analyses(names, &paths);
            }
            // The documents stay with the run they came from.
            let run = plan.save(&unified, staged, 0)?;
            plan.output(&unified, Some(&run))
        }
        None => {
            unified.frontend = unsaved_analyses(replay.as_ref(), &plan.source, &documents)?;
            plan.output(&unified, None)
        }
    }
}

/// The analyses a run read back should replay. An analysis that reads the source can't, since a
/// saved run doesn't hold it, and may not even be where it was.
// rstyle-allow(side_effect_density): process boundary: asks the frontend what it offers
fn replayed_analyses(
    filter: &Filter,
    language: &'static str,
) -> Result<Option<(Frontend, Vec<String>)>, String> {
    if filter.named_frontend_analyses().is_empty() {
        return Ok(None);
    }

    let frontend = Frontend::located(language)?;
    let names = filter.frontend_analyses(language, &frontend.analyses())?;
    let reading_source = names
        .iter()
        .find(|name| !frontend.reads_documents(std::slice::from_ref(name)));

    match reading_source {
        Some(name) => Err(format!(
            "--from reads saved documents, and `{language}:{name}` reads the source; run it \
             without --from"
        )),
        None => Ok(Some((frontend, names))),
    }
}

/// Replays analyses for a run nothing saves: what they found is reported, and no file of theirs
/// outlives the command, so none is named.
// rstyle-allow(side_effect_density): process boundary: runs the analyses in a scratch folder
fn unsaved_analyses(
    replay: Option<&(Frontend, Vec<String>)>,
    source: &Path,
    documents: &Path,
) -> Result<Vec<FrontendResult>, String> {
    let Some((frontend, names)) = replay else {
        return Ok(Vec::new());
    };

    let scratch = tempfile::tempdir().map_err(|error| error.to_string())?;
    let paths = AnalysisPaths {
        source,
        results: scratch.path(),
        documents,
    };
    let mut results = frontend.run_analyses(names, &paths);

    for result in &mut results {
        result.file = None;
    }
    Ok(results)
}

fn read_record(from: &Path, language: &str) -> Result<Option<RunRecord>, String> {
    let record = read_any_record(from)?;
    if let Some(other) = record.as_ref().filter(|record| record.language != language) {
        return Err(format!(
            "{} is a {} run, not {language}",
            from.display(),
            other.language
        ));
    }
    Ok(record)
}

// rstyle-allow(side_effect_density): IO boundary: reads the run's record
fn read_any_record(from: &Path) -> Result<Option<RunRecord>, String> {
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
    Ok(Some(record))
}

/// A sealed capture vouches for itself. Unsealed documents are checked against the count the
/// run recorded, so a saved run missing a file can't read as a smaller, cleaner one.
// rstyle-allow(side_effect_density): IO boundary: reads a saved run's documents
fn saved_records(
    from: &Path,
    record: Option<&RunRecord>,
    (demand, take): (Demand, OnRecords<'_>),
) -> Result<Vec<Record>, String> {
    seen_saved_records((from, record), demand, (&|_| {}, take))
}

/// A saved run's records, with each document shown to `see` as it's read and its records handed
/// to `take`.
// rstyle-allow(side_effect_density): IO boundary: reads a saved run's documents
fn seen_saved_records(
    (from, record): (&Path, Option<&RunRecord>),
    demand: Demand,
    (see, take): (input::See<'_>, OnRecords<'_>),
) -> Result<Vec<Record>, String> {
    let directory = documents_folder(from);
    if directory.join(DOCUMENTS_MANIFEST).exists() {
        return input::seen_capture_records(&directory, demand, (see, take));
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
    input::seen_file_records(&paths, demand, (see, take))
}

/// Where a saved run keeps its documents, or the folder itself if it holds them directly.
fn documents_folder(from: &Path) -> PathBuf {
    let folder = from.join(DOCUMENTS_FOLDER);
    match folder.is_dir() {
        true => folder,
        false => from.to_path_buf(),
    }
}

impl<'a> Plan<'a> {
    /// The project's policy with `--exclude` joining what it excludes. A glob core can't read fails
    /// here, before any frontend runs.
    // rstyle-allow(side_effect_density): IO boundary: loads the configuration
    fn new(
        (cli, args): (&'a Cli, &'a RunArgs),
        (filter, language): (Filter, &'static str),
        (source, project): (PathBuf, PathBuf),
    ) -> Result<Self, String> {
        let mut config =
            cpg::load_config(crate::config_path(cli.config.clone(), &project).as_deref())?;
        config.exclude_also(&args.excludes);
        exclusions::Matcher::compile(&config.exclude.0)?;
        Ok(Self {
            cli,
            args,
            filter,
            language,
            source,
            project,
            config,
        })
    }
}

impl Plan<'_> {
    // rstyle-allow(side_effect_density): IO boundary: loads the ledger
    fn analyzed(&self, records: Vec<Record>, early: Option<Early>) -> Result<Unified, String> {
        let options = self.pattern_options()?;
        let analyses =
            unified::analyze_taking(records, (&self.filter, &self.config, &options), early)?;

        let mut unified = Unified::from(analyses);
        unified.exclude = self.config.exclude.0.clone();
        Ok(unified)
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
        unified.write_sarif(staged.path(), &self.source, &self.config)?;
        transport::write_json(&staged.path().join(REPORT_FILE), Encoding::Zstd, unified)?;
        let policy = crate::config_path(self.cli.config.clone(), &self.project);
        keep_policy(policy.as_deref(), staged.path())?;

        let record = RunRecord {
            schema: RUN_SCHEMA.into(),
            language: self.language.into(),
            source: self.source.clone(),
            documents,
            exclude: Some(self.config.exclude.0.clone()),
        };
        transport::write_json(&staged.path().join(RUN_FILE), Encoding::Json, &record)?;
        staged.publish()
    }

    fn output(&self, unified: &Unified, run: Option<&Path>) -> Result<(String, u8), String> {
        let code = unified.exit_code(self.args.fail_on_candidates);
        let output = match self.cli.format {
            Format::Text => with_location(with_exclusions(unified.text(), &unified.exclude), run),
            Format::Json => json_output(unified, run)?,
        };
        Ok((output, code))
    }
}

/// What a report leaves out is said when it's more than the default, so a replay says what its
/// saved run left out too.
fn with_exclusions(text: String, globs: &[String]) -> String {
    match globs == Excludes::default().0 {
        true => text,
        false => format!(
            "{text}\nrstyle: left out what exclude names: {}",
            globs.join(", ")
        ),
    }
}

/// Keeps the policy a run read beside it, so a later comparison goes by what this run used,
/// whatever the checkout holds by then.
// rstyle-allow(side_effect_density): IO boundary: copies the policy into the run
fn keep_policy(policy: Option<&Path>, run: &Path) -> Result<(), String> {
    let Some(policy) = policy else {
        return Ok(());
    };
    std::fs::copy(policy, run.join(POLICY_FILE))
        .map(drop)
        .map_err(|error| format!("{}: {error}", policy.display()))
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

fn reject_rust_flags(cli: &Cli, language: &str) -> Result<(), String> {
    if !cli.packages.is_empty() {
        return Err(format!(
            "--package applies to Rust; for {language}, choose a directory with --source"
        ));
    }
    if cli.all_targets {
        return Err(format!(
            "--all-targets applies to Rust; the {language} frontend always reads tests"
        ));
    }
    if cli.manifest_path.is_some() {
        return Err(format!(
            "--manifest-path applies to Rust; for {language}, use --source"
        ));
    }
    Ok(())
}

// rstyle-allow(side_effect_density): IO boundary: resolves the source directory
fn chosen_source(
    explicit: Option<&Path>,
    project_root: fn(&Path) -> Option<PathBuf>,
) -> Result<PathBuf, String> {
    match explicit {
        Some(source) => {
            crate::run::canonical(source).map_err(|error| format!("{}: {error}", source.display()))
        }
        None => {
            let current = std::env::current_dir().map_err(|error| error.to_string())?;
            Ok(project_root(&current).unwrap_or(current))
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

/// The project holding `start`. C++ has no one marker for that, so it's the nearest directory that
/// knows how the code is built, or failing that the repository.
fn cxx_project_root(start: &Path) -> Option<PathBuf> {
    marked_or_repository(start, holds_database)
}

/// Whether `directory` holds a compilation database, or a `build/` that does.
fn holds_database(directory: &Path) -> bool {
    ["compile_commands.json", "build/compile_commands.json"]
        .iter()
        .any(|name| directory.join(name).is_file())
}

/// The project holding `start`: the nearest directory saying how it's packaged, or else the repository.
fn py_project_root(start: &Path) -> Option<PathBuf> {
    marked_or_repository(start, |directory| {
        ["pyproject.toml", "setup.py", "setup.cfg"]
            .iter()
            .any(|name| directory.join(name).is_file())
    })
}

/// The nearest directory at or above `start` that `marks_a_project` picks out, or else the repository.
fn marked_or_repository(start: &Path, marks_a_project: fn(&Path) -> bool) -> Option<PathBuf> {
    let found = |marker: fn(&Path) -> bool| start.ancestors().find(|directory| marker(directory));
    found(marks_a_project)
        .or_else(|| found(|directory| directory.join(".git").exists()))
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_python_project_is_what_says_how_it_is_packaged_then_its_repository() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(py_project_root(&nested), None);
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert_eq!(py_project_root(&nested), Some(dir.path().to_path_buf()));
        std::fs::write(dir.path().join("a/pyproject.toml"), "").unwrap();
        assert_eq!(py_project_root(&nested), Some(dir.path().join("a")));
    }

    #[test]
    fn a_python_run_passes_its_interpreter_on() {
        let args = PyArgs {
            source: None,
            python: Some("/env/bin/python".into()),
            run: RunArgs {
                check: false,
                patterns: false,
                filter: Vec::new(),
                out: None,
                from: None,
                fail_on_candidates: false,
                no_ignore: false,
                excludes: Vec::new(),
                display: PatternDisplayArgs::default(),
                mine: MineArgs::default(),
            },
            frontend: FrontendArgs {
                arguments: vec!["--strict".into()],
            },
        };
        let expected = ["--python", "/env/bin/python"];
        assert_eq!(args.arguments(), expected.map(OsString::from));
        assert_eq!(args.passed_on(), [OsString::from("--strict")]);
    }

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

    #[test]
    fn a_cxx_project_is_what_knows_its_build_then_its_repository() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a/b");
        std::fs::create_dir_all(nested.join(".git")).unwrap();
        std::fs::remove_dir(nested.join(".git")).unwrap();
        assert_eq!(cxx_project_root(&nested), None);
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert_eq!(cxx_project_root(&nested), Some(dir.path().to_path_buf()));
        std::fs::create_dir_all(dir.path().join("a/build")).unwrap();
        std::fs::write(dir.path().join("a/build/compile_commands.json"), "[]").unwrap();
        assert_eq!(cxx_project_root(&nested), Some(dir.path().join("a")));
        std::fs::write(nested.join("compile_commands.json"), "[]").unwrap();
        assert_eq!(cxx_project_root(&nested), Some(nested.clone()));
    }

    #[test]
    fn a_cxx_run_passes_its_build_context_on() {
        let args = CxxArgs {
            source: None,
            compilation_database: Some("build".into()),
            includes: vec!["a".into(), "b".into()],
            defines: vec!["X=1".into()],
            run: RunArgs {
                check: false,
                patterns: false,
                filter: Vec::new(),
                out: None,
                from: None,
                fail_on_candidates: false,
                no_ignore: false,
                excludes: Vec::new(),
                display: PatternDisplayArgs::default(),
                mine: MineArgs::default(),
            },
            frontend: FrontendArgs {
                arguments: vec!["--require-database".into()],
            },
        };
        let expected = [
            "--compilation-database",
            "build",
            "--include",
            "a",
            "--include",
            "b",
            "--define",
            "X=1",
        ];
        assert_eq!(args.arguments(), expected.map(OsString::from));
        assert_eq!(
            args.passed_on(),
            [OsString::from("--require-database")],
            "what comes after `--` is kept apart, to come last, as it was"
        );
    }
}
