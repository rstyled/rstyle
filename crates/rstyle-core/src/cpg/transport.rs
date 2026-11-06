//! Language-neutral storage of unchanged JSON documents. See docs/cpg-transport.md.
mod capture;
mod manifest;
mod read;
mod sequence;
mod write;

pub use capture::{CaptureProgress, CaptureReader, read_manifest_bytes};
pub use manifest::{DOCUMENTS_MANIFEST, DocumentManifest, SealedDocument};
pub use read::{read_json, read_sealed, validate_object};
pub use sequence::{SequenceProgress, SequenceReader};
pub use write::{publish, publish_parsed, write_json};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;
use xxhash_rust::xxh3::Xxh3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    #[default]
    Json,
    Zstd,
}

impl Encoding {
    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Json => "none",
            Self::Zstd => "zstd",
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Json => ".cpg.json",
            Self::Zstd => ".cpg.json.zst",
        }
    }

    pub fn from_path(path: &std::path::Path) -> Self {
        if path.extension().is_some_and(|ext| ext == "zst") {
            Self::Zstd
        } else {
            Self::Json
        }
    }

    pub fn from_environment() -> Result<Self, String> {
        std::env::var("RSTYLE_CPG_COMPRESSION")
            .unwrap_or_else(|_| "zstd".into())
            .parse()
    }
}

impl std::str::FromStr for Encoding {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "none" | "json" => Ok(Self::Json),
            "zstd" => Ok(Self::Zstd),
            _ => Err("CPG compression must be none or zstd".into()),
        }
    }
}

#[derive(Default)]
pub(super) struct Observed {
    pub bytes: u64,
    sha: Sha256,
    legacy: Xxh3,
}

impl Observed {
    fn update(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| io::Error::other("Oversized CPG transport"))?;
        self.sha.update(bytes);
        self.legacy.update(bytes);
        Ok(())
    }

    pub fn checksum(&self) -> String {
        format!("sha256:{:x}", self.sha.clone().finalize())
    }

    pub fn matches(&self, bytes: u64, checksum: &str) -> bool {
        self.bytes == bytes
            && match checksum.strip_prefix("xxh3-64:") {
                Some(value) => format!("{:016x}", self.legacy.digest()) == value,
                None => self.checksum() == checksum,
            }
    }
}

#[cfg(test)]
mod tests;
