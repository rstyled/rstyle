//! A language frontend outside this binary: finding it, installing it, and asking it for
//! documents and for its own analyses.
use super::pipe;
use crate::cpg::analyze::FrontendResult;
use crate::cpg::input::{self, STREAM_BUFFER, Store, Stream};
use rstyle_core::config::Excludes;
use rstyle_core::cpg::records::{Demand, Record};
use rstyle_core::cpg::transport::{self, DocumentManifest, Encoding, SequenceReader};
use rstyle_core::filter::valid_name;
use rstyle_core::sarif;
use serde::Deserialize;
use std::ffi::{OsStr, OsString};
use std::io::{BufReader, PipeWriter};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Mutex;

const SCHEMA: &str = "rstyle.frontend-description/1";
const GO_PACKAGE: &str = "github.com/rstyled/rstyle-go/cmd/rstyle-go@latest";
const CXX_REPOSITORY: &str = "https://github.com/rstyled/rstyle-cxx";
const PY_REPOSITORY: &str = "https://github.com/rstyled/rstyle-py";

pub(super) struct Frontend {
    pub(super) language: &'static str,
    program: PathBuf,
    analyses: Vec<Described>,
    /// What `cpg` takes beyond the arguments every frontend gets.
    options: Vec<String>,
    /// What this run's stream needs beyond the arguments every frontend gets.
    stream_arguments: Vec<OsString>,
}

/// What a frontend says of itself, so a stale one fails with a reason instead of partway in.
#[derive(Deserialize)]
struct Description {
    schema: String,
    language: String,
    #[serde(default)]
    analyses: Vec<Described>,
    /// Options of `cpg` that not every frontend takes, such as `exclude`. One it doesn't list
    /// isn't passed.
    #[serde(default)]
    options: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Described {
    name: String,
    #[serde(default)]
    input: Input,
}

/// What an analysis works from. A frontend that says nothing of it reads its source.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Input {
    #[default]
    Source,
    /// The run's documents, on a pipe it inherits, so it needs no second read of the source.
    Documents,
}

/// An analysis that finished: its own file, and its runs as the final report holds them.
struct FinishedAnalysis {
    file: PathBuf,
    runs: Vec<serde_json::Value>,
}

/// Where a run's frontend analyses read from and write to.
pub(super) struct AnalysisPaths<'a> {
    pub source: &'a Path,
    /// Where each analysis's results are published.
    pub results: &'a Path,
    /// The run's documents, for an analysis that reads them.
    pub documents: &'a Path,
}

/// A frontend's complete stream, with what was received.
pub(super) struct Streamed {
    pub records: Vec<Record>,
    pub documents: usize,
}

/// Where a stream's documents are kept, and how.
pub(super) struct Saving<'a> {
    pub directory: &'a Path,
    pub encoding: Encoding,
}

impl Frontend {
    /// The frontend for `language`. Go's is `$RSTYLE_GO`, or `rstyle-go` where Go installs it or on
    /// `PATH`, or else installed. C++'s is `$RSTYLE_CXX`, or `rstyle-cxx` on `PATH`: it's built
    /// against a specific LLVM, so there's no one command that installs it, and a missing one says
    /// how to build it. Python's is `$RSTYLE_PY`, or `rstyle-py` on `PATH`: it pins its analyzer and
    /// needs an environment of its own, so a missing one says how to install it.
    // rstyle-allow(side_effect_density): process boundary: finds the frontend and asks what it offers
    pub(super) fn located(language: &'static str) -> Result<Self, String> {
        let program = match language {
            "cxx" => locate_cxx()?,
            "py" => locate_py()?,
            _ => locate_go()?,
        };
        let description = describe(&program, language)?;
        Ok(Self {
            language,
            program,
            analyses: description.analyses,
            options: description.options,
            stream_arguments: Vec::new(),
        })
    }

    /// The same frontend, with what this run's stream needs beyond what every frontend gets.
    pub(super) fn streaming(self, stream_arguments: Vec<OsString>) -> Self {
        Self {
            stream_arguments,
            ..self
        }
    }

