//! Reuse dependency builds while collecting every requested advisory owner afresh.
use super::{Args, Capture};
use std::{collections::BTreeSet, fs::File, path::PathBuf, process::Command};

fn shared(capture: &Capture, args: &Args) -> bool {
    capture.plan.compiler_advisory_only && args.capture_dependencies.is_empty()
}

pub(super) fn directory(capture: &Capture, args: &Args) -> PathBuf {
    if shared(capture, args) {
        // Runs are immediate children of the workspace's private rstyle/clones directory.
        capture.directory.with_file_name("advisory-build")
    } else {
        capture.directory.join("build")
    }
}

pub(super) fn acquire(capture: &Capture, args: &Args) -> Result<Option<File>, String> {
    if !shared(capture, args) {
        return Ok(None);
    }
    lock_file(capture.directory.with_file_name("advisory-build.lock")).map(Some)
}

fn lock_file(path: PathBuf) -> Result<File, String> {
    let lock = open_lock(path)?;
    lock.lock()
        .map(|()| lock)
        .map_err(|error| format!("Cannot lock advisory builds: {error}"))
}

fn open_lock(path: PathBuf) -> Result<File, String> {
    File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|error| format!("Cannot open advisory build lock: {error}"))
}

pub(super) fn refresh_selected(capture: &Capture, args: &Args) -> Result<(), String> {
    if !shared(capture, args) {
        return Ok(());
    }
    let mut command = clean_command(capture, args);
    perform_clean(capture, &mut command, "cargo-clean.stderr")?;

    let platforms: BTreeSet<_> = capture
        .plan
        .primary_units
        .iter()
        .filter_map(|unit| unit["platform"].as_str())
        .collect();
    for (index, platform) in platforms.into_iter().enumerate() {
        let mut command = clean_command(capture, args);
        command.arg("--target").arg(platform);
        perform_clean(
            capture,
            &mut command,
            &format!("cargo-clean-target-{index}.stderr"),
        )?;
    }
    Ok(())
}

fn perform_clean(capture: &Capture, command: &mut Command, log: &str) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    std::fs::write(capture.directory.join(log), &output.stderr)
        .map_err(|error| error.to_string())?;
    clean_result(&output)
}

fn clean_result(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Cannot refresh selected advisory packages: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn clean_command(capture: &Capture, args: &Args) -> Command {
    let mut command = super::cargo_command();
    command
        .args(["clean", "--manifest-path"])
        .arg(capture.plan.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(directory(capture, args));
    for (enabled, flag) in [(args.locked, "--locked"), (args.offline, "--offline")] {
        if enabled {
            command.arg(flag);
        }
    }

    // Clean all profiles of selected packages: Cargo must invoke the wrapper again
    // even when source bytes, features and compiler options have not changed.
    let packages: BTreeSet<_> = capture.plan.targets.iter().map(|t| &t.package).collect();
    for package in packages {
        command.arg("--package").arg(package);
    }
    command
}
