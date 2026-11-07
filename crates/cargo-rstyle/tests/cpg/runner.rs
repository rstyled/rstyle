//! `rstyle go` and `rstyle cxx` against a fake frontend that keeps to the contract, and `rstyle
//! rust` through the real driver. Unix only: the fake frontend is a shell script.
use super::*;
use rstyle_core::cpg::transport::{SequenceReader, read_json};
use std::os::unix::fs::PermissionsExt;

const FRONTEND: &str = r#"#!/bin/sh
set -eu
case "$1" in
describe)
    if [ -n "${FAKE_DESCRIBE_FAILS:-}" ]; then echo "unknown command" >&2; exit 1; fi
    printf '%s\n' "${FAKE_DESCRIPTION:-}"
    ;;
cpg)
    echo "progress that must not reach rstyle's stdout"
    test "$2" = --source && test "$4" = --output-fd
    if [ -n "${FAKE_ARGUMENTS:-}" ]; then printf '%s\n' "$@" > "$FAKE_ARGUMENTS"; fi
    if [ -n "${FAKE_BAD_DOCUMENT:-}" ]; then
        # The next separator finishes the record, so the runner starts on it at once.
        printf '\036{"layout":"bundled"}\n\036' > "/dev/fd/$5"
        # Only a stop from the runner can end this, and it still holds the pipe open.
        exec sleep 300
    fi
    {
        printf '\036'
        cat "$FAKE_DOCUMENT"
        if [ -z "${FAKE_TRUNCATE:-}" ]; then printf '\n\036null\n'; fi
    } > "/dev/fd/$5"
    exit "${FAKE_CPG_EXIT:-0}"
    ;;
analyze)
    test "$3" = --source && test "$5" = --out
    case "$2" in
    reader|quitter|sinking) test "$7" = --input-fd ;;
    *) test "$#" = 6 ;;
    esac
    case "$2" in
    reader) cat "/dev/fd/$8" > "$FAKE_RECEIVED" ;;
    sinking) cat "/dev/fd/$8" > /dev/null; exit 3 ;;
    esac
    case "$2" in
    broken) exit 3 ;;
    invalid) echo '{"version":"1.0"}' > "$6" ;;
    silent) ;;
    *) echo '{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"fake"}},"results":[{"message":{"text":"found"}}]}]}' > "$6" ;;
    esac
    ;;
*) exit 64 ;;
esac
"#;

const NO_ANALYSES: &str =
    r#"{"schema":"rstyle.frontend-description/1","language":"go","analyses":[]}"#;
const ANALYSES: &str = r#"{"schema":"rstyle.frontend-description/1","language":"go","analyses":[{"name":"calls"},{"name":"broken"},{"name":"invalid"},{"name":"silent"}]}"#;
const READERS: &str = r#"{"schema":"rstyle.frontend-description/1","language":"go","analyses":[{"name":"reader","input":"documents"},{"name":"quitter","input":"documents"},{"name":"sinking","input":"documents"},{"name":"calls","input":"source"}]}"#;
const CXX: &str = r#"{"schema":"rstyle.frontend-description/1","language":"cxx","analyses":[]}"#;
const CXX_READERS: &str = r#"{"schema":"rstyle.frontend-description/1","language":"cxx","analyses":[{"name":"reader","input":"documents"},{"name":"calls","input":"source"}]}"#;

/// A Go module and a frontend to run over it.
struct Project {
    directory: TempDir,
}