    /// The same frontend, told what the run excludes if it takes `--exclude`. One that doesn't
    /// still reads it, and core leaves it out of the report. That's only worth saying when the
    /// run excludes more than the default.
    // rstyle-allow(side_effect_density): IO boundary: says what an older frontend still reads
    pub(super) fn excluding(mut self, globs: &[String]) -> Self {
        if self.options.iter().any(|option| option == "exclude") {
            let arguments = globs
                .iter()
                .flat_map(|glob| [OsString::from("--exclude"), OsString::from(glob)]);
            self.stream_arguments.extend(arguments);
        } else if globs != Excludes::default().0 {
            eprintln!(
                "rstyle: {} doesn't take --exclude, so it still reads what's excluded, and only the \
                 report leaves it out; {}",
                self.name(),
                upgrade(self.language)
            );
        }
        self
    }

    fn name(&self) -> std::path::Display<'_> {
        self.program.display()
    }

    /// The analyses on offer.
    pub(super) fn analyses(&self) -> Vec<String> {
        self.analyses.iter().map(|it| it.name.clone()).collect()
    }

    /// Whether any of these analyses works from the run's documents.
    pub(super) fn reads_documents(&self, names: &[String]) -> bool {
        names
            .iter()
            .any(|name| self.input(name) == Input::Documents)
    }

    fn input(&self, name: &str) -> Input {
        self.analyses
            .iter()
            .find(|analysis| analysis.name == name)
            .map_or(Input::Source, |analysis| analysis.input)
    }

    /// Reads a run from the frontend over its own pipe, keeping the documents as they arrive.
    /// The run counts only if the stream is complete and the frontend exited cleanly.
    // rstyle-allow(side_effect_density): process boundary: runs the frontend and reads its pipe
    pub(super) fn stream(
        &self,
        source: &Path,
        demand: Demand,
        saving: &Saving<'_>,
    ) -> Result<Streamed, String> {
        let (reader, writer) =
            std::io::pipe().map_err(|error| format!("Cannot create a pipe: {error}"))?;
        let child = self.spawn_stream(source, &writer)?;
        // The frontend must hold the only write end, or EOF would never arrive.
        drop(writer);

        let child = Mutex::new(child);
        let stop = || stop_child(&child);
        let mut sequence = SequenceReader::new(BufReader::with_capacity(STREAM_BUFFER, reader));
        let stream = Stream {
            demand,
            on_failure: &stop,
            store: Some(Store {
                directory: saving.directory,
                encoding: saving.encoding,
            }),
        };
        let records = input::stream_records(&mut sequence, &stream);
        let documents = sequence.progress().received_documents;
        let status = wait(child)?;
        self.outcome(records, status)
            .map(|records| Streamed { records, documents })
    }

    fn spawn_stream(&self, source: &Path, writer: &PipeWriter) -> Result<Child, String> {
        let mut command = self.command(&["cpg".as_ref(), "--source".as_ref(), source.as_os_str()]);
        let number = pipe::inherit(&mut command, writer)?;
        command
            .args(["--output-fd", &number])
            .args(&self.stream_arguments)
            .spawn()
            .map_err(|error| format!("Cannot run {}: {error}", self.name()))
    }

    fn outcome(
        &self,
        records: Result<Vec<Record>, String>,
        status: ExitStatus,
    ) -> Result<Vec<Record>, String> {
        match (records, status.success()) {
            (Ok(records), true) => Ok(records),
            (Ok(_), false) => Err(format!(
                "{} sent a complete stream, then failed with {status}; the run is incomplete",
                self.name()
            )),
            // An exit code means it stopped on its own; a stop from this side has none on Unix.
            (Err(error), false) if status.code().is_some() => {
                Err(format!("{error} ({} failed with {status})", self.name()))
            }
            (Err(error), _) => Err(error),
        }
    }

    /// Runs each analysis in turn. One failing doesn't discard the others or the run.
    // rstyle-allow(side_effect_density): process boundary: runs the frontend's analyses
    pub(super) fn run_analyses(
        &self,
        names: &[String],
        paths: &AnalysisPaths<'_>,
    ) -> Vec<FrontendResult> {
        names
            .iter()
            .map(|name| {
                let analysis = format!("{}:{name}", self.language);
                match self.run_analysis(name, paths) {
                    Ok(finished) => FrontendResult {
                        analysis,
                        status: "complete",
                        file: Some(finished.file),
                        runs: finished.runs,
                        error: None,
                    },
                    Err(error) => FrontendResult {
                        analysis,
                        status: "failed",
                        file: None,
                        runs: Vec::new(),
                        error: Some(error),
                    },
                }
            })
            .collect()
    }

    /// A file is published only if the analysis succeeded and wrote valid SARIF. The path is
    /// relative to the run, so reports stay comparable between runs.
    // rstyle-allow(side_effect_density): process boundary: runs one analysis and publishes its file
    fn run_analysis(
        &self,
        name: &str,
        paths: &AnalysisPaths<'_>,
    ) -> Result<FinishedAnalysis, String> {
        let AnalysisPaths {
            source,
            results,
            documents,
        } = *paths;
        let staged = results.join(format!(".{name}.staged"));
        let mut command = self.command(&[
            "analyze".as_ref(),
            name.as_ref(),
            "--source".as_ref(),
            source.as_os_str(),
            "--out".as_ref(),
            staged.as_os_str(),
        ]);
        let status = match self.input(name) {
            Input::Source => command
                .status()
                .map_err(|error| format!("Cannot run {}: {error}", self.name()))?,
            Input::Documents => self.run_reading(command, documents)?,
        };

        let result = published(status, &staged, &results.join(format!("{name}.sarif")));
        let _ = std::fs::remove_file(&staged);
        let category = format!("{}/{name}/", self.language);
        Ok(FinishedAnalysis {
            runs: sarif::labeled_runs(&result?, &category, source)?,
            file: Path::new("frontend").join(format!("{name}.sarif")),
        })
    }

    /// Runs an analysis with the run's documents on a pipe it inherits. It must read them to
    /// the end: only the last record says it has them all, so one that exits first can't have
    /// analyzed the run.
    // rstyle-allow(side_effect_density): process boundary: runs an analysis and feeds it
    fn run_reading(&self, mut command: Command, documents: &Path) -> Result<ExitStatus, String> {
        let manifest = DocumentManifest::read(&transport::read_manifest_bytes(documents)?)?;
        let (reader, mut writer) =
            std::io::pipe().map_err(|error| format!("Cannot create a pipe: {error}"))?;
        let number = pipe::inherit(&mut command, &reader)?;
        let mut child = command
            .args(["--input-fd", &number])
            .spawn()
            .map_err(|error| format!("Cannot run {}: {error}", self.name()))?;
        // The child must hold the only read end, so writing fails if it stops reading.
        drop(reader);

        let (status, sent) = std::thread::scope(|scope| {
            let sending = scope
                .spawn(move || transport::send_sealed(documents, &manifest.documents, &mut writer));
            let status = child.wait();
            (status, sending.join().expect("no panic while sending"))
        });
        let status = status.map_err(|error| format!("Cannot wait for the analysis: {error}"))?;
        match sent {
            Err(error) if status.success() => {
                Err(format!("it exited before reading its documents: {error}"))
            }
            _ => Ok(status),
        }
    }

    /// A frontend's stdout goes to our stderr, so it can't mix with a report on our stdout.
    fn command(&self, args: &[&OsStr]) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::from(std::io::stderr()));
        command
    }
}

