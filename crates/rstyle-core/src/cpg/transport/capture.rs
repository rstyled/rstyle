//! A completed capture is read against one immutable manifest snapshot.
use super::{DOCUMENTS_MANIFEST, DocumentManifest, Observed, SealedDocument, read_sealed};
use serde::{Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

/// Only successfully decoded documents contribute to these verified byte totals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CaptureProgress {
    pub manifest_schema: String,
    pub manifest_checksum: String,
    pub expected_documents: usize,
    pub verified_documents: usize,
    pub stored_bytes: u64,
    pub decoded_bytes: u64,
}

impl CaptureProgress {
    fn recorded(&self, seal: &SealedDocument) -> Result<Self, String> {
        Ok(Self {
            verified_documents: self.verified_documents + 1,
            stored_bytes: total(self.stored_bytes, seal.bytes)?,
            // Legacy manifests describe plain JSON, whose two sizes are identical.
            decoded_bytes: total(self.decoded_bytes, seal.decoded_bytes.unwrap_or(seal.bytes))?,
            ..self.clone()
        })
    }
}

fn total(previous: u64, bytes: u64) -> Result<u64, String> {
    previous
        .checked_add(bytes)
        .ok_or_else(|| "Capture byte totals overflow u64".into())
}

/// Opens the manifest once and verifies each document in its declared order.
/// A read error terminates iteration. Call `complete` before accepting the capture.
pub struct CaptureReader {
    directory: PathBuf,
    manifest: DocumentManifest,
    progress: CaptureProgress,
    position: usize,
    failed: bool,
}

impl CaptureReader {
    pub fn open(directory: &Path) -> Result<Self, String> {
        Self::from_manifest(directory, manifest_bytes(directory)?)
    }

    fn from_manifest(directory: &Path, bytes: Vec<u8>) -> Result<Self, String> {
        let manifest = DocumentManifest::read(&bytes)?;
        if manifest.documents.is_empty() {
            return Err("A completed capture must contain at least one document".into());
        }
        let progress = CaptureProgress {
            manifest_schema: manifest.schema.clone(),
            manifest_checksum: checksum(&bytes)?,
            expected_documents: manifest.documents.len(),
            verified_documents: 0,
            stored_bytes: 0,
            decoded_bytes: 0,
        };

        Ok(Self {
            directory: directory.into(),
            manifest,
            progress,
            position: 0,
            failed: false,
        })
    }

    pub fn progress(&self) -> &CaptureProgress {
        &self.progress
    }

    /// Typed callers can consume one document without constructing a generic JSON tree.
    pub fn next_document<T: DeserializeOwned>(&mut self) -> Result<Option<T>, String> {
        if self.failed {
            return Ok(None);
        }

        let result = self.read_next();
        self.failed = result.is_err();
        result
    }

    fn read_next<T: DeserializeOwned>(&mut self) -> Result<Option<T>, String> {
        let Some(seal) = self.manifest.documents.get(self.position) else {
            return Ok(None);
        };
        let value = read_sealed(&self.directory, seal)
            .map_err(|error| format!("{}: {error}", seal.file))?;

        self.progress = self.progress.recorded(seal)?;
        self.position += 1;
        Ok(Some(value))
    }

    /// Completeness covers transport verification; callers validate the CPG's contents.
    pub fn complete(&self) -> Result<(), String> {
        if self.failed || self.progress.verified_documents != self.progress.expected_documents {
            return Err("Capture was not completely verified".into());
        }
        Ok(())
    }
}

impl Iterator for CaptureReader {
    type Item = Result<serde_json::Value, String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_document().transpose()
    }
}

fn manifest_bytes(directory: &Path) -> Result<Vec<u8>, String> {
    let path = directory.join(DOCUMENTS_MANIFEST);
    std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
}

fn checksum(bytes: &[u8]) -> Result<String, String> {
    let mut observed = Observed::default();
    observed.update(bytes).map_err(|error| error.to_string())?;
    Ok(observed.checksum())
}

#[cfg(test)]
mod tests;