impl Project {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("go.mod"), "module example.com/m\n").unwrap();
        std::fs::write(
            directory.path().join("document.json"),
            serde_json::to_vec(&example()).unwrap(),
        )
        .unwrap();
        script(&directory.path().join("fake-frontend"), FRONTEND);
        Self { directory }
    }

    fn root(&self) -> PathBuf {
        // As the frontend's process sees it: a temporary directory can sit behind a symlink.
        self.directory.path().canonicalize().unwrap()
    }

    fn command(&self, description: &str, args: &[&str]) -> Command {
        let mut command = Command::new(cargo_bin("rstyle"));
        command
            .args(args)
            .current_dir(self.root())
            .env("RSTYLE_GO", self.root().join("fake-frontend"))
            .env("FAKE_DOCUMENT", self.root().join("document.json"))
            .env("FAKE_DESCRIPTION", description);
        command
    }

    fn go(&self, args: &[&str]) -> Output {
        self.command(NO_ANALYSES, &[&["go"], args].concat())
            .output()
            .unwrap()
    }

    fn cxx(&self, args: &[&str]) -> Command {
        let mut command = self.command(CXX, &[&["cxx"], args].concat());
        command.env("RSTYLE_CXX", self.root().join("fake-frontend"));
        command
    }

    fn run_folders(&self) -> Vec<PathBuf> {
        let runs = self.root().join(".rstyle/runs");
        match std::fs::read_dir(runs) {
            Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn script(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {output:?}"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn saved_run(output: &Output) -> PathBuf {
    PathBuf::from(report(output)["run"].as_str().unwrap())
}

fn without_run(mut report: Value) -> Value {
    report.as_object_mut().unwrap().remove("run");
    report
}

#[test]
fn a_go_run_saves_everything_in_one_timestamped_folder() {
    let project = Project::new();
    let output = project.go(&["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    assert!(unified["check"].is_object());
    assert!(unified["patterns"].is_object());
    assert!(unified["runtime_bounds"].is_object());
    assert!(unified["flow"]["functions"].is_array());
    assert!(unified["defects"]["defects"].is_array());

    let run = saved_run(&output);
    assert_eq!(run.parent().unwrap(), project.root().join(".rstyle/runs"));
    let name = run.file_name().unwrap().to_str().unwrap();
    assert!(
        name.len() == 19 && name.ends_with("Z-go") && name.as_bytes()[8] == b'T',
        "{name}"
    );
    for file in [
        "documents/documents.manifest.json",
        "documents/document-000000.cpg.json.zst",
        "check.sarif",
        "patterns.sarif",
        "run.json",
    ] {
        assert!(run.join(file).is_file(), "{file}");
    }
    assert_eq!(
        std::fs::read_to_string(project.root().join(".rstyle/.gitignore")).unwrap(),
        "*\n"
    );
    assert_eq!(
        read_json::<Value>(&run.join("report.json.zst")).unwrap(),
        without_run(unified)
    );
    let bytes = std::fs::read(run.join("report.json.zst")).unwrap();
    assert_eq!(&bytes[..4], [0x28, 0xb5, 0x2f, 0xfd], "a zstd frame");
    assert_eq!(project.run_folders(), [run], "no staging left behind");
}

#[test]
fn a_saved_report_says_nothing_of_when_or_where_it_ran() {
    let project = Project::new();
    let first = saved_run(&project.go(&["--format", "json"]));
    let second = project.root().join("elsewhere");
    let output = project.go(&["--format", "json", "--out", second.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        std::fs::read(first.join("report.json.zst")).unwrap(),
        std::fs::read(second.join("report.json.zst")).unwrap()
    );
}

#[test]
fn the_shorthands_and_filter_select_together() {
    let project = Project::new();
    let check = project.go(&["--check", "--format", "json"]);
    let unified = report(&check);
    assert!(unified["check"].is_object() && unified.get("patterns").is_none());
    let run = saved_run(&check);
    assert!(run.join("check.sarif").exists() && !run.join("patterns.sarif").exists());

    let unified = report(&project.go(&["--patterns", "--format", "json"]));
    assert!(unified["patterns"].is_object() && unified.get("check").is_none());

    let unified = report(&project.go(&["--check", "--filter", "bounds", "--format", "json"]));
    assert!(unified["check"].is_object() && unified["runtime_bounds"].is_object());
    assert!(unified.get("patterns").is_none() && unified.get("flow").is_none());

    let unified = report(&project.go(&["--filter", "flow", "--format", "json"]));
    assert!(unified["flow"]["functions"].is_array() && unified.get("check").is_none());
}

#[test]
fn text_output_names_where_the_run_is() {
    let project = Project::new();
    let output = project.go(&["--check"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = stdout(&output);
    assert!(text.contains("rstyle: 3 functions, 0 violations"), "{text}");
    assert!(text.contains("rstyle: run saved in"), "{text}");
    assert!(
        stderr(&output).contains("progress that must not reach"),
        "{output:?}"
    );
}

#[test]
fn a_failed_stream_publishes_nothing() {
    let project = Project::new();
    let output = project
        .command(NO_ANALYSES, &["go"])
        .env("FAKE_TRUNCATE", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("ended before its terminal null"),
        "{output:?}"
    );
    assert!(project.run_folders().is_empty());
}

#[test]
fn a_bad_document_stops_a_frontend_that_is_still_running() {
    let project = Project::new();
    let started = std::time::Instant::now();
    let output = project
        .command(NO_ANALYSES, &["go"])
        .env("FAKE_BAD_DOCUMENT", "1")
        .output()
        .unwrap();
    assert!(
        started.elapsed().as_secs() < 60,
        "the frontend was left running"
    );
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        !stderr(&output).contains("terminal null"),
        "the document's error is the one reported: {output:?}"
    );
    assert!(project.run_folders().is_empty());
}

#[test]
fn a_complete_stream_from_a_frontend_that_then_failed_is_incomplete() {
    let project = Project::new();
    let output = project
        .command(NO_ANALYSES, &["go"])
        .env("FAKE_CPG_EXIT", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("the run is incomplete"),
        "{output:?}"
    );
    assert!(project.run_folders().is_empty());
}

#[test]
fn a_saved_run_reads_back_without_the_frontend() {
    let project = Project::new();
    let output = project.go(&["--format", "json"]);
    let run = saved_run(&output);
    let expected = without_run(report(&output));

    let again = Command::new(cargo_bin("rstyle"))
        .args(["go", "--format", "json", "--from", run.to_str().unwrap()])
        .current_dir(project.root())
        .env("RSTYLE_GO", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(report(&again), expected);
    assert_eq!(project.run_folders().len(), 1, "--from alone saves nothing");

    let wrong = project
        .command(NO_ANALYSES, &["rust", "--from", run.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(wrong.status.code(), Some(2));
    assert!(
        stderr(&wrong).contains("is a go run, not rust"),
        "{wrong:?}"
    );

    let named = project.go(&["--filter", "go:calls", "--from", run.to_str().unwrap()]);
    assert_eq!(named.status.code(), Some(2));
    assert!(
        stderr(&named).contains("has no analysis `calls`"),
        "{named:?}"
    );
}

/// A saved run, with the frontend's analyses described as `description`.
fn saved_with(project: &Project, description: &str) -> PathBuf {
    let output = project
        .command(description, &["go", "--check", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    saved_run(&output)
}

#[test]
fn a_saved_run_replays_an_analysis_that_reads_documents() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    let received = project.root().join("received");

    let output = project
        .command(
            READERS,
            &[
                "go",
                "--filter",
                "go:reader",
                "--from",
                run.to_str().unwrap(),
                "--format",
                "json",
            ],
        )
        .env("FAKE_RECEIVED", &received)
        // Only the saved documents can answer.
        .env_remove("FAKE_DOCUMENT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    let frontend = &unified["frontend"][0];
    assert_eq!(frontend["status"], "complete", "{frontend}");
    assert_eq!(frontend["runs"][0]["automationDetails"]["id"], "go/reader/");
    assert!(
        frontend.get("file").is_none(),
        "nothing keeps it: {frontend}"
    );

    let bytes = std::fs::read(&received).unwrap();
    let mut sequence = SequenceReader::new(bytes.as_slice());
    assert_eq!(sequence.next().unwrap().unwrap(), example());
    assert!(sequence.next().is_none());
    sequence.complete().unwrap();
    assert_eq!(project.run_folders().len(), 1, "--from alone saves nothing");
}

#[test]
fn text_from_a_replay_says_what_was_found_and_names_no_file() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    let output = project
        .command(
            READERS,
            &[
                "go",
                "--filter",
                "go:reader",
                "--from",
                run.to_str().unwrap(),
            ],
        )
        .env("FAKE_RECEIVED", project.root().join("received"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = stdout(&output);
    assert!(text.contains("go:reader: 1 result"), "{text}");
    assert!(!text.contains("frontend/"), "{text}");
}

#[test]
fn a_replay_saved_with_out_keeps_the_analysis_file() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    let out = project.root().join("replayed");
    let output = project
        .command(
            READERS,
            &[
                "go",
                "--filter",
                "go:reader",
                "--from",
                run.to_str().unwrap(),
                "--out",
                out.to_str().unwrap(),
                "--format",
                "json",
            ],
        )
        .env("FAKE_RECEIVED", project.root().join("received"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        report(&output)["frontend"][0]["file"],
        "frontend/reader.sarif"
    );
    assert!(out.join("frontend/reader.sarif").is_file());
    assert!(out.join("report.sarif").is_file());
}

#[test]
fn a_saved_run_refuses_an_analysis_that_reads_the_source() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    let output = project
        .command(
            READERS,
            &[
                "go",
                "--filter",
                "go:calls",
                "--from",
                run.to_str().unwrap(),
            ],
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("`go:calls` reads the source"),
        "{output:?}"
    );
}

#[test]
fn a_replay_without_its_frontend_says_so() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    let output = Command::new(cargo_bin("rstyle"))
        .args([
            "go",
            "--filter",
            "go:reader",
            "--from",
            run.to_str().unwrap(),
        ])
        .current_dir(project.root())
        .env("RSTYLE_GO", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("RSTYLE_GO"), "{output:?}");
}

/// A run with no frontend to be found, where looking for Go's would install it.
fn with_nothing_installed(project: &Project, bin: &Path, args: &[&str]) -> Output {
    project
        .command(NO_ANALYSES, args)
        .env_remove("RSTYLE_GO")
        .env_remove("RSTYLE_CXX")
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("HOME", project.root())
        .output()
        .unwrap()
}

#[test]
fn another_frontends_analysis_is_refused_without_looking_for_one() {
    let project = Project::new();
    let bin = fake_go(&project, false);
    let from = project.root().join("empty");
    std::fs::create_dir(&from).unwrap();
    let from = from.to_str().unwrap();

    for (language, analysis, refusal) in [
        (
            "rust",
            "go:reader",
            "`go:reader` belongs to the go frontend, not rust",
        ),
        (
            "go",
            "cxx:reader",
            "`cxx:reader` belongs to the cxx frontend, not go",
        ),
        (
            "cxx",
            "go:reader",
            "`go:reader` belongs to the go frontend, not cxx",
        ),
    ] {
        let run = [language, "--filter", analysis];
        let replay = [language, "--filter", analysis, "--from", from];
        for args in [&run[..], &replay[..]] {
            let output = with_nothing_installed(&project, &bin, args);
            assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
            assert!(stderr(&output).contains(refusal), "{args:?}: {output:?}");
            assert!(
                !project.root().join("gobin/installs").exists(),
                "{args:?} installed a frontend for an analysis that can't run"
            );
        }
    }
}

#[test]
fn a_rust_replay_has_no_frontend_to_ask() {
    let project = Project::new();
    let bin = fake_go(&project, false);
    let from = project.root().join("empty");
    std::fs::create_dir(&from).unwrap();

    let args = [
        "rust",
        "--filter",
        "rust:calls",
        "--from",
        from.to_str().unwrap(),
    ];
    let output = with_nothing_installed(&project, &bin, &args);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("The rust frontend has no analysis `calls`: it offers none"),
        "{output:?}"
    );
    assert!(!project.root().join("gobin/installs").exists());
}

#[test]
fn a_replay_of_another_languages_run_says_so_before_looking_for_a_frontend() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    let output = project
        .cxx(&["--filter", "cxx:reader", "--from", run.to_str().unwrap()])
        .env("RSTYLE_CXX", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("is a go run, not cxx"),
        "{output:?}"
    );
}

#[test]
fn a_replay_of_documents_with_no_seals_fails_its_analysis_and_keeps_the_report() {
    let project = Project::new();
    let run = saved_with(&project, READERS);
    std::fs::remove_file(run.join("documents/documents.manifest.json")).unwrap();
    let output = project
        .command(
            READERS,
            &[
                "go",
                "--filter",
                "check,go:reader",
                "--from",
                run.to_str().unwrap(),
                "--format",
                "json",
            ],
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let unified = report(&output);
    assert!(unified["check"].is_object(), "core could read them");
    let frontend = &unified["frontend"][0];
    assert_eq!(frontend["status"], "failed");
    let error = frontend["error"].as_str().unwrap();
    assert!(error.contains("documents.manifest.json"), "{error}");
}

#[test]
fn a_run_saved_from_a_saved_run_holds_no_documents_of_its_own() {
    let project = Project::new();
    let output = project.go(&["--format", "json"]);
    let run = saved_run(&output);
    let expected = without_run(report(&output));

    let copy = project.root().join("copy");
    let again = project.go(&[
        "--format",
        "json",
        "--from",
        run.to_str().unwrap(),
        "--out",
        copy.to_str().unwrap(),
    ]);
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(without_run(report(&again)), expected);
    assert!(copy.join("report.json.zst").is_file() && !copy.join("documents").exists());

    let empty = project.go(&["--from", copy.to_str().unwrap()]);
    assert_eq!(empty.status.code(), Some(2), "{empty:?}");
    assert!(stderr(&empty).contains("holds no documents"), "{empty:?}");
}

#[test]
fn a_language_keeps_its_newest_runs_and_leaves_the_others_alone() {
    let project = Project::new();
    let runs = project.root().join(".rstyle/runs");
    for name in [
        "20200101T000000Z-go",
        "20200102T000000Z-go",
        "20200101T000000Z-rust",
    ] {
        std::fs::create_dir_all(runs.join(name)).unwrap();
    }
    let output = project
        .command(NO_ANALYSES, &["go", "--check", "--format", "json"])
        .env("RSTYLE_KEEP_RUNS", "2")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let run = saved_run(&output);
    let mut left: Vec<_> = project
        .run_folders()
        .into_iter()
        .map(|path| path.file_name().unwrap().to_str().unwrap().to_string())
        .collect();
    left.sort();
    let published = run.file_name().unwrap().to_str().unwrap();
    assert_eq!(
        left,
        ["20200101T000000Z-rust", "20200102T000000Z-go", published]
    );
}

#[test]
fn a_bad_limit_fails_before_anything_runs() {
    let project = Project::new();
    let output = project
        .command(NO_ANALYSES, &["go"])
        .env("RSTYLE_KEEP_RUNS", "none")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("RSTYLE_KEEP_RUNS"), "{output:?}");
    assert!(project.run_folders().is_empty());
}

#[test]
fn an_explicit_folder_is_never_pruned() {
    let project = Project::new();
    let out = project.root().join("kept");
    let args = ["go", "--check", "--out", out.to_str().unwrap()];
    let output = project
        .command(NO_ANALYSES, &args)
        .env("RSTYLE_KEEP_RUNS", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    for _ in 0..2 {
        let output = project
            .command(NO_ANALYSES, &["go", "--check"])
            .env("RSTYLE_KEEP_RUNS", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
    }
    assert!(out.join("report.json.zst").is_file());
    assert_eq!(project.run_folders().len(), 1);
}

#[test]
fn a_run_with_nothing_under_its_root_fails_and_publishes_nothing() {
    let project = Project::new();
    let mut document = example();
    for file in document["unit"]["files"].as_array_mut().unwrap() {
        file["path"] = format!("../elsewhere/{}", file["path"].as_str().unwrap()).into();
    }
    std::fs::write(
        project.root().join("document.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let output = project.go(&["--format", "json"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("under its source root"),
        "{output:?}"
    );
    assert!(project.run_folders().is_empty());
}

#[test]
fn a_saved_run_missing_a_document_is_refused() {
    let project = Project::new();
    let run = saved_run(&project.go(&["--format", "json"]));
    std::fs::remove_file(run.join("documents/document-000000.cpg.json.zst")).unwrap();
    let output = project.go(&["--from", run.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

#[test]
fn out_must_be_new() {
    let project = Project::new();
    let out = project.root().join("out");
    let first = project.go(&["--format", "json", "--out", out.to_str().unwrap()]);
    assert_eq!(saved_run(&first), out);
    let second = project.go(&["--out", out.to_str().unwrap()]);
    assert_eq!(second.status.code(), Some(2), "{second:?}");
    assert!(
        stderr(&second).contains("must be a new directory"),
        "{second:?}"
    );
}

#[test]
fn a_frontend_analysis_keeps_its_file_and_joins_the_final_report() {
    let project = Project::new();
    let output = project
        .command(
            ANALYSES,
            &["go", "--filter", "go:calls", "--format", "json"],
        )
        // Asking for no documents means the document stream never runs.
        .env_remove("FAKE_DOCUMENT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    let frontend = &unified["frontend"][0];
    assert_eq!(frontend["analysis"], "go:calls");
    assert_eq!(frontend["status"], "complete");
    assert_eq!(frontend["file"], "frontend/calls.sarif");
    assert_eq!(frontend["runs"][0]["automationDetails"]["id"], "go/calls/");
    assert!(unified.get("check").is_none());
    let run = saved_run(&output);
    assert!(run.join("frontend/calls.sarif").is_file());
    assert!(!run.join("documents").exists() && !run.join("check.sarif").exists());
    let log: Value =
        serde_json::from_slice(&std::fs::read(run.join("report.sarif")).unwrap()).unwrap();
    assert_eq!(log["runs"], frontend["runs"]);
}

#[test]
fn a_frontend_analysis_can_read_the_documents_the_run_holds() {
    let project = Project::new();
    let received = project.root().join("received");
    let output = project
        .command(
            READERS,
            &["go", "--filter", "go:reader", "--format", "json"],
        )
        .env("FAKE_RECEIVED", &received)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    assert_eq!(unified["frontend"][0]["status"], "complete");
    assert!(
        unified.get("check").is_none(),
        "it asked for no core analysis"
    );

    let run = saved_run(&output);
    assert!(run.join("documents/documents.manifest.json").is_file());
    let bytes = std::fs::read(&received).unwrap();
    let mut sequence = SequenceReader::new(bytes.as_slice());
    assert_eq!(sequence.next().unwrap().unwrap(), example());
    assert!(sequence.next().is_none());
    sequence.complete().unwrap();
}

#[test]
fn a_source_analysis_beside_a_reader_is_handed_no_documents() {
    let project = Project::new();
    let output = project
        .command(READERS, &["go", "--filter", "go:calls", "--format", "json"])
        .env_remove("FAKE_DOCUMENT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(report(&output)["frontend"][0]["status"], "complete");
    assert!(!saved_run(&output).join("documents").exists());
}

#[test]
fn an_analysis_that_exits_before_reading_its_documents_failed() {
    let project = Project::new();
    // More than a pipe holds, so the runner is still writing when it goes.
    let mut document = serde_json::to_vec(&example()).unwrap();
    document.resize(document.len() + (1 << 20), b' ');
    std::fs::write(project.root().join("document.json"), document).unwrap();
    let output = project
        .command(
            READERS,
            &["go", "--filter", "go:quitter", "--format", "json"],
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let unified = report(&output);
    let frontend = &unified["frontend"][0];
    assert_eq!(frontend["status"], "failed");
    let error = frontend["error"].as_str().unwrap();
    assert!(error.contains("exited before reading"), "{error}");
    assert!(!saved_run(&output).join("frontend/quitter.sarif").exists());
}

#[test]
fn an_analysis_that_read_its_documents_can_still_fail() {
    let project = Project::new();
    let output = project
        .command(
            READERS,
            &["go", "--filter", "go:sinking", "--format", "json"],
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let unified = report(&output);
    let error = unified["frontend"][0]["error"].as_str().unwrap();
    assert!(error.contains("failed with"), "{error}");
}

#[test]
fn a_frontend_that_asks_for_an_unknown_input_fails_before_anything_runs() {
    let project = Project::new();
    let description = r#"{"schema":"rstyle.frontend-description/1","language":"go","analyses":[{"name":"a","input":"telepathy"}]}"#;
    let output = project.command(description, &["go"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("telepathy"), "{output:?}");
    assert!(project.run_folders().is_empty());
}

#[test]
fn the_final_report_holds_core_and_frontend_runs_in_order() {
    let project = Project::new();
    let output = project
        .command(
            ANALYSES,
            &[
                "go",
                "--filter",
                "check,patterns,go:calls",
                "--format",
                "json",
            ],
        )
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let run = saved_run(&output);
    let log: Value =
        serde_json::from_slice(&std::fs::read(run.join("report.sarif")).unwrap()).unwrap();
    let categories: Vec<_> = log["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["automationDetails"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        categories,
        ["rstyle/check/", "rstyle/patterns/", "go/calls/"]
    );
    assert_eq!(log["version"], "2.1.0");
}

#[test]
fn text_output_says_what_a_frontend_analysis_found() {
    let project = Project::new();
    let output = project
        .command(ANALYSES, &["go", "--filter", "go:calls"])
        .env_remove("FAKE_DOCUMENT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = stdout(&output);
    assert!(
        text.contains("go:calls: 1 result, in frontend/calls.sarif"),
        "{text}"
    );
}

#[test]
fn no_filter_runs_every_analysis_and_a_failure_is_the_worst_result() {
    let project = Project::new();
    let output = project
        .command(ANALYSES, &["go", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let unified = report(&output);
    assert!(unified["check"].is_object(), "core's results still arrive");
    let frontend = unified["frontend"].as_array().unwrap();
    let statuses: Vec<_> = frontend
        .iter()
        .map(|result| result["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["complete", "failed", "failed", "failed"]);
    let run = saved_run(&output);
    assert!(run.join("frontend/calls.sarif").is_file());
    let files: Vec<_> = std::fs::read_dir(run.join("frontend")).unwrap().collect();
    assert_eq!(
        files.len(),
        1,
        "a failed analysis publishes no file: {files:?}"
    );
    assert!(
        frontend[1]["error"]
            .as_str()
            .unwrap()
            .contains("failed with"),
        "{frontend:?}"
    );
    assert!(
        frontend[2]["error"]
            .as_str()
            .unwrap()
            .contains("SARIF version"),
        "{frontend:?}"
    );
    assert!(
        frontend[3]["error"]
            .as_str()
            .unwrap()
            .contains("no results file"),
        "{frontend:?}"
    );
}

#[test]
fn an_unknown_analysis_fails_before_anything_runs() {
    let project = Project::new();
    for (filter, expected) in [
        ("go:nope", "it offers calls, broken, invalid, silent"),
        ("rust:calls", "belongs to the rust frontend"),
    ] {
        let output = project
            .command(ANALYSES, &["go", "--filter", filter])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(stderr(&output).contains(expected), "{output:?}");
        assert!(!project.root().join(".rstyle").exists(), "nothing ran");
    }
}

#[test]
fn fail_on_candidates_needs_patterns() {
    let output = Project::new().go(&["--check", "--fail-on-candidates"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        stderr(&output).contains("--fail-on-candidates needs"),
        "{output:?}"
    );
}

#[test]
fn rust_flags_are_refused_for_go() {
    for flag in [
        vec!["-p", "x"],
        vec!["--all-targets"],
        vec!["--manifest-path", "Cargo.toml"],
    ] {
        let output = Project::new().go(&flag);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(stderr(&output).contains("applies to Rust"), "{output:?}");
    }
}

#[test]
fn a_stale_frontend_says_how_to_update() {
    let project = Project::new();
    let output = project
        .command(NO_ANALYSES, &["go"])
        .env("FAKE_DESCRIBE_FAILS", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("may be too old"), "{output:?}");

    let output = project
        .command(r#"{"schema":"other/9","language":"go"}"#, &["go"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("this rstyle needs"), "{output:?}");
}

/// A `go` command whose `install` puts the fake frontend where Go would.
fn fake_go(project: &Project, fail_install: bool) -> PathBuf {
    let bin = project.root().join("tool-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let gobin = project.root().join("gobin");
    std::fs::create_dir_all(&gobin).unwrap();
    let install = match fail_install {
        true => "echo 'could not read Username' >&2; exit 1".to_string(),
        false => format!(
            "echo installed >> '{0}/installs'; cp '{1}/fake-frontend' '{0}/rstyle-go'",
            gobin.display(),
            project.root().display()
        ),
    };
    script(
        &bin.join("go"),
        &format!(
            "#!/bin/sh\ncase \"$1\" in\nenv) echo '{}'; echo '{}/gopath' ;;\ninstall) {install} ;;\nesac\n",
            gobin.display(),
            project.root().display(),
        ),
    );
    bin
}

fn without_override(project: &Project, bin: &Path) -> Command {
    let mut command = project.command(NO_ANALYSES, &["go", "--check", "--format", "json"]);
    command
        .env_remove("RSTYLE_GO")
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("HOME", project.root());
    command
}

#[test]
fn a_missing_frontend_is_installed_once() {
    let project = Project::new();
    let bin = fake_go(&project, false);
    for _ in 0..2 {
        let output = without_override(&project, &bin).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
    }
    let installs = std::fs::read_to_string(project.root().join("gobin/installs")).unwrap();
    assert_eq!(installs.lines().count(), 1);
}

#[test]
fn a_failed_install_says_what_to_do_while_private() {
    let project = Project::new();
    let bin = fake_go(&project, true);
    let output = without_override(&project, &bin).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let text = stderr(&output);
    assert!(
        text.contains("GOPRIVATE=github.com/rstyled") && text.contains("RSTYLE_GO"),
        "{text}"
    );
}

#[test]
fn an_override_that_names_nothing_is_an_error_not_an_install() {
    let project = Project::new();
    let output = project
        .command(NO_ANALYSES, &["go"])
        .env("RSTYLE_GO", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("isn't a file"), "{output:?}");
}

#[test]
fn a_cxx_run_hands_on_its_build_context_after_what_every_frontend_gets() {
    let project = Project::new();
    let arguments = project.root().join("arguments");
    let output = project
        .cxx(&[
            "--source",
            ".",
            "--compilation-database",
            "build",
            "--include",
            "inc",
            "--define",
            "X=1",
            "--format",
            "json",
        ])
        .env("FAKE_ARGUMENTS", &arguments)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let written = std::fs::read_to_string(&arguments).unwrap();
    let written: Vec<&str> = written.lines().collect();
    assert_eq!(
        (written[0], written[1], written[3]),
        ("cpg", "--source", "--output-fd")
    );
    assert_eq!(
        written[5..],
        [
            "--compilation-database",
            "build",
            "--include",
            "inc",
            "--define",
            "X=1"
        ]
    );
    let run = saved_run(&output);
    assert!(
        run.file_name().unwrap().to_str().unwrap().ends_with("-cxx"),
        "{run:?}"
    );
}

#[test]
fn a_saved_cxx_run_reads_back_without_the_frontend() {
    let project = Project::new();
    let output = project
        .cxx(&["--check", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let run = saved_run(&output);
    let again = project
        .cxx(&[
            "--check",
            "--format",
            "json",
            "--from",
            run.to_str().unwrap(),
        ])
        .env("RSTYLE_CXX", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(without_run(report(&again)), without_run(report(&output)));
}

#[test]
fn a_cxx_analysis_that_reads_documents_is_all_a_run_needs_to_stream_them() {
    let project = Project::new();
    let received = project.root().join("received");
    let output = project
        .cxx(&["--filter", "cxx:reader", "--format", "json"])
        .env("FAKE_DESCRIPTION", CXX_READERS)
        .env("FAKE_RECEIVED", &received)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    let frontend = &unified["frontend"][0];
    assert_eq!(frontend["status"], "complete", "{frontend}");
    assert_eq!(
        frontend["runs"][0]["automationDetails"]["id"],
        "cxx/reader/"
    );

    let bytes = std::fs::read(&received).unwrap();
    let mut sequence = SequenceReader::new(bytes.as_slice());
    assert_eq!(sequence.next().unwrap().unwrap(), example());
    assert!(sequence.next().is_none());
    sequence.complete().unwrap();
    assert!(
        saved_run(&output)
            .join("documents/document-000000.cpg.json.zst")
            .is_file()
    );
}

#[test]
fn a_saved_cxx_run_replays_through_the_cxx_frontend() {
    let project = Project::new();
    let saved = project
        .cxx(&["--check", "--format", "json"])
        .env("FAKE_DESCRIPTION", CXX_READERS)
        .output()
        .unwrap();
    assert_eq!(saved.status.code(), Some(0), "{saved:?}");
    let run = saved_run(&saved);
    let received = project.root().join("received");

    let replayed = |analysis: &str| {
        project
            .cxx(&[
                "--filter",
                analysis,
                "--from",
                run.to_str().unwrap(),
                "--format",
                "json",
            ])
            .env("FAKE_DESCRIPTION", CXX_READERS)
            .env("FAKE_RECEIVED", &received)
            // Only the C++ frontend and the saved documents can answer.
            .env("RSTYLE_GO", "/nonexistent/frontend")
            .env_remove("FAKE_DOCUMENT")
            .output()
            .unwrap()
    };

    let output = replayed("cxx:reader");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    let frontend = &unified["frontend"][0];
    assert_eq!(frontend["status"], "complete", "{frontend}");
    assert_eq!(
        frontend["runs"][0]["automationDetails"]["id"],
        "cxx/reader/"
    );
    let bytes = std::fs::read(&received).unwrap();
    let mut sequence = SequenceReader::new(bytes.as_slice());
    assert_eq!(sequence.next().unwrap().unwrap(), example());

    let source = replayed("cxx:calls");
    assert_eq!(source.status.code(), Some(2), "{source:?}");
    assert!(
        stderr(&source).contains("`cxx:calls` reads the source"),
        "{source:?}"
    );

    let other = replayed("go:reader");
    assert_eq!(other.status.code(), Some(2), "{other:?}");
    assert!(
        stderr(&other).contains("belongs to the go frontend, not cxx"),
        "{other:?}"
    );
}

/// A build tool on `PATH` that notes how it was called, then does what `body` says.
fn build_tool(project: &Project, name: &str, body: &str) -> PathBuf {
    let root = project.root();
    let bin = root.join("tool-bin");
    std::fs::create_dir_all(&bin).unwrap();
    script(
        &bin.join(name),
        &format!(
            "#!/bin/sh\npwd > '{root}/{name}.directory'\nprintf '%s\\n' \"$@\" > '{root}/{name}.called'\necho 'progress that must not reach stdout'\n{body}\n",
            root = root.display(),
        ),
    );
    bin
}

/// A `rstyle cxx` that finds its build tools in `bin` first, and notes what it tells the frontend.
fn cxx_with_tools(project: &Project, bin: &Path, args: &[&str]) -> Command {
    let mut command = project.cxx(&[&["--format", "json"], args].concat());
    command
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("FAKE_ARGUMENTS", project.root().join("arguments"));
    command
}

fn generating(project: &Project, bin: &Path) -> Output {
    cxx_with_tools(project, bin, &[]).output().unwrap()
}

fn not_generating(project: &Project, bin: &Path) -> Output {
    cxx_with_tools(project, bin, &["--no-generate-database"])
        .output()
        .unwrap()
}

fn in_project(project: &Project, name: &str, text: &str) {
    std::fs::write(project.root().join(name), text).unwrap();
}

fn lines_of(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .lines()
        .map(str::to_string)
        .collect()
}

const ONE_ENTRY: &str = r#"[{"directory":".","file":"a.c","command":"cc a.c"}]"#;
const CMAKE_WRITES: &str = "echo '[{\"file\":\"a.c\"}]' > \"$4/compile_commands.json\"";

/// Generates over a project holding `file`, whose `program` writes to `write_to`, and checks it
/// was called as `called` from the project, and that the frontend was told what it wrote.
fn assert_generates(file: &str, program: &str, write_to: &str, called: &[&str]) {
    let project = Project::new();
    in_project(&project, file, "");
    let bin = build_tool(
        &project,
        program,
        &format!("echo '{ONE_ENTRY}' > {write_to}"),
    );
    let output = generating(&project, &bin);
    assert_eq!(output.status.code(), Some(0), "{program}: {output:?}");
    assert!(report(&output)["check"].is_object(), "stdout is the report");
    let text = stderr(&output);
    assert!(text.contains("generating compile_commands.json"), "{text}");
    assert!(text.contains("--no-generate-database skips"), "{text}");

    let root = project.root();
    let folder = root.join(".rstyle/compile-database");
    let expected: Vec<String> = called
        .iter()
        .map(|it| it.replace("{folder}", folder.to_str().unwrap()))
        .collect();
    assert_eq!(lines_of(&root.join(format!("{program}.called"))), expected);
    assert_eq!(
        lines_of(&root.join(format!("{program}.directory"))),
        [root.to_str().unwrap()]
    );
    assert_eq!(
        lines_of(&root.join("arguments"))[5..],
        ["--compilation-database", folder.to_str().unwrap()]
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".rstyle/.gitignore")).unwrap(),
        "*\n",
        "what it generates stays out of version control"
    );
}

#[test]
fn a_cmake_project_configures_into_rstyles_own_folder() {
    assert_generates(
        "CMakeLists.txt",
        "cmake",
        r#""$4/compile_commands.json""#,
        &[
            "-S",
            ".",
            "-B",
            "{folder}",
            "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON",
        ],
    );
}

#[test]
fn a_meson_project_sets_up_in_rstyles_own_folder() {
    assert_generates(
        "meson.build",
        "meson",
        r#""$2/compile_commands.json""#,
        &["setup", "{folder}"],
    );
}

#[test]
fn a_make_project_builds_under_bear() {
    assert_generates(
        "Makefile",
        "bear",
        r#""$3""#,
        &[
            "--append",
            "--output",
            "{folder}/compile_commands.json",
            "--",
            "make",
        ],
    );
}

#[test]
fn a_database_the_project_already_has_is_kept_and_nothing_runs() {
    for place in ["compile_commands.json", "build/compile_commands.json"] {
        let project = Project::new();
        in_project(&project, "CMakeLists.txt", "");
        std::fs::create_dir(project.root().join("build")).unwrap();
        in_project(&project, place, ONE_ENTRY);
        let bin = build_tool(&project, "cmake", "exit 3");

        let output = generating(&project, &bin);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(!project.root().join("cmake.called").exists(), "{place}");
        assert!(!stderr(&output).contains("rstyle: warning:"), "{output:?}");
        assert_eq!(
            lines_of(&project.root().join("arguments")).len(),
            5,
            "the frontend finds it by itself"
        );
    }
}

/// Asserts a run warned with `reason`, said how to specify a database, and went on without one.
fn assert_warned_and_guessed(project: &Project, output: &Output, reason: &str) {
    assert_eq!(output.status.code(), Some(0), "the run goes on: {output:?}");
    assert!(report(output)["check"].is_object(), "stdout is the report");
    let text = stderr(output);
    assert!(text.contains("rstyle: warning:"), "{text}");
    assert!(text.contains(reason), "{reason}: {text}");
    assert!(text.contains("--compilation-database PATH"), "{text}");
    assert_eq!(
        lines_of(&project.root().join("arguments")).len(),
        5,
        "no database named to the frontend"
    );
    assert_eq!(project.run_folders().len(), 1);
}

#[test]
fn with_no_build_to_guess_a_run_warns_what_it_looked_for() {
    let project = Project::new();
    let bin = build_tool(&project, "cmake", "exit 3");
    let output = generating(&project, &bin);
    assert_warned_and_guessed(
        &project,
        &output,
        "no CMakeLists.txt, meson.build or Makefile",
    );
    assert!(!project.root().join("cmake.called").exists());
    assert!(stderr(&output).contains("bear -- <your build command>"));
}

#[test]
fn a_build_tool_that_is_missing_is_named_in_the_warning() {
    for (file, program, manual) in [
        ("CMakeLists.txt", "cmake", "cmake -S . -B build"),
        ("meson.build", "meson", "meson setup build"),
        ("Makefile", "bear", "bear -- make"),
    ] {
        let project = Project::new();
        in_project(&project, file, "");
        // Only what the fake frontend needs, so a real build tool here can't answer.
        let bin = project.root().join("only-cat");
        std::fs::create_dir(&bin).unwrap();
        std::os::unix::fs::symlink("/bin/cat", bin.join("cat")).unwrap();
        let output = project
            .cxx(&["--format", "json"])
            .env("PATH", &bin)
            .env("FAKE_ARGUMENTS", project.root().join("arguments"))
            .output()
            .unwrap();
        assert_warned_and_guessed(&project, &output, &format!("{program} isn't on PATH"));
        assert!(stderr(&output).contains(manual), "{output:?}");
    }
}

#[test]
fn a_failed_generation_warns_and_the_run_goes_on_with_guessed_flags() {
    let project = Project::new();
    in_project(&project, "CMakeLists.txt", "");
    let bin = build_tool(
        &project,
        "cmake",
        "echo 'CMake Error: no compiler' >&2; exit 3",
    );
    let output = generating(&project, &bin);
    assert_warned_and_guessed(&project, &output, "cmake failed");
    let text = stderr(&output);
    assert!(
        text.contains("CMake Error: no compiler"),
        "its own reason: {text}"
    );
    assert!(text.contains("cmake -S . -B build"), "{text}");
    assert!(
        !text.contains("without --no-generate-database"),
        "it just tried that: {text}"
    );
}

#[test]
fn a_build_that_writes_no_usable_database_warns_too() {
    for (body, reason) in [
        ("", "didn't write a usable one"),
        (
            "echo 'not json' > \"$4/compile_commands.json\"",
            "didn't write a usable one",
        ),
        (
            "echo '[]' > \"$4/compile_commands.json\"",
            "lists nothing to compile",
        ),
    ] {
        let project = Project::new();
        in_project(&project, "CMakeLists.txt", "");
        let bin = build_tool(&project, "cmake", body);
        let output = generating(&project, &bin);
        assert_warned_and_guessed(&project, &output, reason);
    }
}

#[test]
fn a_failed_generation_falls_back_on_what_an_earlier_run_generated() {
    let project = Project::new();
    let root = project.root();
    in_project(&project, "CMakeLists.txt", "");
    let folder = root.join(".rstyle/compile-database");

    let bin = build_tool(&project, "cmake", CMAKE_WRITES);
    assert_eq!(generating(&project, &bin).status.code(), Some(0));

    let bin = build_tool(&project, "cmake", "exit 3");
    let output = generating(&project, &bin);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = stderr(&output);
    assert!(text.contains("cmake failed"), "{text}");
    assert!(text.contains("may be out of date"), "{text}");
    assert_eq!(
        lines_of(&root.join("arguments"))[5..],
        ["--compilation-database", folder.to_str().unwrap()]
    );
}

#[test]
fn a_missing_frontend_is_reported_before_any_build_runs() {
    let project = Project::new();
    in_project(&project, "CMakeLists.txt", "");
    let bin = build_tool(&project, "cmake", "exit 0");
    let output = cxx_with_tools(&project, &bin, &[])
        .env("RSTYLE_CXX", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("RSTYLE_CXX names"), "{output:?}");
    assert!(!project.root().join("cmake.called").exists());
}

#[test]
fn told_not_to_generate_a_run_guesses_the_build_warns_and_builds_nothing() {
    for (file, program, named, manual) in [
        (
            "CMakeLists.txt",
            "cmake",
            "a CMake project",
            "cmake -S . -B build",
        ),
        (
            "meson.build",
            "meson",
            "a Meson project",
            "meson setup build",
        ),
        ("Makefile", "bear", "a Make project", "bear -- make"),
    ] {
        let project = Project::new();
        in_project(&project, file, "");
        let bin = build_tool(&project, program, "exit 3");
        let output = not_generating(&project, &bin);
        assert_warned_and_guessed(&project, &output, named);
        let text = stderr(&output);
        assert!(text.contains(manual), "{text}");
        assert!(text.contains("without --no-generate-database"), "{text}");
        assert!(!project.root().join(format!("{program}.called")).exists());
    }
}

#[test]
fn told_not_to_generate_with_no_build_to_guess_it_still_says_how_to_specify_one() {
    let project = Project::new();
    let bin = project.root().join("tool-bin");
    std::fs::create_dir(&bin).unwrap();
    let output = not_generating(&project, &bin);
    assert_warned_and_guessed(
        &project,
        &output,
        "no CMakeLists.txt, meson.build or Makefile",
    );
    let text = stderr(&output);
    assert!(text.contains("bear -- <your build command>"), "{text}");
    assert!(
        !text.contains("without --no-generate-database"),
        "it has no build to ask: {text}"
    );
}

#[test]
fn naming_a_database_needs_no_guess_no_build_and_no_warning() {
    let project = Project::new();
    in_project(&project, "CMakeLists.txt", "");
    let bin = build_tool(&project, "cmake", "exit 3");
    let output = cxx_with_tools(&project, &bin, &["--compilation-database", "elsewhere"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!stderr(&output).contains("rstyle: warning:"), "{output:?}");
    assert!(!project.root().join("cmake.called").exists());
}

#[test]
fn a_database_in_a_build_directory_is_found_without_being_named() {
    let project = Project::new();
    let root = project.root();
    in_project(&project, "CMakeLists.txt", "");
    for folder in ["cmake-build-release", "cmake-build-debug", "third_party"] {
        std::fs::create_dir(root.join(folder)).unwrap();
        in_project(
            &project,
            &format!("{folder}/compile_commands.json"),
            ONE_ENTRY,
        );
    }
    let bin = build_tool(&project, "cmake", "exit 3");
    let debug = root.join("cmake-build-debug");

    for output in [generating(&project, &bin), not_generating(&project, &bin)] {
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let text = stderr(&output);
        assert!(
            text.contains("using the compile_commands.json in"),
            "{text}"
        );
        assert!(text.contains(debug.to_str().unwrap()), "{text}");
        assert!(text.contains("also found"), "{text}");
        assert!(!text.contains("third_party"), "not a build: {text}");
        assert!(!text.contains("rstyle: warning:"), "{text}");
        assert!(!root.join("cmake.called").exists(), "nothing to generate");
    }
    assert_eq!(
        lines_of(&root.join("arguments"))[5..],
        ["--compilation-database", debug.to_str().unwrap()]
    );
}

#[test]
fn a_generated_database_is_refreshed_each_run_and_reused_when_told_not_to_generate() {
    let project = Project::new();
    let root = project.root();
    in_project(&project, "CMakeLists.txt", "");
    let bin = build_tool(&project, "cmake", CMAKE_WRITES);
    let folder = root.join(".rstyle/compile-database");
    let named = ["--compilation-database", folder.to_str().unwrap()];

    assert_eq!(generating(&project, &bin).status.code(), Some(0));
    std::fs::remove_file(root.join("cmake.called")).unwrap();

    let reused = not_generating(&project, &bin);
    assert_eq!(reused.status.code(), Some(0), "{reused:?}");
    assert_eq!(lines_of(&root.join("arguments"))[5..], named);
    assert!(stderr(&reused).contains("generated earlier"), "{reused:?}");
    assert!(!root.join("cmake.called").exists());

    assert_eq!(generating(&project, &bin).status.code(), Some(0));
    assert!(root.join("cmake.called").exists(), "each run refreshes it");
    assert_eq!(lines_of(&root.join("arguments"))[5..], named);
}

#[test]
fn not_generating_conflicts_with_a_named_database_and_a_saved_run() {
    let project = Project::new();
    for extra in [["--compilation-database", "build"], ["--from", "somewhere"]] {
        let output = project
            .cxx(&[&["--no-generate-database"], &extra[..]].concat())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(
            stderr(&output).contains("cannot be used with"),
            "{output:?}"
        );
    }
}

#[test]
fn a_missing_cxx_frontend_says_how_to_build_it_and_installs_nothing() {
    let project = Project::new();
    let output = project
        .cxx(&[])
        .env_remove("RSTYLE_CXX")
        .env("PATH", project.root().join("no-such-directory"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let text = stderr(&output);
    assert!(
        text.contains("rstyle-cxx isn't on PATH") && text.contains("RSTYLE_CXX"),
        "{text}"
    );
    assert!(project.run_folders().is_empty());
}

#[test]
fn a_cxx_override_that_names_nothing_is_an_error() {
    let output = Project::new()
        .cxx(&[])
        .env("RSTYLE_CXX", "/nonexistent/frontend")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("RSTYLE_CXX names"), "{output:?}");
}

#[test]
fn a_frontend_for_another_language_is_refused_for_cxx() {
    let project = Project::new();
    let output = project
        .command(NO_ANALYSES, &["cxx"])
        .env("RSTYLE_CXX", project.root().join("fake-frontend"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("cxx"), "{output:?}");
    assert!(project.run_folders().is_empty());
}

#[test]
fn a_stale_cxx_frontend_says_to_rebuild_it() {
    let output = Project::new()
        .cxx(&[])
        .env("FAKE_DESCRIBE_FAILS", "1")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("rebuild it from"), "{output:?}");
}

#[test]
fn rust_flags_are_refused_for_cxx() {
    for flag in [
        vec!["-p", "x"],
        vec!["--all-targets"],
        vec!["--manifest-path", "Cargo.toml"],
    ] {
        let output = Project::new().cxx(&flag).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let text = stderr(&output);
        assert!(
            text.contains("applies to Rust") && text.contains("C++"),
            "{output:?}"
        );
    }
}

#[test]
fn an_analysis_of_another_frontend_is_refused_for_cxx() {
    let output = Project::new()
        .cxx(&["--filter", "go:calls"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}

/// `rstyle rust` over a fixture, through the driver next to the test binary.
fn rust_run(root: &Path, args: &[&str]) -> Output {
    Command::new(cargo_bin("rstyle"))
        .arg("rust")
        .args(args)
        .current_dir(root)
        .env("RSTYLE_DRIVER", driver())
        // Runs live under the fixture's own target, not a shared one.
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap()
}

#[test]
fn a_rust_run_saves_beside_go_runs_and_matches_the_compiler_route() {
    let directory = fixture("animals");
    let root = directory.path().canonicalize().unwrap();
    let output = rust_run(&root, &["--format", "json", "--no-ignore"]);
    assert!(
        output.status.code().is_some_and(|code| code < 2),
        "{output:?}"
    );
    let unified = report(&output);
    let run = saved_run(&output);
    assert_eq!(run.parent().unwrap(), root.join(".rstyle/runs"));
    assert!(
        run.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with("Z-rust")
    );
    assert!(root.join("target/rstyle/runs/advisory-build").is_dir());
    assert!(run.join("documents/documents.manifest.json").is_file());
    assert_eq!(
        std::fs::read_to_string(root.join(".rstyle/.gitignore")).unwrap(),
        "*\n"
    );

    let (_, documents) = routes_over(&root, &[]);
    assert_eq!(
        unified["patterns"]["candidates"],
        documents["evaluation"]["candidates"]
    );
    assert!(!documents_in(&run.join("documents")).is_empty());
    assert_locations_resolve(&root, &run);

    assert_reads_back(&root, &run, &unified);

    let expected = unified.clone();
    let second = rust_run(&root, &["--format", "json", "--no-ignore"]);
    assert_eq!(
        without_run(report(&second)),
        without_run(unified),
        "reusing the capture changes nothing"
    );
    assert_cargo_clean_leaves_the_run(&root, &run);
    assert_reads_back(&root, &run, &expected);
}

fn assert_reads_back(root: &Path, run: &Path, unified: &Value) {
    let again = rust_run(
        root,
        &[
            "--format",
            "json",
            "--no-ignore",
            "--from",
            run.to_str().unwrap(),
        ],
    );
    assert_eq!(report(&again), without_run(unified.clone()), "{again:?}");
}

fn assert_cargo_clean_leaves_the_run(root: &Path, run: &Path) {
    // An inherited CARGO_TARGET_DIR would make this clean the build running the test.
    let cleaned = Command::new("cargo")
        .args(["clean", "--quiet", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .current_dir(workspace_root())
        .env_remove("CARGO_TARGET_DIR")
        .status()
        .unwrap();
    assert!(cleaned.success());
    assert!(
        !root.join("target").exists(),
        "`cargo clean` clears the capture"
    );
    assert!(run.join("report.json.zst").is_file(), "but not a saved run");
}

fn assert_locations_resolve(root: &Path, run: &Path) {
    let log: Value =
        serde_json::from_slice(&std::fs::read(run.join("patterns.sarif")).unwrap()).unwrap();
    let results = log["runs"][0]["results"].as_array().unwrap();
    assert!(!results.is_empty());
    for result in results {
        let artifact = &result["locations"][0]["physicalLocation"]["artifactLocation"];
        assert_eq!(artifact["uriBaseId"], "%SRCROOT%", "{artifact}");
        assert!(
            root.join(artifact["uri"].as_str().unwrap()).is_file(),
            "{artifact}"
        );
    }
}

#[test]
fn a_rust_run_missing_a_document_cannot_read_as_a_cleaner_one() {
    let directory = fixture("animals");
    let root = directory.path().canonicalize().unwrap();
    let output = rust_run(&root, &["--check", "--format", "json"]);
    let run = saved_run(&output);
    let documents = documents_in(&run.join("documents"));
    std::fs::remove_file(&documents[0]).unwrap();
    let again = rust_run(&root, &["--check", "--from", run.to_str().unwrap()]);
    assert_eq!(again.status.code(), Some(2), "{again:?}");
    assert!(stderr(&again).contains(".cpg.json"), "{again:?}");
}

#[test]
fn rust_has_no_analyses_of_its_own_yet() {
    let directory = fixture("animals");
    let output = rust_run(directory.path(), &["--filter", "rust:calls"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(stderr(&output).contains("it offers none"), "{output:?}");
}

/// A crate whose function stores a value it replaces before reading it.
fn crate_with_a_dead_store() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "#![allow(unused_assignments)]\n\
         fn f() -> i32 { 1 }\n\
         pub fn dead() -> i32 {\n    let mut v = f();\n    v = f();\n    v\n}\n\
         // rstyle-allow(dead_store): the first call's effect is the point\n\
         pub fn allowed() -> i32 {\n    let mut v = f();\n    v = f();\n    v\n}\n\
         pub fn alive() -> i32 {\n    let mut v = f();\n    v += f();\n    v\n}\n",
    )
    .unwrap();
    directory
}

#[test]
fn a_rust_run_finds_a_value_nothing_reads() {
    let directory = crate_with_a_dead_store();
    let output = rust_run(
        directory.path(),
        &["--filter", "defects", "--format", "json"],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    let defects = &unified["defects"];
    assert_eq!(defects["suppressed"], 1, "{defects}");
    let rows = defects["defects"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["rule"], "dead_store");
    assert_eq!(rows[0]["path"], "src/lib.rs");
    assert_eq!(rows[0]["line"], 4);
    assert!(saved_run(&output).join("defects.sarif").is_file());
}

/// A crate with a build script, whose own functions no route should report.
fn crate_with_build_script() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("build.rs"),
        "fn main() {\n    helper();\n}\nfn helper() {}\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
    )
    .unwrap();
    directory
}

#[test]
fn a_rust_run_reads_what_the_default_route_reads() {
    let directory = crate_with_build_script();
    let root = directory.path().canonicalize().unwrap();
    let output = rust_run(&root, &["--check", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    let documents = documents_in(&saved_run(&output).join("documents"));
    assert!(
        documents
            .iter()
            .all(|path| !path.contains("build_script_build")),
        "{documents:?}"
    );

    let default_route = rstyle(&root, &["--format", "json"]);
    assert_eq!(default_route.status.code(), Some(0), "{default_route:?}");
    let default_route = report(&default_route);
    assert_eq!(unified["check"]["summary"]["functions"], 1);
    assert_eq!(
        unified["check"]["summary"], default_route["summary"],
        "{default_route}"
    );
    assert_eq!(unified["check"]["functions"], default_route["functions"]);
}

/// Two build scripts define the same `main`, and a run reads neither, so their shared function
/// id can't reject it. A whole run reads every analysis's documents together.
#[test]
fn two_crates_with_build_scripts_make_one_run() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    for name in ["a", "b"] {
        let member = root.join(name);
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n"),
        )
        .unwrap();
        std::fs::write(member.join("build.rs"), "fn main() {}\n").unwrap();
        std::fs::write(
            member.join("src/lib.rs"),
            format!("pub fn {name}_value() -> u32 {{\n    1\n}}\n"),
        )
        .unwrap();
    }
    let output = rust_run(&root, &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let unified = report(&output);
    assert_eq!(unified["check"]["summary"]["functions"], 2);
    let flow = unified["flow"]["functions"].as_array().unwrap();
    assert_eq!(flow.len(), 2, "{flow:?}");
    assert!(
        flow.iter().all(|row| row["flow"]["standing"] != "unknown"),
        "{flow:?}"
    );
}
