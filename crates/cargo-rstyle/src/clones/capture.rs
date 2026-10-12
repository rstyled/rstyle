//! Isolated Cargo compilation and an explicit compiler-unit ledger.

use super::{Args, read_json, write_json};
use crate::{Cli, run};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Target {
    pub package: String,
    pub manifest_dir: PathBuf,
    pub source: PathBuf,
    pub name: String,
    pub kind: Vec<String>,
    pub test: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Plan {
    pub root: PathBuf,
    pub driver: PathBuf,
    pub targets: Vec<Target>,
}

#[derive(Serialize, Deserialize)]
pub struct Unit {
    pub target: Target,
    pub test: bool,
    pub invocation: Vec<String>,
    pub directory: PathBuf,
    pub artifacts: Vec<Artifact>,
}

#[derive(Serialize, Deserialize)]
pub struct Artifact {
    pub file: String,
    pub sha256: String,
}

pub struct Capture {
    pub directory: PathBuf,
    pub plan: Plan,
    pub units: Vec<Unit>,
    pub failures: Vec<String>,
}

pub fn prepare(cli: &Cli) -> Result<Capture, String> {
    let metadata = crate::workspace::load(cli.manifest_path.as_deref())?;
    if let Some(package) = metadata.unknown_packages(&cli.packages).first() {
        return Err(format!("unknown package {package}"));
    }
    let parent = metadata.target_directory.join("rstyle/clones");
    std::fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let directory = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?
        .keep();
    let plan = Plan {
        root: metadata.workspace_root,
        driver: run::driver_path()?,
        targets: targets(cli)?,
    };
    write_json(&directory.join("plan.json"), &plan)?;
    Ok(Capture {
        directory,
        plan,
        units: Vec::new(),
        failures: Vec::new(),
    })
}

fn targets(cli: &Cli) -> Result<Vec<Target>, String> {
    let mut command = Command::new("cargo");
    command
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .env("RUSTUP_TOOLCHAIN", run::pinned_toolchain());
    if let Some(path) = &cli.manifest_path {
        command.arg("--manifest-path").arg(path);
    }
    let output = command.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    selected_targets(&value, cli)
}

fn selected_targets(metadata: &Value, cli: &Cli) -> Result<Vec<Target>, String> {
    let mut result = Vec::new();
    for package in metadata["packages"].as_array().ok_or("missing packages")? {
        let name = package["name"].as_str().ok_or("missing package name")?;
        if !cli.packages.is_empty() && !cli.packages.iter().any(|p| p == name) {
            continue;
        }
        for target in package["targets"].as_array().ok_or("missing targets")? {
            if !wanted_target(target, cli.all_targets) {
                continue;
            }
            if target["required-features"]
                .as_array()
                .is_some_and(|f| !f.is_empty())
            {
                return Err("targets with required-features are not supported yet".into());
            }
            result.push(target_record(package, target)?);
        }
    }
    if result.is_empty() {
        return Err("selection contains no Rust compilation targets".into());
    }
    Ok(result)
}

fn wanted_target(target: &Value, all: bool) -> bool {
    target["kind"].as_array().is_some_and(|kinds| {
        kinds.iter().any(|kind| {
            matches!(kind.as_str(), Some("lib" | "rlib" | "bin" | "proc-macro"))
                || (all && matches!(kind.as_str(), Some("test" | "example" | "bench")))
        })
    })
}

fn target_record(package: &Value, target: &Value) -> Result<Target, String> {
    let manifest = PathBuf::from(
        package["manifest_path"]
            .as_str()
            .ok_or("missing manifest")?,
    );
    Ok(Target {
        package: package["id"]
            .as_str()
            .ok_or("missing package identity")?
            .into(),
        manifest_dir: manifest
            .parent()
            .ok_or("missing manifest directory")?
            .into(),
        source: PathBuf::from(target["src_path"].as_str().ok_or("missing target source")?),
        name: target["name"].as_str().ok_or("missing target name")?.into(),
        kind: serde_json::from_value(target["kind"].clone()).map_err(|e| e.to_string())?,
        test: target["test"].as_bool().unwrap_or(false),
    })
}

fn command(capture: &Capture, cli: &Cli, args: &Args) -> Result<Command, String> {
    let mut command = Command::new("cargo");
    run::clear_rstyle_environment(&mut command);
    command
        .args(["check", "--message-format=json"])
        .env("RUSTUP_TOOLCHAIN", run::pinned_toolchain())
        .env(
            "RUSTC_WORKSPACE_WRAPPER",
            std::env::current_exe().map_err(|e| e.to_string())?,
        )
        .env("RUSTC_WRAPPER", "")
        .env_remove("RUSTC")
        .env("CARGO_TARGET_DIR", capture.directory.join("build"))
        .env("CARGO_INCREMENTAL", "0")
        .env("RSTYLE_CAPTURE_PLAN", capture.directory.join("plan.json"));
    selection_args(&mut command, cli);
    build_args(&mut command, args);
    Ok(command)
}

fn selection_args(command: &mut Command, cli: &Cli) {
    if cli.packages.is_empty() {
        command.arg("--workspace");
    }
    for package in &cli.packages {
        command.args(["-p", package]);
    }
    if cli.all_targets {
        command.arg("--all-targets");
    }
    if let Some(path) = &cli.manifest_path {
        command.arg("--manifest-path").arg(path);
    }
}

fn build_args(command: &mut Command, args: &Args) {
    if args.release {
        command.arg("--release");
    }
    if args.locked {
        command.arg("--locked");
    }
    if args.offline {
        command.arg("--offline");
    }
    if args.no_default_features {
        command.arg("--no-default-features");
    }
    if args.all_features {
        command.arg("--all-features");
    }
    if let Some(features) = &args.features {
        command.args(["--features", features]);
    }
    if let Some(target) = &args.target {
        command.args(["--target", target]);
    }
    if let Some(jobs) = args.jobs {
        command.arg("--jobs").arg(jobs.to_string());
    }
}

pub fn compile(capture: &mut Capture, cli: &Cli, args: &Args) -> Result<(), String> {
    let output = command(capture, cli, args)?
        .output()
        .map_err(|e| e.to_string())?;
    std::fs::write(capture.directory.join("cargo.jsonl"), &output.stdout)
        .map_err(|e| e.to_string())?;
    std::fs::write(capture.directory.join("cargo.stderr"), &output.stderr)
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        capture.failures.push(format!(
            "Cargo compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
        return Ok(());
    }
    if let Err(error) = reconcile(capture, &output.stdout, cli.all_targets) {
        capture.failures.push(error);
    }
    Ok(())
}

fn reconcile(capture: &mut Capture, cargo: &[u8], all: bool) -> Result<(), String> {
    let artifacts: Vec<Value> = cargo
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    for target in &capture.plan.targets {
        if !artifacts.iter().any(|a| matches_artifact(a, target)) {
            capture.failures.push(format!(
                "missing Cargo artifact for {}:{}",
                target.package, target.name
            ));
        }
    }
    for artifact in artifacts
        .iter()
        .filter(|a| a["reason"] == "compiler-artifact")
    {
        let Some(target) = capture
            .plan
            .targets
            .iter()
            .find(|t| matches_artifact(artifact, t))
        else {
            continue;
        };
        let test = artifact["profile"]["test"]
            .as_bool()
            .ok_or("missing artifact test context")?;
        let unit_path = unit_ledger_path(capture, target, test)?;
        let unit: Unit = read_json(&unit_path)?;
        if unit.target.package != target.package
            || unit.target.source != target.source
            || unit.test != test
        {
            return Err("compiler-unit ledger disagrees with Cargo artifact".into());
        }
        capture.units.push(unit);
    }
    check_expected(capture, all);
    Ok(())
}

fn matches_artifact(artifact: &Value, target: &Target) -> bool {
    artifact["reason"] == "compiler-artifact"
        && artifact["package_id"] == target.package
        && artifact["target"]["src_path"] == target.source.to_string_lossy().as_ref()
        && artifact["target"]["name"] == target.name
}

fn unit_ledger_path(capture: &Capture, target: &Target, test: bool) -> Result<PathBuf, String> {
    let key = super::digest(
        &serde_json::to_vec(&json!([target.package, target.source, test]))
            .map_err(|e| e.to_string())?,
    );
    Ok(capture.directory.join(format!("unit-{key}.json")))
}

fn check_expected(capture: &mut Capture, all: bool) {
    for target in &capture.plan.targets {
        let tests = all
            && target.test
            && target
                .kind
                .iter()
                .any(|k| matches!(k.as_str(), "lib" | "bin"));
        for test in std::iter::once(target.kind.iter().any(|k| k == "test" || k == "bench"))
            .chain(tests.then_some(true))
        {
            if !capture.units.iter().any(|u| {
                u.target.package == target.package
                    && u.target.source == target.source
                    && u.test == test
            }) {
                capture.failures.push(format!(
                    "missing expected compilation unit {}:{} test={test}",
                    target.package, target.name
                ));
            }
        }
    }
}

/// Internal workspace wrapper. Each invocation owns its directory and only
/// publishes the ledger entry after rustc has succeeded and facts are present.
pub fn wrapper(plan_path: &Path, invocation: Vec<String>) -> Result<u8, String> {
    let plan: Plan = read_json(plan_path)?;
    let target = invocation_target(&plan, &invocation)?;
    let mut command = Command::new(&plan.driver);
    run::clear_rstyle_environment(&mut command);
    command.args(&invocation);
    let Some(target) = target else {
        let status = command.status().map_err(|e| e.to_string())?;
        return Ok(if status.success() { 0 } else { 2 });
    };
    capture_unit(plan_path, target, invocation, command)
}

fn invocation_target(plan: &Plan, invocation: &[String]) -> Result<Option<Target>, String> {
    if invocation.iter().any(|a| a.starts_with("--print")) {
        return Ok(None);
    }
    let Some(directory) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return Ok(None);
    };
    let directory = PathBuf::from(directory);
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let matches: Vec<_> = plan
        .targets
        .iter()
        .filter(|t| {
            t.manifest_dir == directory
                && invocation
                    .iter()
                    .any(|a| a.ends_with(".rs") && cwd.join(a) == t.source)
        })
        .cloned()
        .collect();
    match matches.as_slice() {
        [] => Ok(None),
        [target] => Ok(Some(target.clone())),
        _ => Err("ambiguous target source in capture plan".into()),
    }
}

fn capture_unit(
    plan: &Path,
    target: Target,
    invocation: Vec<String>,
    mut command: Command,
) -> Result<u8, String> {
    let root = plan.parent().ok_or("missing run directory")?;
    let directory = tempfile::Builder::new()
        .prefix("invocation-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?
        .keep();
    let test = invocation.iter().any(|a| a == "--test");
    let unit = Unit {
        target,
        test,
        invocation,
        directory,
        artifacts: Vec::new(),
    };
    write_json(&unit.directory.join("started.json"), &unit)?;
    command.env("RSTYLE_HIR_DOCUMENT_DIR", &unit.directory);
    let status = command.status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Ok(2);
    }
    let mut unit = unit;
    unit.artifacts = artifact_inventory(&unit.directory)?;
    let key = super::digest(
        &serde_json::to_vec(&json!([unit.target.package, unit.target.source, test]))
            .map_err(|e| e.to_string())?,
    );
    let path = root.join(format!("unit-{key}.json"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("duplicate or missing unit ledger: {e}"))?;
    serde_json::to_writer(file, &unit).map_err(|e| e.to_string())?;
    Ok(0)
}

fn artifact_inventory(directory: &Path) -> Result<Vec<Artifact>, String> {
    let facts: Vec<rstyle_pdg::HirFnFacts> = read_json(&directory.join("facts.json"))?;
    let mut files = vec!["facts.json".to_string()];
    for index in 0..facts.len() {
        let assessment = format!("function-{index}.assessment.json");
        let value: Value = read_json(&directory.join(&assessment))?;
        files.push(assessment);
        if !value["manifest_file"].is_null() {
            let expected = format!("function-{index}.manifest.json");
            if value["manifest_file"] != expected {
                return Err("ambiguous manifest filename".into());
            }
            files.extend([
                expected,
                format!("function-{index}.pdg.json"),
                format!("function-{index}.cfg.json"),
            ]);
        }
    }
    files
        .into_iter()
        .map(|file| {
            let bytes = std::fs::read(directory.join(&file)).map_err(|e| e.to_string())?;
            Ok(Artifact {
                file,
                sha256: super::digest(&bytes),
            })
        })
        .collect()
}

pub fn verify_inventory(unit: &Unit) -> Result<(), String> {
    let mut names = std::collections::BTreeSet::new();
    for artifact in &unit.artifacts {
        if !names.insert(&artifact.file) || Path::new(&artifact.file).components().count() != 1 {
            return Err("ambiguous artifact inventory".into());
        }
        let bytes = std::fs::read(unit.directory.join(&artifact.file))
            .map_err(|e| format!("missing captured artifact {}: {e}", artifact.file))?;
        if super::digest(&bytes) != artifact.sha256 {
            return Err(format!(
                "captured artifact digest mismatch: {}",
                artifact.file
            ));
        }
    }
    let actual = artifact_inventory(&unit.directory)?;
    if actual
        .iter()
        .map(|a| &a.file)
        .collect::<std::collections::BTreeSet<_>>()
        != names
    {
        return Err("incomplete captured artifact inventory".into());
    }
    Ok(())
}
