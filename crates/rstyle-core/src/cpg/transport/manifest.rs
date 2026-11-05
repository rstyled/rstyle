use super::{Encoding, Observed};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

pub const DOCUMENTS_MANIFEST: &str = "documents.manifest.json";
const SCHEMA: &str = "rstyle.cpg-documents.manifest/2";
const LEGACY: &str = "rstyle.cpg-documents.manifest/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedDocument {
    pub file: String,
    pub checksum: String,
    pub bytes: u64,
    #[serde(default)]
    pub encoding: Encoding,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoded_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoded_checksum: Option<String>,
}

impl SealedDocument {
    /// Seal a plain document supplied by an existing producer.
    pub fn new(position: usize, bytes: &[u8]) -> Self {
        let mut observed = Observed::default();
        observed
            .update(bytes)
            .expect("slice length fits a transport");
        Self {
            file: format!("document-{position:06}.cpg.json"),
            checksum: observed.checksum(),
            bytes: observed.bytes,
            encoding: Encoding::Json,
            decoded_bytes: Some(observed.bytes),
            decoded_checksum: Some(observed.checksum()),
        }
    }

    pub fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        let mut observed = Observed::default();
        observed.update(bytes).map_err(|error| error.to_string())?;
        self.verify_stored(&observed)
    }

    pub(super) fn verify_stored(&self, observed: &Observed) -> Result<(), String> {
        if !observed.matches(self.bytes, &self.checksum) {
            return Err(format!(
                "Captured document {} disagrees with its manifest",
                self.file
            ));
        }
        Ok(())
    }

    pub(super) fn verify_decoded(&self, observed: &Observed) -> Result<(), String> {
        if let (Some(bytes), Some(checksum)) = (self.decoded_bytes, &self.decoded_checksum)
            && !observed.matches(bytes, checksum)
        {
            return Err(format!(
                "Decoded document {} disagrees with its manifest",
                self.file
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentManifest {
    pub schema: String,
    pub documents: Vec<SealedDocument>,
}

impl DocumentManifest {
    pub fn of(documents: Vec<SealedDocument>) -> Self {
        Self {
            schema: SCHEMA.into(),
            documents,
        }
    }

    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        if manifest.schema != SCHEMA && manifest.schema != LEGACY {
            return Err("Unsupported compat.cpg document manifest".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for document in &manifest.documents {
            let mut path = Path::new(&document.file).components();
            if !matches!(path.next(), Some(Component::Normal(_)))
                || path.next().is_some()
                || document.file.contains('\\')
                || !seen.insert(&document.file)
            {
                return Err(
                    "A document manifest names a duplicate or a file outside its directory".into(),
                );
            }
            if manifest.schema == SCHEMA {
                if document.decoded_bytes.is_none()
                    || document.decoded_checksum.is_none()
                    || !document.file.ends_with(document.encoding.suffix())
                    || !document.checksum.starts_with("sha256:")
                    || !document
                        .decoded_checksum
                        .as_ref()
                        .is_some_and(|s| s.starts_with("sha256:"))
                {
                    return Err("Incomplete or inconsistent compat.cpg transport seal".into());
                }
            } else if document.encoding != Encoding::Json
                || document.decoded_bytes.is_some()
                || document.decoded_checksum.is_some()
            {
                return Err("Legacy document manifests describe plain JSON only".into());
            }
        }
        Ok(manifest)
    }
}
