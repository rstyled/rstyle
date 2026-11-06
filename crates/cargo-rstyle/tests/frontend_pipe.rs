//! The pipe a runner hands a frontend, and the input descriptor a receiver reads, on every
//! platform: a descriptor on Unix and an inherited handle on Windows. CI runs this on Windows.

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn example_document() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rstyle-core/src/cpg/testdata/example.cpg.json")
        .canonicalize()
        .unwrap()
}

/// `cargo test --test` doesn't build examples, so build the frontend once.
fn frontend() -> &'static Path {
    static FRONTEND: OnceLock<PathBuf> = OnceLock::new();
    FRONTEND.get_or_init(|| {
        let status = Command::new("cargo")
            .args([
                "build",
                "--quiet",
                "-p",
                "cargo-rstyle",
                "--example",
                "fake_frontend",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .unwrap();
        assert!(status.success(), "building the fake frontend failed");
        let name = format!("fake_frontend{}", std::env::consts::EXE_SUFFIX);
        cargo_bin("rstyle").with_file_name("examples").join(name)
    })
}

fn run_go(project: &Path, args: &[&str]) -> Output {
    Command::new(cargo_bin("rstyle"))
        .arg("go")
        .args(args)
        .current_dir(project)
        .env("RSTYLE_GO", frontend())
        .env("FAKE_DOCUMENT", example_document())
        .output()
        .unwrap()
}

#[test]
fn a_frontend_streams_a_run_over_the_pipe_it_inherits() {
    let project = tempfile::tempdir().unwrap();
    let output = run_go(project.path(), &["--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["check"]["summary"]["functions"], 3);
    assert!(report["patterns"].is_object() && report["runtime_bounds"].is_object());

    let run = Path::new(report["run"].as_str().unwrap());
    assert!(run.starts_with(project.path().canonicalize().unwrap().join(".rstyle/runs")));
    for file in [
        "check.sarif",
        "patterns.sarif",
        "report.json.zst",
        "run.json",
        "documents/documents.manifest.json",
    ] {
        assert!(run.join(file).is_file(), "{file}");
    }
    let log: Value =
        serde_json::from_slice(&std::fs::read(run.join("check.sarif")).unwrap()).unwrap();
    let root = log["runs"][0]["originalUriBaseIds"]["%SRCROOT%"]["uri"]
        .as_str()
        .unwrap();
    assert!(
        root.starts_with("file:///") && root.ends_with('/'),
        "{root}"
    );
    assert!(!root.contains("%3F"), "no verbatim prefix in {root}");
}

#[test]
fn a_saved_run_reads_back_without_its_frontend() {
    let project = tempfile::tempdir().unwrap();
    let output = run_go(project.path(), &["--check", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let mut report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let run = report.as_object_mut().unwrap().remove("run").unwrap();
    let again = run_go(
        project.path(),
        &[
            "--check",
            "--format",
            "json",
            "--from",
            run.as_str().unwrap(),
        ],
    );
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&again.stdout).unwrap(),
        report
    );
}

#[cfg(windows)]
mod handles {
    use super::*;
    use std::ffi::c_void;
    use std::io::Write;
    use std::os::windows::io::AsRawHandle;
    use std::process::Stdio;

    unsafe extern "system" {
        fn SetHandleInformation(handle: *mut c_void, mask: u32, flags: u32) -> i32;
    }

    #[test]
    fn a_receiver_reads_a_stream_from_an_inherited_handle() {
        let (reader, mut writer) = std::io::pipe().unwrap();
        let handle = reader.as_raw_handle();
        // SAFETY: `reader` owns this open handle for the whole call.
        assert_ne!(unsafe { SetHandleInformation(handle.cast(), 1, 1) }, 0);
        let child = Command::new(cargo_bin("cargo-rstyle"))
            .args(["cpg", "receive", "--format", "json", "--input-fd"])
            .arg((handle as usize).to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        drop(reader);
        writer.write_all(b"\x1e").unwrap();
        writer
            .write_all(&std::fs::read(example_document()).unwrap())
            .unwrap();
        writer.write_all(b"\n\x1enull\n").unwrap();
        drop(writer);
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(receipt["status"], "accepted");
    }

    #[test]
    fn a_value_that_is_no_handle_is_an_error() {
        let output = Command::new(cargo_bin("cargo-rstyle"))
            .args(["cpg", "receive", "--input-fd", "123456"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("Cannot duplicate input handle"));
    }
}