fn stop_child(child: &Mutex<Child>) {
    if let Ok(mut child) = child.lock() {
        let _ = child.kill();
    }
}

fn wait(child: Mutex<Child>) -> Result<ExitStatus, String> {
    child
        .into_inner()
        .map_err(|error| error.to_string())?
        .wait()
        .map_err(|error| format!("Cannot wait for the frontend: {error}"))
}

// rstyle-allow(side_effect_density): IO boundary: reads a frontend's file and publishes it
fn published(status: ExitStatus, staged: &Path, file: &Path) -> Result<serde_json::Value, String> {
    if !status.success() {
        return Err(format!("the analysis failed with {status}"));
    }

    let bytes = std::fs::read(staged).map_err(|error| format!("no results file: {error}"))?;
    let log = serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON: {error}"))?;
    sarif::validate(&log)?;
    std::fs::rename(staged, file).map_err(|error| format!("{}: {error}", file.display()))?;
    Ok(log)
}

/// What a frontend this rstyle reads as out of date should do.
fn upgrade(language: &str) -> String {
    match language {
        "cxx" => format!("rebuild it from {CXX_REPOSITORY}"),
        "py" => format!("reinstall it from {PY_REPOSITORY}"),
        _ => format!("update it with `go install {GO_PACKAGE}`"),
    }
}

