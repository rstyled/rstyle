//! Isolated Cargo compilation and an explicit compiler-unit ledger.

use super::{Args, read_json, write_json};
use crate::{Cli, run};
use rstyle_core::cli_args::flag_value;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub package: String,
    pub manifest_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_context: Option<rstyle_pdg::nominal_binding::PackageContext>,
    pub source: PathBuf,
    pub name: String,
    pub kind: Vec<String>,
    pub test: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Plan {
    pub extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    pub root: PathBuf,
    pub driver: PathBuf,
    pub targets: Vec<Target>,
    pub driver_sha256: String,
    pub orchestrator_sha256: String,
    pub compiler_identity: String,
    pub cargo_identity: String,
    pub primary_units: Vec<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct Unit {
    pub extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
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
    pub expected_unit_graph: Value,
}

pub fn prepare(cli: &Cli, args: &Args) -> Result<Capture, String> {
    let metadata = selected_workspace(cli)?;
    let mut capture = initial_capture(metadata, args.extraction_profile)?;
    capture.plan.targets = targets(cli, args.extraction_profile)?;

    publish_plan(capture)
}

fn initial_capture(
    metadata: crate::workspace::Metadata,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Capture, String> {
    let directory = run_directory(&metadata.target_directory)?;
    let environment = capture_environment()?;
    let plan = plan(metadata.workspace_root, profile, Vec::new(), environment);

    Ok(new_capture(directory, plan))
}

fn publish_plan(capture: Capture) -> Result<Capture, String> {
    write_json(&capture.directory.join("plan.json"), &capture.plan)?;

    Ok(capture)
}

fn selected_workspace(cli: &Cli) -> Result<crate::workspace::Metadata, String> {
    let metadata = crate::workspace::load_with_toolchain(
        cli.manifest_path.as_deref(),
        Some(&run::pinned_toolchain()),
    )?;
    if let Some(package) = metadata.unknown_packages(&cli.packages).first() {
        return Err(format!("unknown package {package}"));
    }
    Ok(metadata)
}

fn new_capture(directory: PathBuf, plan: Plan) -> Capture {
    Capture {
        directory,
        plan,
        units: Vec::new(),
        failures: Vec::new(),
        expected_unit_graph: Value::Null,
    }
}

fn run_directory(target: &Path) -> Result<PathBuf, String> {
    let parent = target.join("rstyle/clones");
    std::fs::create_dir_all(&parent).map_err(|e| e.to_string())?;

    Ok(tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?
        .keep())
}

fn plan(
    root: PathBuf,
    extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    targets: Vec<Target>,
    environment: CaptureEnvironment,
) -> Plan {
    Plan {
        extraction_profile,
        root,
        driver_sha256: environment.binaries.driver.digest,
        orchestrator_sha256: environment.binaries.orchestrator.digest,
        compiler_identity: environment.compiler_identity,
        cargo_identity: environment.cargo_identity,
        driver: environment.binaries.driver.path,
        targets,
        primary_units: Vec::new(),
    }
}

struct CaptureEnvironment {
    binaries: BinaryIdentities,
    compiler_identity: String,
    cargo_identity: String,
}

fn capture_environment() -> Result<CaptureEnvironment, String> {
    Ok(CaptureEnvironment {
        binaries: binary_identities()?,
        compiler_identity: tool_identity("rustc", "-vV")?,
        cargo_identity: tool_identity("cargo", "--version")?,
    })
}

struct ExecutableIdentity {
    path: PathBuf,
    digest: String,
}

struct BinaryIdentities {
    driver: ExecutableIdentity,
    orchestrator: ExecutableIdentity,
}

fn binary_identities() -> Result<BinaryIdentities, String> {
    Ok(BinaryIdentities {
        driver: executable_identity(run::driver_path()?)?,
        orchestrator: executable_identity(std::env::current_exe().map_err(|e| e.to_string())?)?,
    })
}

fn executable_identity(path: PathBuf) -> Result<ExecutableIdentity, String> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let digest = file_digest(&path)?;

    Ok(ExecutableIdentity { path, digest })
}

fn file_digest(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(super::digest(&bytes))
}

fn tool_identity(tool: &str, argument: &str) -> Result<String, String> {
    let output = Command::new(tool)
        .arg(argument)
        .env("RUSTUP_TOOLCHAIN", run::pinned_toolchain())
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!("cannot determine pinned {tool} identity"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn targets(
    cli: &Cli,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Vec<Target>, String> {
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
    selected_targets(&value, cli, profile)
}

fn selected_targets(
    metadata: &Value,
    cli: &Cli,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Vec<Target>, String> {
    let mut result = Vec::new();
    for package in metadata["packages"].as_array().ok_or("missing packages")? {
        let name = package["name"].as_str().ok_or("missing package name")?;
        if !cli.packages.is_empty() && !cli.packages.iter().any(|p| p == name) {
            continue;
        }
        let context = package_context(package, profile)?;
        for target in package["targets"].as_array().ok_or("missing targets")? {
            if !wanted_target(target, cli.all_targets) {
                continue;
            }
            result.push(target_record(package, target, context.clone())?);
        }
    }
    if result.is_empty() {
        return Err("selection contains no Rust compilation targets".into());
    }
    Ok(result)
}

fn package_context(
    package: &Value,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Option<rstyle_pdg::nominal_binding::PackageContext>, String> {
    use rstyle_pdg::{
        concrete_types::digest,
        nominal_binding::{PACKAGE_IDENTITY, PackageContext},
    };
    if !profile.supports("rust.verified-helper-calls") {
        return Ok(None);
    }
    let manifest_path = PathBuf::from(
        package["manifest_path"]
            .as_str()
            .ok_or("Missing package manifest path")?,
    );
    let manifest_bytes = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let context = PackageContext {
        identity: PACKAGE_IDENTITY.into(),
        package_id: package["id"]
            .as_str()
            .ok_or("Missing resolved package ID")?
            .into(),
        name: package["name"]
            .as_str()
            .ok_or("Missing resolved package name")?
            .into(),
        version: package["version"]
            .as_str()
            .ok_or("Missing resolved package version")?
            .into(),
        manifest_path,
        manifest_digest: digest(manifest_bytes.as_bytes()),
        manifest_bytes,
    };
    context.validate()?;

    Ok(Some(context))
}

fn wanted_target(target: &Value, all: bool) -> bool {
    target["kind"].as_array().is_some_and(|kinds| {
        kinds.iter().any(|kind| {
            matches!(
                kind.as_str(),
                Some("lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "bin" | "proc-macro")
            ) || (all && matches!(kind.as_str(), Some("test" | "example" | "bench")))
        })
    })
}

fn target_record(
    package: &Value,
    target: &Value,
    package_context: Option<rstyle_pdg::nominal_binding::PackageContext>,
) -> Result<Target, String> {
    let manifest = PathBuf::from(
        package["manifest_path"]
            .as_str()
            .ok_or("missing manifest")?,
    );
    Ok(Target {
        package_context,
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
    command.args(selection_args(cli)).args(build_args(args));

    Ok(command)
}

fn selection_args(cli: &Cli) -> Vec<std::ffi::OsString> {
    let packages = if cli.packages.is_empty() {
        vec!["--workspace".into()]
    } else {
        cli.packages
            .iter()
            .flat_map(|p| ["-p".into(), p.into()])
            .collect()
    };
    let targets = cli.all_targets.then(|| "--all-targets".into());
    let manifest = cli
        .manifest_path
        .as_ref()
        .map(|p| ["--manifest-path".into(), p.as_os_str().to_owned()]);

    packages
        .into_iter()
        .chain(targets)
        .chain(manifest.into_iter().flatten())
        .collect()
}

fn build_args(args: &Args) -> Vec<std::ffi::OsString> {
    let switches = [
        (args.release, "--release"),
        (args.locked, "--locked"),
        (args.offline, "--offline"),
        (args.no_default_features, "--no-default-features"),
        (args.all_features, "--all-features"),
    ];
    let values = [
        args.features.as_ref().map(|v| ("--features", v.clone())),
        args.target.as_ref().map(|v| ("--target", v.clone())),
        args.jobs.map(|v| ("--jobs", v.to_string())),
    ];

    switches
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, key)| key.into())
        .chain(
            values
                .into_iter()
                .flatten()
                .flat_map(|(key, value)| [key.into(), value.into()]),
        )
        .collect()
}

pub fn compile(capture: &mut Capture, cli: &Cli, args: &Args) -> Result<(), String> {
    let graph = expected_units(capture, cli, args)?;
    configure_capture(capture, graph)?;
    perform_compile(capture, cli, args)
}

fn configure_capture(capture: &mut Capture, graph: Value) -> Result<(), String> {
    capture.plan.primary_units = primary_units(&graph)?;
    capture.expected_unit_graph = graph;
    write_json(&capture.directory.join("plan.json"), &capture.plan)
}

fn primary_units(graph: &Value) -> Result<Vec<Value>, String> {
    let units = graph["units"].as_array().ok_or("missing Cargo units")?;
    let roots = graph["roots"].as_array().ok_or("missing Cargo roots")?;

    roots
        .iter()
        .map(|root| {
            let index = root
                .as_u64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("invalid Cargo root index")?;
            units
                .get(index)
                .cloned()
                .ok_or_else(|| "Cargo root index outside unit inventory".into())
        })
        .collect()
}

fn perform_compile(capture: &mut Capture, cli: &Cli, args: &Args) -> Result<(), String> {
    let output = command(capture, cli, args)?
        .output()
        .map_err(|e| e.to_string())?;
    save_logs(capture, &output)?;

    compilation_result(capture, &output);
    Ok(())
}

fn expected_units(capture: &Capture, cli: &Cli, args: &Args) -> Result<Value, String> {
    let output = command(capture, cli, args)?
        .args(["--unit-graph", "-Z", "unstable-options"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Cargo compilation planning failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let graph: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

    if graph["version"] != 1 || !graph["units"].is_array() {
        return Err("unsupported Cargo unit graph".into());
    }
    write_json(&capture.directory.join("expected-unit-graph.json"), &graph)?;
    Ok(graph)
}

fn compilation_result(capture: &mut Capture, output: &std::process::Output) {
    if let Err(error) = cargo_result(capture, output) {
        capture.failures.push(error);
    }
}

fn cargo_result(capture: &mut Capture, output: &std::process::Output) -> Result<(), String> {
    if !output.status.success() {
        return Err(format!(
            "Cargo compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    reconcile(capture, &output.stdout)
}

fn save_logs(capture: &Capture, output: &std::process::Output) -> Result<(), String> {
    std::fs::write(capture.directory.join("cargo.jsonl"), &output.stdout)
        .map_err(|e| e.to_string())?;
    std::fs::write(capture.directory.join("cargo.stderr"), &output.stderr)
        .map_err(|e| e.to_string())
}

fn reconcile(capture: &mut Capture, cargo: &[u8]) -> Result<(), String> {
    let artifacts: Vec<Value> = cargo
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

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
        if !intended_artifact(capture, artifact, target) {
            continue;
        }
        verify_planned_context(capture, artifact, target)?;
        let test = artifact["profile"]["test"]
            .as_bool()
            .ok_or("missing artifact test context")?;
        let unit_path = unit_ledger_path(capture, target, test)?;
        let unit: Unit = read_json(&unit_path)?;
        if &unit.target != target
            || unit.test != test
            || unit.directory.parent() != Some(capture.directory.as_path())
        {
            return Err("compiler-unit ledger disagrees with Cargo artifact".into());
        }
        capture.units.push(unit);
    }
    check_expected(capture);
    Ok(())
}

fn verify_planned_context(
    capture: &Capture,
    artifact: &Value,
    target: &Target,
) -> Result<(), String> {
    let planned = &capture.plan.primary_units;
    let agrees = planned.iter().any(|unit| {
        unit["pkg_id"] == target.package
            && unit["target"]["src_path"] == target.source.to_string_lossy().as_ref()
            && unit["target"]["name"] == target.name
            && unit["features"] == artifact["features"]
            && [
                "opt_level",
                "debuginfo",
                "debug_assertions",
                "overflow_checks",
            ]
            .iter()
            .all(|key| unit["profile"][key] == artifact["profile"][key])
    });

    if !agrees {
        return Err("Cargo artifact build context disagrees with planned unit inventory".into());
    }
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

fn check_expected(capture: &mut Capture) {
    for target in &capture.plan.targets {
        let expected = capture
            .plan
            .primary_units
            .iter()
            .filter(|u| {
                u["pkg_id"] == target.package
                    && u["target"]["src_path"] == target.source.to_string_lossy().as_ref()
                    && u["target"]["name"] == target.name
            })
            .count();
        let observed = capture
            .units
            .iter()
            .filter(|u| u.target.package == target.package && u.target.source == target.source)
            .count();
        if expected != observed {
            capture.failures.push(format!("compilation-unit inventory mismatch {}:{}: expected {expected}, observed {observed}", target.package, target.name));
        }
    }
    if capture.units.is_empty() {
        capture
            .failures
            .push("selection produced no captured Rust compilation units".into());
    }
}

/// Internal workspace wrapper. Each invocation owns its directory and only
/// publishes the ledger entry after rustc has succeeded and facts are present.
pub fn wrapper(plan_path: &Path, invocation: Vec<String>) -> Result<u8, String> {
    let plan: Plan = read_json(plan_path)?;
    let target = invocation_target(&plan, &invocation)?;
    let command = driver_command(&plan, &invocation, target.as_ref())?;

    match target {
        None => passthrough(command),
        Some(target) => capture_unit(plan_path, target, invocation, command),
    }
}

fn driver_command(
    plan: &Plan,
    invocation: &[String],
    target: Option<&Target>,
) -> Result<Command, String> {
    let arguments = driver_arguments(plan, invocation, target)?;
    let mut command = Command::new(&plan.driver);
    run::clear_rstyle_environment(&mut command);

    command.args(arguments);
    Ok(command)
}

fn driver_arguments(
    plan: &Plan,
    invocation: &[String],
    target: Option<&Target>,
) -> Result<Vec<String>, String> {
    let mut arguments = vec![format!(
        "{}={}",
        rstyle_pdg::extraction_profile::DRIVER_SELECTION_FLAG,
        plan.extraction_profile.token()
    )];
    if plan.extraction_profile.is_data()
        && let Some(target) = target
    {
        let context = selected_unit_context(target, invocation)?;
        arguments.push(format!(
            "{}={}",
            rstyle_pdg::nominal_boundary::CONTEXT_FLAG,
            context.transport()?
        ));
    }

    arguments.extend_from_slice(invocation);
    Ok(arguments)
}

pub fn selected_unit_context(
    target: &Target,
    invocation: &[String],
) -> Result<rstyle_pdg::nominal_boundary::UnitContext, String> {
    Ok(rstyle_pdg::nominal_boundary::UnitContext {
        package: target
            .package_context
            .clone()
            .ok_or("Missing selected Cargo package context")?,
        root_source: target.source.clone(),
        crate_name: flag_value(invocation, "--crate-name")
            .ok_or("Missing observed crate name")?
            .into(),
    })
}

fn passthrough(mut command: Command) -> Result<u8, String> {
    let status = command.status().map_err(|e| e.to_string())?;
    Ok(if status.success() { 0 } else { 2 })
}

fn invocation_target(plan: &Plan, invocation: &[String]) -> Result<Option<Target>, String> {
    let build_script = invocation
        .windows(2)
        .any(|pair| pair[0] == "--crate-name" && pair[1] == "build_script_build")
        || invocation
            .iter()
            .any(|arg| arg == "--crate-name=build_script_build");
    if build_script || invocation.iter().any(|a| a.starts_with("--print")) {
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
        [target] if intended_invocation(plan, target, invocation) => Ok(Some(target.clone())),
        [_] => Ok(None),
        _ => Err("ambiguous target source in capture plan".into()),
    }
}

fn capture_unit(
    plan: &Path,
    target: Target,
    invocation: Vec<String>,
    mut command: Command,
) -> Result<u8, String> {
    if invocation
        .iter()
        .any(|arg| arg.starts_with("incremental=") || arg.starts_with("-Cincremental="))
    {
        return Err("strict capture requires incremental compilation disabled; remove explicit -C incremental flags".into());
    }
    let root = plan.parent().ok_or("missing run directory")?;
    let directory = tempfile::Builder::new()
        .prefix("invocation-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?
        .keep();
    let test = invocation.iter().any(|a| a == "--test");
    let unit = Unit {
        extraction_profile: read_json::<Plan>(plan)?.extraction_profile,
        target,
        test,
        invocation,
        directory,
        artifacts: Vec::new(),
    };
    let status = invoke_unit(&unit, &mut command)?;

    if !status.success() {
        return Ok(2);
    }
    publish_unit(root, unit)?;
    Ok(0)
}

fn invoke_unit(unit: &Unit, command: &mut Command) -> Result<std::process::ExitStatus, String> {
    write_json(&unit.directory.join("started.json"), unit)?;
    command.env("RSTYLE_HIR_DOCUMENT_DIR", &unit.directory);

    command.status().map_err(|e| e.to_string())
}

fn publish_unit(root: &Path, mut unit: Unit) -> Result<(), String> {
    unit.artifacts = artifact_inventory(&unit.directory, unit.extraction_profile)?;
    let key = super::digest(
        &serde_json::to_vec(&json!([unit.target.package, unit.target.source, unit.test]))
            .map_err(|e| e.to_string())?,
    );
    let path = root.join(format!("unit-{key}.json"));

    let file = reserve_ledger(&path)?;
    serde_json::to_writer(file, &unit).map_err(|e| e.to_string())?;
    Ok(())
}

fn reserve_ledger(path: &Path) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("duplicate or missing unit ledger: {e}"))
}

fn artifact_inventory(
    directory: &Path,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Vec<Artifact>, String> {
    let facts: Vec<rstyle_pdg::HirFnFacts> = read_json(&directory.join("facts.json"))?;
    let mut files = vec!["facts.json".to_string()];
    if profile.supports("rust.conditional-source-correspondence") {
        files.push("source-inventory.json".into());
    }
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

    let actual = artifact_inventory(&unit.directory, unit.extraction_profile)?;
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

fn intended_artifact(capture: &Capture, artifact: &Value, target: &Target) -> bool {
    let linked = artifact["filenames"].as_array().is_some_and(|files| {
        files.iter().any(|file| {
            file.as_str()
                .is_some_and(|name| Path::new(name).extension().is_none_or(|ext| ext != "rmeta"))
        })
    });

    let features = artifact["features"].as_array().cloned().unwrap_or_default();
    intended_mode(&capture.plan, target, linked, &features)
}

fn intended_invocation(plan: &Plan, target: &Target, invocation: &[String]) -> bool {
    let linked = flag_value(invocation, "--emit")
        .is_some_and(|emit| emit.split(',').any(|kind| kind == "link"));

    let features = invocation_features(invocation);
    intended_mode(plan, target, linked, &features)
}

fn intended_mode(plan: &Plan, target: &Target, linked: bool, features: &[Value]) -> bool {
    plan.primary_units.iter().any(|unit| {
        unit["pkg_id"] == target.package
            && unit["target"]["src_path"] == target.source.to_string_lossy().as_ref()
            && unit["target"]["name"] == target.name
            && unit["features"]
                .as_array()
                .is_some_and(|expected| expected == features)
            && ((unit["mode"] == "check" && !linked) || (unit["mode"] == "build" && linked))
    })
}

fn invocation_features(invocation: &[String]) -> Vec<Value> {
    let mut features: Vec<_> = invocation
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| {
            let cfg = arg.strip_prefix("--cfg=").or_else(|| {
                (arg == "--cfg")
                    .then(|| invocation.get(index + 1).map(String::as_str))
                    .flatten()
            })?;
            cfg.strip_prefix("feature=\"")?
                .strip_suffix('"')
                .map(|feature| Value::String(feature.into()))
        })
        .collect();
    features.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    features.dedup();

    features
}
