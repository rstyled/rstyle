//! Completed raw observations, reused only with the current Cargo freshness decision.
use super::{Args, Capture, Plan, Unit, build_lane, read_json, write_json};
use crate::Cli;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Output,
    time::SystemTime,
};

#[derive(Serialize, Deserialize)]
pub(super) struct Index {
    schema: u32,
    plan: Plan,
    tools: Vec<Tool>,
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize, PartialEq)]
struct Tool {
    path: PathBuf,
    modified: SystemTime,
    len: u64,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    artifact: Value,
    unit: Unit,
}

fn index_path(capture: &Capture) -> PathBuf {
    capture.directory.with_file_name("advisory-owners.json")
}

pub(super) fn take(capture: &Capture, args: &Args) -> Result<Option<Index>, String> {
    if !build_lane::shared(capture, args) {
        return Ok(None);
    }
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
        if !build_lane::shared(capture, self.args) {
            return super::perform_compile(capture, self.cli, self.args);
        }
        self.compile_shared(capture, previous)
    }

    fn compile_shared(&self, capture: &mut Capture, previous: Option<Index>) -> Result<(), String> {
        let prepared = self.prepare(capture, previous)?;
        let output = self.cached(capture, prepared.previous.as_ref())?;

        super::compilation_result(capture, &output);
        publish_completed(capture, &output, prepared.tools)
    }

    fn prepare(&self, capture: &Capture, previous: Option<Index>) -> Result<Prepared, String> {
        let tools = tools(&capture.plan)?;
        let previous = previous.filter(|index| reusable(index, capture, &tools));
        if previous.is_none() {
            build_lane::refresh_selected(capture, self.args)?;
        }
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
        build_lane::refresh_selected(capture, self.args)?;
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
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok(Tool {
        path,
        modified: metadata.modified().map_err(|e| e.to_string())?,
        len: metadata.len(),
    })
}

fn reusable(index: &Index, capture: &Capture, tools: &[Tool]) -> bool {
    index.schema == 1
        && index.plan == capture.plan
        && index.tools == tools
        && !index.entries.is_empty()
        && index.entries.iter().all(|entry| {
            super::artifact_files(entry.unit.extraction_profile)
                .iter()
                .all(|file| entry.unit.directory.join(file).is_file())
        })
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
        let entry = previous.and_then(|index| {
            index.entries.iter().find(|entry| {
                entry.unit.target == *target && same_artifact(&entry.artifact, artifact)
            })
        });
        let Some(entry) = entry else {
            return Ok(false);
        };
        units.push(&entry.unit);
    }

    for unit in units {
        materialize(capture, unit)?;
    }
    Ok(true)
}

fn materialize(capture: &Capture, original: &Unit) -> Result<(), String> {
    let unit = reused_unit(capture, original)?;
    copy_snapshot(original, &unit)?;
    super::publish_ledger(&capture.plan, &capture.directory, &unit)
}

fn reused_unit(capture: &Capture, original: &Unit) -> Result<Unit, String> {
    let mut unit = original.clone();
    unit.directory = tempfile::Builder::new()
        .prefix("invocation-reused-")
        .tempdir_in(&capture.directory)
        .map_err(|e| e.to_string())?
        .keep();
    Ok(unit)
}

fn copy_snapshot(original: &Unit, unit: &Unit) -> Result<(), String> {
    for file in super::artifact_files(unit.extraction_profile) {
        std::fs::copy(original.directory.join(&file), unit.directory.join(&file))
            .map_err(|e| format!("Cannot materialize completed advisory owner: {e}"))?;
    }
    Ok(())
}

fn restart(capture: &mut Capture) -> Result<(), String> {
    let parent = capture.directory.parent().ok_or("Missing capture parent")?;
    capture.directory = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?
        .keep();

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
        &completed_index(capture, cargo, tools)?,
    )
}

fn completed_index(capture: &Capture, cargo: &[u8], tools: Vec<Tool>) -> Result<Index, String> {
    Ok(Index {
        schema: 1,
        plan: capture.plan.clone(),
        tools,
        entries: entries(capture, cargo)?,
    })
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
