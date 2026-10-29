//! Explicit producer artifacts, independent of matching orchestration.

use rstyle_pdg::unit_store::{BundleManifest, UnitStore, write_sealed_diagnostic_snapshot};
use serde::Serialize;
use std::{
    io,
    path::{Path, PathBuf},
};

type Writer = io::BufWriter<std::fs::File>;
type Publication = Result<BundleManifest, String>;

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
    /// Publish a successful advisory capture without constructing strict evidence.
    pub fn write_advisory_capture(
        &self,
        facts: &[rstyle_pdg::HirFnFacts],
        inventory: Option<&serde_json::Value>,
    ) -> io::Result<()> {
        self.write_optional_inventory(inventory)?;
        self.write_advisory(facts)
    }

    fn write_advisory(&self, facts: &[rstyle_pdg::HirFnFacts]) -> io::Result<()> {
        self.write_shared_with(|writer| {
            write_sealed_diagnostic_snapshot(writer, "compiler-advisory-only", facts)
        })
    }

    pub fn write_capture(&self, capture: &CapturedOutput<'_>) -> io::Result<()> {
        self.write_optional_inventory(capture.inventory)?;
        self.write_evidence(capture.store)
    }

    fn write_optional_inventory(&self, inventory: Option<&serde_json::Value>) -> io::Result<()> {
        inventory.map_or(Ok(()), |value| self.write_source_inventory(value))
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
        self.write_shared_with(|writer| store.write_sealed_bundle(writer))
    }

    fn write_shared_with(&self, publish: impl FnOnce(Writer) -> Publication) -> io::Result<()> {
        let directory = self.shared_directory()?;
        std::fs::create_dir_all(directory)?;
        let manifest = write_sealed_file(directory.join("unit.evidence.json"), publish)?;
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
    publish: impl FnOnce(Writer) -> Publication,
) -> io::Result<BundleManifest> {
    let file = std::fs::File::create(path)?;
    let writer = io::BufWriter::new(file);
    publish(writer).map_err(io::Error::other)
}

fn write_json(path: &Path, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;

    std::fs::write(path, bytes)
}
