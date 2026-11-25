//! Completed raw observations, reused only with the current Cargo freshness decision.
use super::{
    Args, Capture, Plan, Target, Unit, build_lane, read_json, relocation::Relocation, retention,
    stop, write_json,
};
use crate::Cli;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
    process::Output,
};

const INDEX_FILE: &str = "advisory-owners.json";
/// 3: the runs directory it was written in, so a build directory restored into a checkout at
/// another path still serves the owners it captured. 2: tools named by their bytes.
const SCHEMA: u32 = 3;

#[derive(Serialize, Deserialize)]
pub(super) struct Index {
    schema: u32,
    /// The runs directory the index was written in, which its units' directories sit under.
    #[serde(default)]
    runs: PathBuf,
    plan: Plan,
    tools: Vec<Tool>,
    entries: Vec<Entry>,
}

/// A program the capture ran, by its bytes: the same build reinstalled, as a fresh worker does,
/// still serves the owners it captured, and a rebuilt one doesn't.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
struct Tool {
    checksum: String,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    artifact: Value,
    unit: Unit,
}

fn index_path(capture: &Capture) -> PathBuf {
    capture.directory.with_file_name(INDEX_FILE)
}

// rstyle-allow(side_effect_density): IO boundary: reads the owner index
/// The runs the next shared build may reuse units from.
pub(super) fn indexed_runs(parent: &Path) -> BTreeSet<OsString> {
    let Ok(index) = read_json::<Index>(&parent.join(INDEX_FILE)) else {
        return BTreeSet::new();
    };

    index
        .entries
        .iter()
        .filter_map(|entry| entry.unit.directory.parent()?.file_name())
        .map(OsString::from)
        .collect()
}

pub(super) fn take(capture: &Capture) -> Result<Option<Index>, String> {
    let path = index_path(capture);
    let previous = read_json(&path).ok();
    Ok(if remove_index(&path)? { previous } else { None })
}

fn remove_index(path: &Path) -> Result<bool, String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("Cannot consume advisory owner index: {error}")),
    }
}

pub(super) fn compile(
    capture: &mut Capture,
    cli: &Cli,
    args: &Args,
    previous: Option<Index>,
) -> Result<(), String> {
    Compile { cli, args }.run(capture, previous)
}

struct Compile<'a> {
    cli: &'a Cli,
    args: &'a Args,
}

struct Prepared {
    tools: Vec<Tool>,
    previous: Option<Index>,
}

impl Compile<'_> {
    fn run(&self, capture: &mut Capture, previous: Option<Index>) -> Result<(), String> {
        let prepared = self.prepare(capture, previous)?;
        let output = self.watched(capture, &prepared)?;

        super::compilation_result(capture, &output);
        publish_completed(capture, &output, prepared.tools)
    }

    // rstyle-allow(side_effect_density): signal boundary: compiles while noting SIGTERM, and keeps what a stop left
    /// Compiles as `cached` does, noting SIGTERM meanwhile rather than dying of it. A compile asked to stop keeps the
    /// owners it finished and fails.
    fn watched(&self, capture: &mut Capture, prepared: &Prepared) -> Result<Output, String> {
        let watch = stop::watch();
        let output = self.cached(capture, prepared.previous.as_ref());
        drop(watch);
        match stop::asked() {
            true => keep_stopped(capture, &output?.stdout, prepared),
            false => output,
        }
    }

    fn prepare(&self, capture: &Capture, previous: Option<Index>) -> Result<Prepared, String> {
        let tools = tools(&capture.plan)?;
        let previous = previous.and_then(|index| reusable(index, capture, &tools));
        let stale = capture
            .plan
            .stale_packages(previous.as_ref().map(|index| &index.plan));
        build_lane::refresh(capture, self.args, &stale)?;
        Ok(Prepared { tools, previous })
    }

    fn cached(&self, capture: &mut Capture, previous: Option<&Index>) -> Result<Output, String> {
        let output = super::compile_output(capture, self.cli, self.args)?;
        if !output.status.success() || restore(capture, &output.stdout, previous)? {
            return Ok(output);
        }

        self.retry(capture)
    }

    fn retry(&self, capture: &mut Capture) -> Result<Output, String> {
        restart(capture)?;
        build_lane::refresh(capture, self.args, &capture.plan.packages())?;
        super::compile_output(capture, self.cli, self.args)
    }
}

