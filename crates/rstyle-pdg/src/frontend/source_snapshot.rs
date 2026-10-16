//! Immutable file ownership with an actual identity memo, never a digest claim.
use serde::{Deserialize, Serialize};
use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

/// One immutable UTF-8 source snapshot. Clones share bytes and the actual digest;
/// deserialization always creates a new owner with an empty identity memo.
#[derive(Clone, Debug, Deserialize)]
#[serde(from = "String")]
pub struct SourceSnapshot(Arc<Snapshot>);

#[derive(Debug)]
struct Snapshot {
    bytes: Arc<str>,
    identity: OnceLock<String>,
}

impl SourceSnapshot {
    /// Actual SHA-256 of the owned bytes, calculated at most once per owner.
    pub fn identity(&self) -> &str {
        self.0.identity.get_or_init(|| {
            crate::diagnostics::Stage::new(
                "source.snapshot_hash",
                "immutable-file",
                Some(self.as_bytes()),
            )
            .observe(|| Ok::<_, String>(crate::concrete_types::digest(self.as_bytes())))
            .expect("infallible source identity")
        })
    }

    pub(super) fn cached_identity(&self) -> Option<&String> {
        self.0.identity.get()
    }

    /// Whether both views share the same immutable owner and identity memo.
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Arc::ptr_eq(&this.0, &other.0)
    }
}

impl From<Arc<str>> for SourceSnapshot {
    fn from(bytes: Arc<str>) -> Self {
        Self(Arc::new(Snapshot {
            bytes,
            identity: OnceLock::new(),
        }))
    }
}
impl From<String> for SourceSnapshot {
    fn from(bytes: String) -> Self {
        Self::from(Arc::<str>::from(bytes))
    }
}
impl From<&str> for SourceSnapshot {
    fn from(bytes: &str) -> Self {
        Self::from(bytes.to_owned())
    }
}
impl Deref for SourceSnapshot {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0.bytes
    }
}
impl AsRef<str> for SourceSnapshot {
    fn as_ref(&self) -> &str {
        self
    }
}
impl PartialEq for SourceSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Self::ptr_eq(self, other) || self.as_ref() == other.as_ref()
    }
}
impl Eq for SourceSnapshot {}
impl Serialize for SourceSnapshot {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

impl std::fmt::Display for SourceSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_identity_is_shared_but_never_imported_or_inherited_by_edits() {
        let bytes: SourceSnapshot = "\u{feff}// π\r\nstruct A;\r\n".into();
        assert!(bytes.0.identity.get().is_none());
        let cloned = bytes.clone();
        let actual = bytes.identity();
        assert_eq!(actual, crate::concrete_types::digest(bytes.as_bytes()));
        assert!(std::ptr::eq(actual, cloned.identity()));
        assert!(SourceSnapshot::ptr_eq(&bytes, &cloned));

        let encoded = serde_json::to_vec(&bytes).unwrap();
        let imported: SourceSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert!(!SourceSnapshot::ptr_eq(&bytes, &imported));
        assert!(imported.0.identity.get().is_none());
        assert_eq!(actual, imported.identity());
        let edited: SourceSnapshot = format!("{bytes}// edited").into();
        assert!(edited.0.identity.get().is_none());
        assert_ne!(actual, edited.identity());
    }
}
