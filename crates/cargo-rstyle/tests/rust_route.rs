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

    let rows = unified["runtime_bounds"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let path = row["path"].as_str().unwrap();
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

/// Whether the newest capture compiled any of the workspace's own crates.
fn compiled_the_workspace(root: &Path) -> bool {
    let mut runs: Vec<_> = std::fs::read_dir(root.join("target/rstyle/runs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("cargo.jsonl").is_file())
        .collect();
    runs.sort_by_key(|run| {
        std::fs::metadata(run.join("cargo.jsonl"))
            .unwrap()
            .modified()
            .unwrap()
    });
    let cargo = std::fs::read_to_string(runs.last().unwrap().join("cargo.jsonl")).unwrap();
    cargo
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter(|row| row["reason"] == "compiler-artifact")
        .filter(|row| row["target"]["name"] == "app")
        .any(|row| row["fresh"] == false)
}

#[test]
fn a_run_compiles_only_what_changed() {
    let directory = workspace();
    let root = directory.path();
    let first = report(&run(
        "rstyle",
        root,
        &["rust", "--check", "--format", "json"],
    ));
    assert!(compiled_the_workspace(root));
    assert_eq!(first["check"]["summary"]["functions"], 2);

    let second = report(&run(
        "rstyle",
        root,
        &["rust", "--check", "--format", "json"],
    ));
    assert!(
        !compiled_the_workspace(root),
        "the same source compiled again"
    );
    assert_eq!(second["check"], first["check"]);

    let default = report(&run("cargo-rstyle", root, &["--format", "json"]));
    assert!(
        !compiled_the_workspace(root),
        "`cargo rstyle` compiled again"
    );
    assert_eq!(default["summary"], first["check"]["summary"]);

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn only() -> u32 {\n    1\n}\n",
    )
    .unwrap();
    let edited = report(&run(
        "rstyle",
        root,
        &["rust", "--check", "--format", "json"],
    ));
    assert!(compiled_the_workspace(root), "an edit didn't compile");
    assert_eq!(edited["check"]["summary"]["functions"], 1);
}

#[test]
fn a_compile_error_reaches_the_user_and_publishes_nothing() {
    let directory = workspace();
    let root = directory.path();
    std::fs::write(root.join("src/lib.rs"), "pub fn broken( {\n").unwrap();
    let output = run("rstyle", root, &["rust", "--check"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Cargo compilation failed"), "{stderr}");
    assert!(
        stderr.contains("lib.rs"),
        "the compiler's own error: {stderr}"
    );
    let runs = root.join(".rstyle/runs");
    let published = std::fs::read_dir(&runs).map_or(0, Iterator::count);
    assert_eq!(published, 0, "{}", runs.display());
}

#[test]
fn all_targets_reads_a_function_once_however_many_compilations_hold_it() {
    let directory = workspace();
    let root = directory.path();
    let mut source = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    source.push_str("\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn adds() {\n        assert_eq!(super::add(1, 1), 2);\n    }\n}\n");
    std::fs::write(root.join("src/lib.rs"), source).unwrap();
    let args = ["rust", "--all-targets", "--format", "json"];
    let unified = report(&run("rstyle", root, &args));
    assert_eq!(unified["check"]["summary"]["functions"], 3, "{unified}");
}
