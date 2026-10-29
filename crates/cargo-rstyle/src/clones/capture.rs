//! Isolated Cargo compilation and an explicit compiler-unit ledger.
pub(super) mod associations;
mod build_lane;
mod dependencies;
pub(super) mod ledger;
mod owner_reuse;
mod saved;
pub(in crate::clones) use saved::load_saved;

use super::{Args, read_json, write_json};
use crate::{Cli, run};
use rstyle_core::cli_args::flag_value;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    #[serde(default)]
    pub compiler_advisory_only: bool,
    #[serde(default)]
    pub compiler_advisory: bool,
    #[serde(default)]
    pub compiler_patterns: bool,
    #[serde(default)]
    pub local_source_facts: bool,
    #[serde(default)]
    pub local_mutation_facts: bool,
    pub ledger_schema: String,
    pub extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    pub root: PathBuf,
    pub driver: PathBuf,
    pub targets: Vec<Target>,
    pub compiler_identity: String,
    pub cargo_identity: String,
    pub primary_units: Vec<Value>,
    #[serde(default)]
    pub dependency_units: Vec<Value>,
}

impl Plan {
    fn with_advisory(self, args: &Args) -> Self {
        let mutations = args.mutation_rules || args.mutation_effects || args.effect_density;
        Self {
            compiler_advisory_only: args.compiler_advisory_only,
            compiler_advisory: args.compiler_inputs_requested(),
            compiler_patterns: args.compiler_patterns,
            local_source_facts: args.compiler_advisory_only
                && (args.source_rules || args.guard_clauses || mutations),
            local_mutation_facts: args.compiler_advisory_only && mutations,
            ..self
        }
    }

