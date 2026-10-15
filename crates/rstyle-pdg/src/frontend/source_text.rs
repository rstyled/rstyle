//! Immutable original bytes and checked ranges; identities belong to the bytes.
use crate::ByteSpan;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, Range},
    sync::{Arc, OnceLock},
};

/// An immutable UTF-8 range into an owned original source snapshot.
/// Wire encoding remains a string. The cached identity is computed from these
/// exact bytes, never installed from a serialized digest claim.
#[derive(Clone, Debug, Deserialize)]
#[serde(from = "String")]
pub struct SourceText(Arc<TextRange>);

#[derive(Debug)]
struct TextRange {
    snapshot: Arc<str>,
    range: Range<usize>,
    identity: OnceLock<String>,
}

impl SourceText {
    /// Create a checked range without copying the immutable original snapshot.
    /// A digest claim is never accepted as the identity of the range.
    pub fn from_snapshot(snapshot: Arc<str>, span: ByteSpan) -> Result<Self, String> {
        let range = span.start as usize..span.end as usize;
        snapshot
            .get(range.clone())
            .ok_or("Invalid original UTF-8 source range")?;
        Ok(Self(Arc::new(TextRange {
            snapshot,
            range,
            identity: OnceLock::new(),
        })))
    }

    pub(crate) fn snapshot_range(&self) -> Result<(&Arc<str>, ByteSpan), String> {
        let start = u32::try_from(self.0.range.start).map_err(|_| "Source range exceeds u32")?;
        let end = u32::try_from(self.0.range.end).map_err(|_| "Source range exceeds u32")?;
        Ok((&self.0.snapshot, ByteSpan { start, end }))
    }

    pub fn as_str(&self) -> &str {
        &self.0.snapshot[self.0.range.clone()]
    }

    /// SHA-256 of this exact immutable range, calculated at most once.
    pub fn identity(&self) -> &str {
        self.0
            .identity
            .get_or_init(|| crate::concrete_types::digest(self.as_bytes()))
    }

    pub(crate) fn in_snapshot(&self, snapshot: Arc<str>, span: ByteSpan) -> Result<Self, String> {
        let range = span.start as usize..span.end as usize;
        if snapshot.get(range.clone()) != Some(self.as_str()) {
            return Err("Captured function range disagrees with owned source snapshot".into());
        }
        Ok(Self(Arc::new(TextRange {
            snapshot,
            range,
            identity: self.0.identity.clone(),
        })))
    }
}

impl From<String> for SourceText {
    fn from(bytes: String) -> Self {
        let range = 0..bytes.len();
        Self(Arc::new(TextRange {
            snapshot: bytes.into(),
            range,
            identity: OnceLock::new(),
        }))
    }
}

impl From<&str> for SourceText {
    fn from(bytes: &str) -> Self {
        bytes.to_owned().into()
    }
}

impl Deref for SourceText {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for SourceText {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.as_str() == other.as_str()
    }
}
impl Eq for SourceText {}

impl PartialEq<String> for SourceText {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl Serialize for SourceText {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_snapshot_construction_preserves_owner_and_range_bounds() {
        let snapshot: Arc<str> = "π\r\nfn f() {}".into();
        let range = ByteSpan {
            start: 4,
            end: snapshot.len() as u32,
        };
        let source = SourceText::from_snapshot(Arc::clone(&snapshot), range).unwrap();
        assert_eq!(source.as_str(), "fn f() {}");
        assert!(Arc::ptr_eq(&snapshot, &source.0.snapshot));
        assert!(source.0.identity.get().is_none());
        assert_eq!(
            source.identity(),
            crate::concrete_types::digest(b"fn f() {}")
        );

        for span in [
            ByteSpan { start: 1, end: 2 },
            ByteSpan { start: 5, end: 3 },
            ByteSpan {
                start: 0,
                end: u32::MAX,
            },
        ] {
            assert!(SourceText::from_snapshot(Arc::clone(&snapshot), span).is_err());
        }
    }

    #[test]
    fn exact_ranges_share_original_bytes_and_one_actual_identity() {
        let snapshot: Arc<str> = "\u{feff}// π\r\nfn f() {}\r\n".into();
        let start = snapshot.find("fn").unwrap();
        let text = SourceText::from("fn f() {}");
        assert!(text.0.identity.get().is_none());
        let identity = text.identity().to_owned();
        let view = text
            .in_snapshot(
                Arc::clone(&snapshot),
                ByteSpan {
                    start: start as u32,
                    end: (start + text.len()) as u32,
                },
            )
            .unwrap();
        assert!(Arc::ptr_eq(&view.0.snapshot, &snapshot));
        assert_eq!(view.as_ptr(), snapshot[start..].as_ptr());
        assert_eq!(view.identity(), identity);
        let cloned = view.clone();
        assert!(std::ptr::eq(view.identity(), cloned.identity()));
        assert_eq!(serde_json::to_string(&view).unwrap(), "\"fn f() {}\"");
        let loaded: SourceText = serde_json::from_str("\"fn f() {}\"").unwrap();
        assert_eq!(loaded, view);
        assert!(loaded.0.identity.get().is_none());
        assert_ne!(SourceText::from("fn f() {1}").identity(), view.identity());
        for span in [
            ByteSpan { start: 1, end: 3 },
            ByteSpan { start: 9, end: 2 },
            ByteSpan {
                start: 0,
                end: u32::MAX,
            },
        ] {
            assert!(text.in_snapshot(Arc::clone(&snapshot), span).is_err());
        }
        assert!(
            SourceText::from("different")
                .in_snapshot(
                    snapshot,
                    ByteSpan {
                        start: start as u32,
                        end: (start + text.len()) as u32,
                    }
                )
                .is_err()
        );
    }
}
