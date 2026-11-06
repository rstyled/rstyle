//! Bound the saved runs a workspace keeps, since a fresh capture holds gigabytes.
use super::{build_lane, owner_reuse};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
    time::SystemTime,
};

const KEPT_RUNS: usize = 5;
pub(in crate::compiler) const LIMIT_VARIABLE: &str = "RSTYLE_KEEP_RUNS";
const RUN_PREFIX: &str = "run-";
const REMOVING_PREFIX: &str = "pruning-";
const LOCK_FILE: &str = "run.lock";

/// How many saved runs to keep, from `RSTYLE_KEEP_RUNS`; `None` keeps them all.
pub(in crate::compiler) fn limit() -> Result<Option<usize>, String> {
    let value = std::env::var_os(LIMIT_VARIABLE);
    parsed_limit(
        value
            .map(|value| value.to_string_lossy().into_owned())
            .as_deref(),
    )
}

fn parsed_limit(value: Option<&str>) -> Result<Option<usize>, String> {
    match value {
        None => Ok(Some(KEPT_RUNS)),
        Some("all") => Ok(None),
        Some(text) => match text.parse() {
            Ok(count) if count > 0 => Ok(Some(count)),
            _ => Err(format!(
                "{LIMIT_VARIABLE} must be a positive count or `all`, not `{text}`"
            )),
        },
    }
}

pub(super) fn new_run(parent: &Path) -> Result<PathBuf, String> {
    let directory = tempfile::Builder::new()
        .prefix(RUN_PREFIX)
        .tempdir_in(parent)
        .map_err(|error| error.to_string())?
        .keep();

    hold(&directory)?;
    Ok(directory)
}

/// Keep a run from being removed for the rest of this process, the life of one command.
pub(in crate::compiler) fn hold(run: &Path) -> Result<(), String> {
    let lock = build_lane::open_lock(run.join(LOCK_FILE))?;
    lock.lock_shared()
        .map_err(|error| format!("Cannot hold saved run {}: {error}", run.display()))?;

    // The descriptor closing is the release, so a crashed process holds nothing.
    std::mem::forget(lock);
    Ok(())
}

/// A cleanup failure leaves the capture itself sound, so it is reported rather than fatal.
pub(super) fn prune(parent: &Path, limit: Option<usize>) {
    let Some(keep) = limit else {
        return;
    };
    if let Err(error) = prune_to(parent, keep) {
        eprintln!("rstyle: cannot remove old saved runs: {error}");
    }
}

// rstyle-allow(side_effect_density): IO boundary: lists a workspace's runs and removes the expired ones
fn prune_to(parent: &Path, keep: usize) -> Result<(), String> {
    // A shared build has consumed the owner index, so what it will read is unknown here.
    let Some(_lane) = build_lane::try_acquire(parent)? else {
        return Ok(());
    };
    let interrupted = children(parent, REMOVING_PREFIX)?;
    let runs = children(parent, RUN_PREFIX)?
        .into_iter()
        .map(|run| (last_used(&run), run))
        .collect();
    let expired = expired(runs, keep, &owner_reuse::indexed_runs(parent));

    let failures: Vec<_> = interrupted
        .iter()
        .map(|directory| remove(directory))
        .chain(expired.iter().map(|run| retire(run)))
        .filter_map(Result::err)
        .collect();
    match failures.is_empty() {
        true => Ok(()),
        false => Err(failures.join("; ")),
    }
}

// rstyle-allow(side_effect_density): IO boundary: lists one directory
fn children(parent: &Path, prefix: &str) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();

    for entry in std::fs::read_dir(parent).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let named = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(prefix));
        if named
            && entry
                .file_type()
                .map_err(|error| error.to_string())?
                .is_dir()
        {
            found.push(entry.path());
        }
    }
    Ok(found)
}

// A replay rewrites the report in place, which leaves the directory's own time alone.
fn last_used(run: &Path) -> SystemTime {
    [run.join("report.json"), run.to_path_buf()]
        .iter()
        .filter_map(|path| path.metadata().and_then(|found| found.modified()).ok())
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn expired(
    mut runs: Vec<(SystemTime, PathBuf)>,
    keep: usize,
    indexed: &BTreeSet<OsString>,
) -> Vec<PathBuf> {
    runs.sort_unstable();
    runs.truncate(runs.len().saturating_sub(keep));

    runs.into_iter()
        .map(|(_, run)| run)
        .filter(|run| run.file_name().is_none_or(|name| !indexed.contains(name)))
        .collect()
}

// rstyle-allow(side_effect_density): IO boundary: claims a run, then renames and removes it
fn retire(run: &Path) -> Result<(), String> {
    let lock = build_lane::open_lock(run.join(LOCK_FILE))?;
    // A run another command holds is considered again by a later capture.
    let Some(_claim) = build_lane::try_lock(lock)? else {
        return Ok(());
    };
    let mut removing = OsString::from(REMOVING_PREFIX);
    removing.push(run.file_name().ok_or("Saved run has no name")?);
    let removing = run.with_file_name(removing);

    // Renamed first, so a removal cut short never looks like a run.
    std::fs::rename(run, &removing).map_err(|error| error.to_string())?;
    remove(&removing)
}

fn remove(directory: &Path) -> Result<(), String> {
    std::fs::remove_dir_all(directory).map_err(|error| format!("{}: {error}", directory.display()))
}

#[cfg(test)]
mod tests;