    fn planned_units(&self) -> impl Iterator<Item = &Value> {
        self.primary_units.iter().chain(&self.dependency_units)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Unit {
    #[serde(default)]
    pub compiler_advisory_only: bool,
    pub extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile,
    pub target: Target,
    pub test: bool,
    pub invocation: Vec<String>,
    pub directory: PathBuf,
    pub artifacts: Vec<Artifact>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub file: String,
    pub checksum: String,
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
    capture.plan = capture.plan.with_advisory(args);

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
        compiler_advisory_only: false,
        compiler_advisory: false,
        compiler_patterns: false,
        local_source_facts: false,
        local_mutation_facts: false,
        ledger_schema: ledger::SCHEMA.into(),
        extraction_profile,
        root,
        compiler_identity: environment.compiler_identity,
        cargo_identity: environment.cargo_identity,
        driver: environment.driver,
        targets,
        primary_units: Vec::new(),
        dependency_units: Vec::new(),
    }
}

struct CaptureEnvironment {
    driver: PathBuf,
    compiler_identity: String,
    cargo_identity: String,
}

fn capture_environment() -> Result<CaptureEnvironment, String> {
    Ok(CaptureEnvironment {
        driver: run::driver_path()?
            .canonicalize()
            .map_err(|e| e.to_string())?,
        compiler_identity: tool_identity("rustc", "-vV")?,
        cargo_identity: tool_identity("cargo", "--version")?,
    })
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
        nominal_binding::{PACKAGE_IDENTITY, PackageContext},
        source_identity::checksum,
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
        manifest_digest: checksum(manifest_bytes.as_bytes()),
        manifest_bytes: manifest_bytes.into(),
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

fn cargo_command() -> Command {
    let mut command = Command::new("cargo");
    run::clear_rstyle_environment(&mut command);
    command
        .env("RUSTUP_TOOLCHAIN", run::pinned_toolchain())
        .env("RUSTC_WORKSPACE_WRAPPER", "")
        .env("RUSTC_WRAPPER", "")
        .env_remove("RUSTC");
    command
}

fn command(capture: &Capture, cli: &Cli, args: &Args) -> Result<Command, String> {
    let mut command = cargo_command();
    command
        .args(["check", "--message-format=json"])
        .env("CARGO_TARGET_DIR", build_lane::directory(capture, args))
        .env("CARGO_INCREMENTAL", "0")
        .env("RSTYLE_CAPTURE_PLAN", capture.directory.join("plan.json"));
    let wrapper = if args.capture_dependencies.is_empty() {
        "RUSTC_WORKSPACE_WRAPPER"
    } else {
        "RUSTC_WRAPPER"
    };
    command
        .env(wrapper, std::env::current_exe().map_err(|e| e.to_string())?)
        .args(selection_args(cli))
        .args(build_args(args));

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
    let _lane = build_lane::acquire(capture, args)?;
    let previous = owner_reuse::take(capture, args)?;
    compile_in_lane(capture, cli, args, previous)
}

fn compile_in_lane(
    capture: &mut Capture,
    cli: &Cli,
    args: &Args,
    previous: Option<owner_reuse::Index>,
) -> Result<(), String> {
    configure_capture(capture, cli, args)?;
    owner_reuse::compile(capture, cli, args, previous)
}

fn configure_capture(capture: &mut Capture, cli: &Cli, args: &Args) -> Result<(), String> {
    let graph = expected_units(capture, cli, args)?;
    let selected = dependencies::from_cargo(&graph, cli, args)?;
    capture.plan = configured_plan(&capture.plan, &graph, selected)?;
    capture.expected_unit_graph = graph;

    write_json(&capture.directory.join("plan.json"), &capture.plan)
}

fn configured_plan(
    plan: &Plan,
    graph: &Value,
    selected: Vec<dependencies::Selected>,
) -> Result<Plan, String> {
    let (units, targets): (Vec<_>, Vec<_>) = selected
        .into_iter()
        .map(|dependency| (dependency.unit, dependency.target))
        .unzip();

    Ok(Plan {
        primary_units: primary_units(graph)?,
        dependency_units: units,
        targets: plan.targets.iter().cloned().chain(targets).collect(),
        ..plan.clone()
    })
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
    let output = compile_output(capture, cli, args)?;
    compilation_result(capture, &output);
    Ok(())
}

fn compile_output(
    capture: &Capture,
    cli: &Cli,
    args: &Args,
) -> Result<std::process::Output, String> {
    let observation =
        rstyle_pdg::diagnostics::Stage::new("capture.compile", "cargo check", None).enter();
    observation.finish(run_compile(capture, cli, args))
}

fn run_compile(capture: &Capture, cli: &Cli, args: &Args) -> Result<std::process::Output, String> {
    let output = crate::diagnostic_process::output(&mut command(capture, cli, args)?)
        .map_err(|e| e.to_string())?;
    save_logs(capture, &output)?;
    Ok(output)
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
    let artifacts = cargo_artifacts(cargo)?;
    for artifact in &artifacts {
        let Some(target) = selected_artifact(capture, artifact) else {
            continue;
        };
        verify_planned_context(capture, artifact, target)?;
        let test = artifact["profile"]["test"]
            .as_bool()
            .ok_or("missing artifact test context")?;
        let unit_path = ledger::path(&capture.plan, &capture.directory, target, test)?;
        let unit: Unit = read_json(&unit_path)?;
        if &unit.target != target
            || unit.compiler_advisory_only != capture.plan.compiler_advisory_only
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

fn cargo_artifacts(cargo: &[u8]) -> Result<Vec<Value>, String> {
    let records: Vec<Value> = cargo
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    Ok(records
        .into_iter()
        .filter(|a| a["reason"] == "compiler-artifact")
        .collect())
}

fn selected_artifact<'a>(capture: &'a Capture, artifact: &Value) -> Option<&'a Target> {
    capture
        .plan
        .targets
        .iter()
        .find(|t| matches_artifact(artifact, t))
        .filter(|target| intended_artifact(capture, artifact, target))
}

fn verify_planned_context(
    capture: &Capture,
    artifact: &Value,
    target: &Target,
) -> Result<(), String> {
    let agrees = capture.plan.planned_units().any(|unit| {
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

pub(in crate::clones) fn check_expected(capture: &mut Capture) {
    for target in &capture.plan.targets {
        let expected = capture
            .plan
            .planned_units()
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

    if (plan.compiler_advisory || plan.compiler_advisory_only) && target.is_some() {
        let flags = [
            plan.compiler_advisory
                .then_some((rstyle_core::advisory::CAPTURE_ENV, "1")),
            plan.compiler_advisory_only
                .then_some(("RSTYLE_COMPILER_ADVISORY_ONLY", "1")),
            plan.compiler_patterns
                .then_some((rstyle_core::advisory::patterns::CAPTURE_ENV, "1")),
            plan.local_source_facts
                .then_some(("RSTYLE_LOCAL_SOURCE_FACTS", "1")),
            plan.local_mutation_facts
                .then_some(("RSTYLE_LOCAL_MUTATION_FACTS", "1")),
        ];
        command
            .envs(flags.into_iter().flatten())
            .env(rstyle_core::advisory::SOURCE_ROOT_ENV, &plan.root);
    }
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
    let selected_plan: Plan = read_json(plan)?;
    let directory = tempfile::Builder::new()
        .prefix("invocation-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?
        .keep();
    let test = invocation.iter().any(|a| a == "--test");
    let unit = Unit {
        compiler_advisory_only: selected_plan.compiler_advisory_only,
        extraction_profile: selected_plan.extraction_profile,
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
    publish_unit(&selected_plan, root, unit)?;
    Ok(0)
}

fn invoke_unit(unit: &Unit, command: &mut Command) -> Result<std::process::ExitStatus, String> {
    write_json(&unit.directory.join("started.json"), unit)?;
    command.env("RSTYLE_HIR_DOCUMENT_DIR", &unit.directory);

    command.status().map_err(|e| e.to_string())
}

fn publish_unit(plan: &Plan, root: &Path, mut unit: Unit) -> Result<(), String> {
    unit.artifacts = artifact_inventory(&unit.directory, unit.extraction_profile)?;
    publish_ledger(plan, root, &unit)
}

fn publish_ledger(plan: &Plan, root: &Path, unit: &Unit) -> Result<(), String> {
    let path = ledger::path(plan, root, &unit.target, unit.test)?;

    let file = reserve_ledger(&path)?;
    serde_json::to_writer(file, unit).map_err(|e| e.to_string())?;
    Ok(())
}

fn reserve_ledger(path: &Path) -> Result<std::fs::File, String> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("duplicate or missing unit ledger: {e}"))
}

fn artifact_files(profile: rstyle_pdg::extraction_profile::ExtractionProfile) -> Vec<String> {
    let mut files = vec![
        "unit.evidence.json".into(),
        "unit.evidence.manifest.json".into(),
    ];
    if profile.supports("rust.conditional-source-correspondence") {
        files.push("source-inventory.json".into());
    }
    files
}

fn artifact_inventory(
    directory: &Path,
    profile: rstyle_pdg::extraction_profile::ExtractionProfile,
) -> Result<Vec<Artifact>, String> {
    artifact_files(profile)
        .into_iter()
        .map(|file| read_artifact(directory, file))
        .collect()
}

fn read_artifact(directory: &Path, file: String) -> Result<Artifact, String> {
    if file == "unit.evidence.json" {
        let bytes = std::fs::read(directory.join("unit.evidence.manifest.json"))
            .map_err(|error| error.to_string())?;
        let manifest: rstyle_pdg::unit_store::BundleManifest =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        return Ok(Artifact {
            file,
            checksum: manifest.checksum,
        });
    }
    let bytes = std::fs::read(directory.join(&file)).map_err(|error| error.to_string())?;
    Ok(Artifact {
        file,
        checksum: rstyle_pdg::unit_store::transport_checksum(&bytes),
    })
}

pub struct Snapshot {
    pub bundle: rstyle_pdg::unit_store::VerifiedBundle,
    files: std::collections::BTreeMap<String, Vec<u8>>,
}

impl Snapshot {
    pub fn bytes(&self, file: &str) -> Result<&[u8], String> {
        if file == "unit.evidence.json" {
            return Ok(self.bundle.bytes());
        }
        self.files
            .get(file)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("Missing captured artifact {file}"))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self, file: &str) -> Result<T, String> {
        serde_json::from_slice(self.bytes(file)?).map_err(|error| format!("{file}: {error}"))
    }
}

pub fn load_inventory(unit: &Unit) -> Result<Snapshot, String> {
    let mut files = std::collections::BTreeMap::new();
    for artifact in &unit.artifacts {
        if files.contains_key(&artifact.file) || Path::new(&artifact.file).components().count() != 1
        {
            return Err("ambiguous artifact inventory".into());
        }
        let bytes = std::fs::read(unit.directory.join(&artifact.file))
            .map_err(|error| format!("missing captured artifact {}: {error}", artifact.file))?;
        if artifact.file != "unit.evidence.json"
            && rstyle_pdg::unit_store::transport_checksum(&bytes) != artifact.checksum
        {
            return Err(format!(
                "captured artifact checksum mismatch: {}",
                artifact.file
            ));
        }
        files.insert(artifact.file.clone(), bytes);
    }

    let actual: std::collections::BTreeSet<_> = files.keys().cloned().collect();
    if actual
        != artifact_files(unit.extraction_profile)
            .into_iter()
            .collect()
    {
        return Err("incomplete captured artifact inventory".into());
    }
    sealed_snapshot(unit, files)
}

fn sealed_snapshot(
    unit: &Unit,
    mut files: std::collections::BTreeMap<String, Vec<u8>>,
) -> Result<Snapshot, String> {
    let manifest: rstyle_pdg::unit_store::BundleManifest =
        serde_json::from_slice(&files["unit.evidence.manifest.json"])
            .map_err(|error| error.to_string())?;
    let artifact = unit
        .artifacts
        .iter()
        .find(|artifact| artifact.file == "unit.evidence.json")
        .ok_or("Missing bundle inventory entry")?;

    if manifest.checksum != artifact.checksum {
        return Err("Bundle inventory disagrees with publication seal".into());
    }
    let bundle = manifest.verify_owned(files.remove("unit.evidence.json").unwrap())?;
    Ok(Snapshot { bundle, files })
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
    plan.planned_units().any(|unit| {
        unit["pkg_id"] == target.package
            && unit["target"]["src_path"] == target.source.to_string_lossy().as_ref()
            && unit["target"]["name"] == target.name
            && unit["features"]
                .as_array()
                .is_some_and(|expected| expected == features)
            && ((unit["mode"] == "check" && !linked) || (unit["mode"] == "build" && linked))
    })
}

pub(in crate::clones) fn invocation_features(invocation: &[String]) -> Vec<Value> {
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

#[cfg(test)]
mod digest_tests {

    #[test]
    fn content_digest_matches_standard_vectors_and_recaptures_edits() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("content");
        std::fs::write(&file, b"").unwrap();
        assert_eq!(
            super::super::digest(&std::fs::read(&file).unwrap()),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(
            super::super::digest(&std::fs::read(&file).unwrap()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        std::fs::write(&file, b"abd").unwrap();
        assert_eq!(
            super::super::digest(&std::fs::read(&file).unwrap()),
            super::super::digest(b"abd")
        );
        std::fs::write(&file, vec![b'a'; 1_000_000]).unwrap();
        assert_eq!(
            super::super::digest(&std::fs::read(&file).unwrap()),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn bundle_inventory_reuses_publication_checksum_without_reading_bundle() {
        let directory = tempfile::tempdir().unwrap();
        let store = rstyle_pdg::unit_store::UnitStore::capture(Vec::new()).unwrap();
        let manifest = store.write_sealed_bundle(Vec::new()).unwrap();
        super::write_json(
            &directory.path().join("unit.evidence.manifest.json"),
            &manifest,
        )
        .unwrap();
        // No bundle file exists: publication inventory consumes the streaming seal.
        let artifacts = artifact_inventory(
            directory.path(),
            rstyle_pdg::extraction_profile::ExtractionProfile::R0,
        )
        .unwrap();
        let bundle = artifacts
            .iter()
            .find(|artifact| artifact.file == "unit.evidence.json")
            .unwrap();
        assert_eq!(bundle.checksum, manifest.checksum);
    }

    fn snapshot_unit(directory: &Path) -> Unit {
        Unit {
            compiler_advisory_only: false,
            extraction_profile: rstyle_pdg::extraction_profile::ExtractionProfile::R0,
            target: Target {
                package: "snapshot".into(),
                manifest_dir: directory.into(),
                package_context: None,
                source: directory.join("lib.rs"),
                name: "snapshot".into(),
                kind: vec!["lib".into()],
                test: false,
            },
            test: false,
            invocation: Vec::new(),
            directory: directory.into(),
            artifacts: artifact_inventory(
                directory,
                rstyle_pdg::extraction_profile::ExtractionProfile::R0,
            )
            .unwrap(),
        }
    }

    #[test]
    fn verified_snapshot_survives_file_edits_without_rereading() {
        let directory = tempfile::tempdir().unwrap();
        let store = rstyle_pdg::unit_store::UnitStore::capture(Vec::new()).unwrap();
        let mut bytes = Vec::new();
        let manifest = store.write_sealed_bundle(&mut bytes).unwrap();
        std::fs::write(directory.path().join("unit.evidence.json"), &bytes).unwrap();
        super::write_json(
            &directory.path().join("unit.evidence.manifest.json"),
            &manifest,
        )
        .unwrap();
        let mut unit = snapshot_unit(directory.path());
        let snapshot = load_inventory(&unit).unwrap();
        let original = unit.artifacts[0].checksum.clone();
        unit.artifacts[0].checksum = rstyle_pdg::unit_store::transport_checksum(b"different seal");
        assert!(load_inventory(&unit).is_err());
        unit.artifacts[0].checksum = original;
        let manifest_path = directory.path().join("unit.evidence.manifest.json");
        let original_manifest = std::fs::read(&manifest_path).unwrap();
        let mut changed_manifest = original_manifest.clone();
        changed_manifest[0] ^= 1;
        std::fs::write(&manifest_path, changed_manifest).unwrap();
        assert!(load_inventory(&unit).is_err());
        std::fs::write(&manifest_path, original_manifest).unwrap();
        std::fs::write(
            directory.path().join("unit.evidence.json"),
            b"corrupt replacement",
        )
        .unwrap();
        rstyle_pdg::unit_store::UnitStore::from_verified_current_run(&snapshot.bundle, |_| Ok(()))
            .unwrap();
        assert!(load_inventory(&unit).is_err());
    }
}
