//! Explicit raw-facts retention, separate from successful producer publication.
use rstyle_pdg::HirFnFacts;
use serde::Serialize;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

type Writer = io::BufWriter<std::fs::File>;

#[derive(Serialize)]
struct Snapshot<'a> {
    version: u32,
    scope: &'static str,
    phase: &'a str,
    facts: &'a [HirFnFacts],
}

pub fn snapshot(phase: &str, facts: &[HirFnFacts]) -> Result<(), String> {
    snapshot_directory()?
        .map(|directory| write_snapshot(directory, phase, facts))
        .unwrap_or(Ok(()))
}

fn snapshot_directory() -> Result<Option<PathBuf>, String> {
    let directory = std::env::var_os("RSTYLE_DIAGNOSTIC_FACTS_DIR");
    if directory.is_none() && rstyle_pdg::diagnostics::switch("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY") {
        return Err("capture-only requires RSTYLE_DIAGNOSTIC_FACTS_DIR".into());
    }

    Ok(directory.map(PathBuf::from))
}

fn write_snapshot(directory: PathBuf, phase: &str, facts: &[HirFnFacts]) -> Result<(), String> {
    let snapshot = Snapshot {
        version: 1,
        scope: "diagnostic-raw-facts-only",
        phase,
        facts,
    };
    persist(&directory, &snapshot).map_err(|error| error.to_string())
}

fn persist(directory: &Path, snapshot: &Snapshot<'_>) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    let path = directory.join(format!("{}-{}.json", std::process::id(), snapshot.phase));

    persist_snapshot(&path, snapshot)?;
    announce(&path, snapshot.phase);
    Ok(())
}

fn persist_snapshot(path: &Path, snapshot: &Snapshot<'_>) -> io::Result<()> {
    persist_with(path, |writer| snapshot.write(writer))
}

impl Snapshot<'_> {
    fn write(&self, writer: impl Write) -> io::Result<()> {
        if rstyle_pdg::diagnostics::switch("RSTYLE_DIAGNOSTIC_SHARED_SOURCES") {
            rstyle_pdg::unit_store::write_diagnostic_snapshot(writer, self.phase, self.facts)
                .map_err(io::Error::other)
        } else {
            serde_json::to_writer(writer, self).map_err(io::Error::other)
        }
    }
}

pub(super) fn persist_file(path: &Path, snapshot: &impl Serialize) -> io::Result<()> {
    persist_with(path, |writer| {
        serde_json::to_writer(writer, snapshot).map_err(io::Error::other)
    })
}

fn persist_with(path: &Path, write: impl FnOnce(&mut Writer) -> io::Result<()>) -> io::Result<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    write_file(file, write)
}

fn write_file(
    file: std::fs::File,
    write: impl FnOnce(&mut Writer) -> io::Result<()>,
) -> io::Result<()> {
    let mut writer = io::BufWriter::new(file);
    write(&mut writer)?;
    writer.flush()?;
    writer.get_ref().sync_all()
}

fn announce(path: &Path, phase: &str) {
    eprintln!(
        "rstyle diagnostic snapshot={} phase={phase} scope=diagnostic-raw-facts-only",
        path.display()
    );
}
