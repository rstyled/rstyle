//! Explicit producer artifacts, independent of matching orchestration.

use rstyle_pdg::unit_store::UnitStore;
use serde::Serialize;
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct Destination {
    pub facts_file: PathBuf,
    pub document_directory: Option<PathBuf>,
}

/// The source inventory and facts published together for one completed capture.
pub struct CapturedOutput<'a> {
    pub store: &'a rstyle_pdg::unit_store::UnitStore,
    pub inventory: Option<&'a serde_json::Value>,
}

impl Destination {
    pub fn write_capture(&self, capture: &CapturedOutput<'_>) -> io::Result<()> {
        if let Some(inventory) = capture.inventory {
            self.write_source_inventory(inventory)?;
        }
        self.write_evidence(capture.store)
    }

    fn write_evidence(&self, store: &UnitStore) -> io::Result<()> {
        if self.document_directory.is_some() {
            self.write_shared(store)
        } else {
            // Raw frontend observations only; local helpers resolve through bundles.
            write_json(&self.facts_file, store.functions())
        }
    }

    fn shared_directory(&self) -> io::Result<&Path> {
        self.document_directory
            .as_deref()
            .ok_or_else(|| io::Error::other("Shared publication requires a unit directory"))
    }

    fn write_shared(&self, store: &UnitStore) -> io::Result<()> {
        let directory = self.shared_directory()?;
        std::fs::create_dir_all(directory)?;
        let manifest = write_sealed_file(directory.join("unit.evidence.json"), store)?;
        write_json(&directory.join("unit.evidence.manifest.json"), &manifest)
    }

    pub fn write_source_inventory(&self, value: &serde_json::Value) -> io::Result<()> {
        if let Some(directory) = &self.document_directory {
            std::fs::create_dir_all(directory)?;
            write_json(&directory.join("source-inventory.json"), value)?;
        }
        Ok(())
    }
}

fn write_sealed_file(
    path: PathBuf,
    store: &UnitStore,
) -> io::Result<rstyle_pdg::unit_store::BundleManifest> {
    let file = std::fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    store.write_sealed_bundle(writer).map_err(io::Error::other)
}

fn write_json(path: &Path, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;

    std::fs::write(path, bytes)
}
