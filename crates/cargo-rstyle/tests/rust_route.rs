//! The Rust route through the real driver, on every platform. CI runs this on Windows, where the
//! rest of the compiler tests don't run.

use assert_cmd::cargo::cargo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::OnceLock;
use tempfile::TempDir;

/// `cargo test --test` doesn't build sibling binaries, so build the driver once.
fn build_driver() {
    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| {
        let status = Command::new("cargo")
            .args(["build", "--quiet", "-p", "rstyle-driver"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .unwrap();
        assert!(status.success(), "building rstyle-driver failed");
    });
}

/// A crate with a build script, whose own functions no route reads.
fn workspace() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("build.rs"), "fn main() {}\n").unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n\npub fn twice(a: u32) -> u32 {\n    add(a, a)\n}\n",
    )
    .unwrap();
    directory
}

fn run(program: &str, root: &Path, args: &[&str]) -> Output {
    build_driver();
    Command::new(cargo_bin(program))
        .args(args)
        .current_dir(root)
        // Runs and builds stay in the crate's own target.
        .env_remove("CARGO_TARGET_DIR")
        .output()
        .unwrap()
}

fn report(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {output:?}"))
}

#[test]
fn a_rust_run_reads_a_workspace_through_the_driver() {
    let directory = workspace();
    let root = directory.path();
    let mut unified = report(&run("rstyle", root, &["rust", "--format", "json"]));
    assert_eq!(unified["check"]["summary"]["functions"], 2);
    assert!(unified["patterns"].is_object() && unified["runtime_bounds"].is_object());
    assert_eq!(unified["flow"]["functions"].as_array().unwrap().len(), 2);

    let saved = unified.as_object_mut().unwrap().remove("run").unwrap();
    let saved = Path::new(saved.as_str().unwrap());
    assert_eq!(saved.parent().unwrap().file_name().unwrap(), "runs");
    assert!(saved.join("report.json.zst").is_file());
    assert!(saved.join("report.sarif").is_file());

    let log: Value =
        serde_json::from_slice(&std::fs::read(saved.join("check.sarif")).unwrap()).unwrap();
    let source = log["runs"][0]["originalUriBaseIds"]["%SRCROOT%"]["uri"]
        .as_str()
        .unwrap();
    assert!(
        source.starts_with("file:///") && source.ends_with('/'),
        "{source}"
    );
    assert!(!source.contains("%3F"), "no verbatim prefix in {source}");

    let paths = unified["check"]["functions"].as_array().unwrap();
    for function in paths {
        let path = function["path"].as_str().unwrap();
        assert!(
            path.ends_with("lib.rs") && !Path::new(path).is_absolute(),
            "{path}"
        );
    }

    let again = run(
        "rstyle",
        root,
        &[
            "rust",
            "--format",
            "json",
            "--from",
            saved.to_str().unwrap(),
        ],
    );
    assert_eq!(report(&again), unified);
}

#[test]
fn a_manifest_path_reads_the_same_workspace_from_anywhere() {
    let directory = workspace();
    let elsewhere = tempfile::tempdir().unwrap();
    let manifest = directory.path().join("Cargo.toml");
    let args = [
        "rust",
        "--check",
        "--format",
        "json",
        "--manifest-path",
        manifest.to_str().unwrap(),
    ];
    let unified = report(&run("rstyle", elsewhere.path(), &args));
    assert_eq!(unified["check"]["summary"]["functions"], 2);
    assert!(directory.path().join(".rstyle/runs").is_dir());
    assert_eq!(std::fs::read_dir(elsewhere.path()).unwrap().count(), 0);
}

#[test]
fn the_default_route_reads_what_a_run_reads_and_shares_its_own_capture() {
    let directory = workspace();
    let root = directory.path();
    let unified = report(&run(
        "rstyle",
        root,
        &["rust", "--check", "--format", "json"],
    ));
    let default = report(&run("cargo-rstyle", root, &["--format", "json"]));
    assert_eq!(unified["check"]["summary"], default["summary"]);
    assert_eq!(unified["check"]["functions"], default["functions"]);

    let patterns = run("cargo-rstyle", root, &["patterns", "--format", "json"]);
    assert_eq!(patterns.status.code(), Some(0), "{patterns:?}");
    let patterns = report(&patterns);
    let saved = Path::new(patterns["run_directory"].as_str().unwrap());
    let cargo = std::fs::read_to_string(saved.join("cargo.jsonl")).unwrap();
    let compiled = cargo
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["reason"] == "compiler-artifact")
        .any(|row| row["fresh"] == false);
    assert!(!compiled, "patterns compiled again: {cargo}");
}
