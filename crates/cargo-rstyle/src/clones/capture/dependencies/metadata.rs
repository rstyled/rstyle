//! Full metadata is needed only for explicitly requested dependency packages.
use crate::{Cli, clones::Args, run};
use serde_json::Value;
use std::{ffi::OsString, process::Command};

// rstyle-allow(side_effect_density): Cargo metadata boundary resolves requested dependencies before applying pure unit selection.
pub(in crate::clones::capture) fn select(
    graph: &Value,
    cli: &Cli,
    args: &Args,
) -> Result<Vec<super::Selected>, String> {
    if args.capture_dependencies.is_empty() {
        return Ok(Vec::new());
    }

    let mut command = Command::new("cargo");
    command
        .args(["metadata", "--format-version", "1"])
        .env("RUSTUP_TOOLCHAIN", run::pinned_toolchain())
        .args(options(cli, args));

    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "Dependency metadata resolution failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    if metadata.get("version").and_then(Value::as_u64) != Some(1) {
        return Err("Unsupported resolved dependency metadata version".into());
    }

    super::select(
        &metadata,
        graph,
        super::Selection {
            requests: &args.capture_dependencies,
            profile: args.extraction_profile,
            target: args.target.as_deref(),
        },
    )
}

fn options(cli: &Cli, args: &Args) -> Vec<OsString> {
    let switches = [
        (args.offline, "--offline"),
        (args.locked, "--locked"),
        (args.all_features, "--all-features"),
        (args.no_default_features, "--no-default-features"),
    ];
    let values = [
        cli.manifest_path
            .as_ref()
            .map(|path| ("--manifest-path", path.as_os_str().to_owned())),
        args.features
            .as_ref()
            .map(|features| ("--features", features.into())),
        args.target
            .as_ref()
            .map(|target| ("--filter-platform", target.into())),
    ];
    switches
        .into_iter()
        .filter(|(enabled, _)| *enabled)
        .map(|(_, key)| key.into())
        .chain(
            values
                .into_iter()
                .flatten()
                .flat_map(|(key, value)| [key.into(), value]),
        )
        .collect()
}