fn publish_completed(capture: &Capture, output: &Output, tools: Vec<Tool>) -> Result<(), String> {
    match output.status.success() && capture.failures.is_empty() {
        true => publish(capture, &output.stdout, tools),
        false => Ok(()),
    }
}

fn tools(plan: &Plan) -> Result<Vec<Tool>, String> {
    [
        std::env::current_exe().map_err(|e| e.to_string())?,
        plan.driver.clone(),
    ]
    .into_iter()
    .map(tool)
    .collect()
}

fn tool(path: PathBuf) -> Result<Tool, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    Ok(Tool {
        checksum: rstyle_capture::checksum(&bytes),
    })
}

/// The index as this checkout would have written it: the same when it was written here, and
/// with each path it records moved into this checkout when it was written in another.
fn relocated(index: Index, capture: &Capture) -> Result<Index, String> {
    let relocation = Relocation::new(&index.runs, &index.plan, capture)?;
    let entries = index
        .entries
        .iter()
        .map(|entry| {
            Ok(Entry {
                artifact: relocation.value(&entry.artifact),
                unit: relocation.unit(&entry.unit)?,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Index {
        schema: index.schema,
        runs: relocation.directory(&index.runs),
        plan: relocation.plan(&index.plan)?,
        tools: index.tools,
        entries,
    })
}

/// The previous index, moved into this checkout, when it can serve this capture's units: written
/// in this schema by the same tools for a plan captured the same way, with each unit it names
/// still stored. A plan with other units, as a manifest edit leaves, still serves those it shares.
fn reusable(index: Index, capture: &Capture, tools: &[Tool]) -> Option<Index> {
    if index.schema != SCHEMA {
        return None;
    }
    let index = relocated(index, capture).ok()?;
    let serves = index.plan.same_capture(&capture.plan)
        && index.tools == tools
        && !index.entries.is_empty()
        && index.entries.iter().all(|entry| stored(&entry.unit));
    serves.then_some(index)
}

// rstyle-allow(side_effect_density): IO boundary: checks a unit's files are still there
fn stored(unit: &Unit) -> bool {
    super::stored_files(unit)
        .is_ok_and(|files| files.iter().all(|file| unit.directory.join(file).is_file()))
}

fn same_artifact(previous: &Value, current: &Value) -> bool {
    match (previous.as_object(), current.as_object()) {
        (Some(previous), Some(current)) => {
            previous.len() == current.len()
                && previous
                    .iter()
                    .all(|(key, value)| key == "fresh" || current.get(key) == Some(value))
        }
        _ => false,
    }
}

fn restore(capture: &Capture, cargo: &[u8], previous: Option<&Index>) -> Result<bool, String> {
    let artifacts = super::cargo_artifacts(cargo)?;
    let mut units = Vec::new();

    for artifact in &artifacts {
        let Some(target) = super::selected_artifact(capture, artifact) else {
            continue;
        };
        if artifact["fresh"] != true {
            continue;
        }
        let Some(entry) = previous_entry(previous, target, artifact) else {
            return Ok(false);
        };
        units.push(&entry.unit);
    }

    for unit in units {
        materialize(capture, unit)?;
    }
    Ok(true)
}

/// The previous capture's owner of the unit Cargo reports as `artifact`, if it captured that unit.
fn previous_entry<'a>(
    previous: Option<&'a Index>,
    target: &Target,
    artifact: &Value,
) -> Option<&'a Entry> {
    previous?
        .entries
        .iter()
        .find(|entry| entry.unit.target == *target && same_artifact(&entry.artifact, artifact))
}

fn materialize(capture: &Capture, original: &Unit) -> Result<Unit, String> {
    let unit = reused_unit(capture, original)?;
    copy_snapshot(original, &unit)?;
    super::publish_ledger(&capture.plan, &capture.directory, &unit)?;
    Ok(unit)
}

fn reused_unit(capture: &Capture, original: &Unit) -> Result<Unit, String> {
    let mut unit = original.clone();
    // The documents don't depend on it, and the ledger answers to this run's plan.
    unit.compiler_runtime_bounds = capture.plan.compiler_runtime_bounds;
    unit.directory = tempfile::Builder::new()
        .prefix("invocation-reused-")
        .tempdir_in(&capture.directory)
        .map_err(|e| e.to_string())?
        .keep();
    Ok(unit)
}

fn copy_snapshot(original: &Unit, unit: &Unit) -> Result<(), String> {
    for file in super::stored_files(original)? {
        let (from, to) = (original.directory.join(&file), unit.directory.join(&file));
        // Published artifacts never change, so a run shares them instead of copying them.
        std::fs::hard_link(&from, &to)
            .or_else(|_| std::fs::copy(&from, &to).map(|_| ()))
            .map_err(|e| format!("Cannot materialize completed advisory owner: {e}"))?;
    }
    Ok(())
}

// rstyle-allow(side_effect_density): Capture restart boundary creates a fresh directory and republishes the existing plan.
fn restart(capture: &mut Capture) -> Result<(), String> {
    let parent = capture.directory.parent().ok_or("Missing capture parent")?;
    capture.directory = retention::new_run(parent)?;

    capture.units.clear();
    capture.failures.clear();
    write_json(&capture.directory.join("plan.json"), &capture.plan)?;
    write_json(
        &capture.directory.join("expected-unit-graph.json"),
        &capture.expected_unit_graph,
    )
}

fn publish(capture: &Capture, cargo: &[u8], tools: Vec<Tool>) -> Result<(), String> {
    persist_index(
        index_path(capture),
        &owners_index(capture, tools, entries(capture, cargo)?)?,
    )
}

fn owners_index(capture: &Capture, tools: Vec<Tool>, entries: Vec<Entry>) -> Result<Index, String> {
    Ok(Index {
        schema: SCHEMA,
        runs: capture
            .directory
            .parent()
            .ok_or("Missing capture parent")?
            .to_path_buf(),
        plan: capture.plan.clone(),
        tools,
        entries,
    })
}

// rstyle-allow(side_effect_density): IO boundary: writes the owner index a stopped capture leaves
/// Keeps the owners a capture asked to stop had finished, as the index the next capture reuses them from, and fails
/// saying it stopped. Cargo keeps every unit it finished, so the next capture compiles only the rest.
fn keep_stopped(capture: &Capture, cargo: &[u8], prepared: &Prepared) -> Result<Output, String> {
    let kept = match finished(capture, cargo, prepared.previous.as_ref())? {
        Some(entries) if !entries.is_empty() => {
            let count = entries.len();
            let index = owners_index(capture, prepared.tools.clone(), entries)?;
            persist_index(index_path(capture), &index)?;
            count
        }
        _ => 0,
    };
    Err(format!(
        "Capture stopped: kept {kept} finished owners for the next capture"
    ))
}

// rstyle-allow(side_effect_density): IO boundary: reads the stopped capture's ledgers and links reused owners
/// The owners a stopped capture finished, by the artifacts Cargo reported before it stopped: each from its ledger in
/// this run or, for one Cargo found fresh, from the previous index. None when one has neither, since the next capture
/// would find that unit fresh with nothing it captured.
fn finished(
    capture: &Capture,
    cargo: &[u8],
    previous: Option<&Index>,
) -> Result<Option<Vec<Entry>>, String> {
    let mut entries = Vec::new();
    for artifact in super::reported_artifacts(cargo) {
        let Some(target) = super::selected_artifact(capture, &artifact) else {
            continue;
        };
        let unit = match super::ledger_unit(capture, &artifact, target) {
            Ok(unit) => unit,
            Err(_) => match previous_entry(previous, target, &artifact) {
                Some(entry) if artifact["fresh"] == true => materialize(capture, &entry.unit)?,
                _ => return Ok(None),
            },
        };
        entries.push(Entry { artifact, unit });
    }
    Ok(Some(entries))
}

fn persist_index(path: PathBuf, index: &Index) -> Result<(), String> {
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("Missing owner index parent")?)
            .map_err(|e| e.to_string())?;
    serde_json::to_writer(file.as_file_mut(), index).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;

    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

fn entries(capture: &Capture, cargo: &[u8]) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for artifact in super::cargo_artifacts(cargo)? {
        let Some(target) = super::selected_artifact(capture, &artifact) else {
            continue;
        };
        let unit = capture
            .units
            .iter()
            .find(|unit| unit.target == *target && artifact["profile"]["test"] == unit.test)
            .ok_or("Missing reconciled advisory owner")?;
        entries.push(Entry {
            artifact,
            unit: unit.clone(),
        });
    }
    Ok(entries)
}
