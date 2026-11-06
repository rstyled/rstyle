//! Serialize advisory build reuse and selected-package recapture.
use super::{Args, Capture};
use std::{
    collections::BTreeSet,
    fs::{File, TryLockError},
    path::{Path, PathBuf},
    process::Command,
};

const LOCK_FILE: &str = "advisory-build.lock";

pub(super) fn shared(_capture: &Capture, _args: &Args) -> bool {
    true
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
    lock_file(capture.directory.with_file_name(LOCK_FILE)).map(Some)
}

/// The lane under `parent`, unless a shared build is holding it.
pub(super) fn try_acquire(parent: &Path) -> Result<Option<File>, String> {
    try_lock(open_lock(parent.join(LOCK_FILE))?)
}

pub(super) fn try_lock(lock: File) -> Result<Option<File>, String> {
    match lock.try_lock() {
        Ok(()) => Ok(Some(lock)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(format!("Cannot take lock: {error}")),
    }
}

fn lock_file(path: PathBuf) -> Result<File, String> {
    let lock = open_lock(path)?;
    lock.lock()
        .map(|()| lock)
        .map_err(|error| format!("Cannot lock advisory builds: {error}"))
}

pub(super) fn open_lock(path: PathBuf) -> Result<File, String> {
    File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|error| format!("Cannot open lock {}: {error}", path.display()))
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