/// What a frontend offers: its analyses, and the options its `cpg` takes.
#[derive(Debug)]
struct Offered {
    analyses: Vec<Described>,
    options: Vec<String>,
}

// rstyle-allow(side_effect_density): process boundary: asks the frontend what it offers
fn describe(program: &Path, language: &str) -> Result<Offered, String> {
    let output = Command::new(program)
        .arg("describe")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("Cannot run {}: {error}", program.display()))?;
    if !output.status.success() {
        return Err(format!(
            "`{} describe` failed with {}, so it may be too old; {}:\n{}",
            program.display(),
            output.status,
            upgrade(language),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let description: Description = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("`{} describe`: {error}", program.display()))?;
    described(description, language)
        .map_err(|error| format!("`{} describe`: {error}", program.display()))
}

fn described(description: Description, language: &str) -> Result<Offered, String> {
    if description.schema != SCHEMA {
        return Err(format!(
            "speaks {}, and this rstyle needs {SCHEMA}",
            description.schema
        ));
    }
    if description.language != language {
        return Err(format!(
            "is a {} frontend, not {language}",
            description.language
        ));
    }
    let analyses = description
        .analyses
        .into_iter()
        .map(|analysis| match valid_name(&analysis.name) {
            true => Ok(analysis),
            false => Err(format!("names an invalid analysis `{}`", analysis.name)),
        })
        .collect::<Result<_, _>>()?;
    Ok(Offered {
        analyses,
        options: description.options,
    })
}

// rstyle-allow(side_effect_density): process boundary: finds the frontend, or installs it
fn locate_go() -> Result<PathBuf, String> {
    if let Some(path) = overridden("RSTYLE_GO")? {
        return Ok(path);
    }
    if let Some(found) = installed_go() {
        return Ok(found);
    }
    install_go()?;
    installed_go().ok_or_else(|| {
        format!("`go install {GO_PACKAGE}` succeeded, but rstyle-go isn't in Go's bin directory or on PATH")
    })
}

// rstyle-allow(side_effect_density): IO boundary: reads the override and checks what it names
fn overridden(variable: &str) -> Result<Option<PathBuf>, String> {
    let Some(path) = std::env::var_os(variable) else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    match path.is_file() {
        true => Ok(Some(path)),
        false => Err(format!(
            "{variable} names {}, which isn't a file",
            path.display()
        )),
    }
}

fn locate_cxx() -> Result<PathBuf, String> {
    located_without_installing(
        "RSTYLE_CXX",
        "rstyle-cxx",
        format!(
            "rstyle-cxx isn't on PATH. It's built against a specific LLVM, so rstyle can't install \
             it: build it from {CXX_REPOSITORY}, then put it on PATH or point RSTYLE_CXX at it"
        ),
    )
}

fn locate_py() -> Result<PathBuf, String> {
    located_without_installing(
        "RSTYLE_PY",
        "rstyle-py",
        format!(
            "rstyle-py isn't on PATH: install it with `uv tool install git+{PY_REPOSITORY}`, or point \
             RSTYLE_PY at it"
        ),
    )
}

/// What `variable` names, or else `program` on `PATH`, for a frontend rstyle can't install itself.
// rstyle-allow(side_effect_density): IO boundary: finds the frontend
fn located_without_installing(
    variable: &str,
    program: &str,
    missing: String,
) -> Result<PathBuf, String> {
    if let Some(path) = overridden(variable)? {
        return Ok(path);
    }
    let name = format!("{program}{}", std::env::consts::EXE_SUFFIX);
    on_path(&name).ok_or(missing)
}

fn installed_go() -> Option<PathBuf> {
    let name = format!("rstyle-go{}", std::env::consts::EXE_SUFFIX);
    go_bin_directories()
        .into_iter()
        .map(|directory| directory.join(&name))
        .find(|path| path.is_file())
        .or_else(|| on_path(&name))
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

/// Where `go install` puts binaries, by asking Go, or its documented default without it.
fn go_bin_directories() -> Vec<PathBuf> {
    match go_environment() {
        Some(directories) => directories,
        None => home_directory()
            .map(|home| home.join("go/bin"))
            .into_iter()
            .collect(),
    }
}

fn go_environment() -> Option<Vec<PathBuf>> {
    let output = Command::new("go")
        .args(["env", "GOBIN", "GOPATH"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines();
    let bin = lines
        .next()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from);
    let paths = lines.next().unwrap_or_default();
    let from_path = std::env::split_paths(OsStr::new(paths))
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.join("bin"));
    Some(bin.into_iter().chain(from_path).collect())
}

fn home_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

// rstyle-allow(side_effect_density): process boundary: runs `go install`
fn install_go() -> Result<(), String> {
    eprintln!("rstyle: rstyle-go not found; running `go install {GO_PACKAGE}`");
    let status = Command::new("go")
        .args(["install", GO_PACKAGE])
        .stdin(Stdio::null())
        .stdout(Stdio::from(std::io::stderr()))
        .status()
        .map_err(|error| {
            format!("rstyle-go isn't installed, and installing it needs Go on PATH: {error}")
        })?;
    match status.success() {
        true => Ok(()),
        false => Err(format!(
            "`go install {GO_PACKAGE}` failed with {status}. While rstyle-go is private, set \
             GOPRIVATE=github.com/rstyled, or point RSTYLE_GO at a local build"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn description(schema: &str, language: &str, analyses: &[&str]) -> Description {
        Description {
            schema: schema.into(),
            language: language.into(),
            analyses: analyses
                .iter()
                .map(|name| Described {
                    name: (*name).into(),
                    input: Input::Source,
                })
                .collect(),
            options: Vec::new(),
        }
    }

    #[test]
    fn a_description_lists_the_analyses_on_offer() {
        let analyses = described(description(SCHEMA, "go", &["calls"]), "go")
            .unwrap()
            .analyses;
        assert_eq!(analyses.len(), 1);
        assert_eq!(analyses[0].name, "calls");
    }

    #[test]
    fn a_stale_or_wrong_frontend_says_why() {
        let stale = described(
            description("rstyle.frontend-description/0", "go", &[]),
            "go",
        );
        assert!(stale.unwrap_err().contains("this rstyle needs"));
        let wrong = described(description(SCHEMA, "rust", &[]), "go");
        assert!(wrong.unwrap_err().contains("is a rust frontend"));
    }

    #[test]
    fn an_analysis_reads_its_source_unless_it_says_otherwise() {
        let parsed = |text: &str| serde_json::from_str::<Described>(text).map(|it| it.input);
        assert_eq!(parsed(r#"{"name":"a"}"#).unwrap(), Input::Source);
        assert_eq!(
            parsed(r#"{"name":"a","input":"source"}"#).unwrap(),
            Input::Source
        );
        assert_eq!(
            parsed(r#"{"name":"a","input":"documents"}"#).unwrap(),
            Input::Documents
        );
        assert!(parsed(r#"{"name":"a","input":"both"}"#).is_err());
    }

    #[test]
    fn an_analysis_name_cannot_climb_out_of_its_folder() {
        let error = described(description(SCHEMA, "go", &["../x"]), "go").unwrap_err();
        assert!(error.contains("invalid analysis"), "{error}");
    }

    fn frontend(options: &[&str]) -> Frontend {
        Frontend {
            language: "go",
            program: PathBuf::from("rstyle-go"),
            analyses: Vec::new(),
            options: options.iter().map(|option| (*option).into()).collect(),
            stream_arguments: vec!["--compilation-database".into(), "build".into()],
        }
    }

    #[test]
    fn exclusions_follow_the_other_arguments_only_for_a_frontend_that_takes_them() {
        let globs = ["target/**".to_string(), "old.py".into()];
        let taking = frontend(&["exclude"]).excluding(&globs);
        let expected = [
            "--compilation-database",
            "build",
            "--exclude",
            "target/**",
            "--exclude",
            "old.py",
        ];
        assert_eq!(taking.stream_arguments, expected.map(OsString::from));

        let older = frontend(&[]).excluding(&globs);
        assert_eq!(older.stream_arguments, expected.map(OsString::from)[..2]);
    }

    #[test]
    fn a_description_says_what_its_cpg_takes_and_older_ones_say_nothing() {
        let parsed = |text: &str| serde_json::from_str::<Description>(text).map(|it| it.options);
        let older = r#"{"schema":"rstyle.frontend-description/1","language":"go"}"#;
        assert!(parsed(older).unwrap().is_empty());
        let taking =
            r#"{"schema":"rstyle.frontend-description/1","language":"go","options":["exclude"]}"#;
        assert_eq!(parsed(taking).unwrap(), ["exclude"]);
    }
}
