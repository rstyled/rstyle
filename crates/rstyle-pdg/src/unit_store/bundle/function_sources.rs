//! Bundle-local source ownership; readable facts are restored before inspection.
use crate::frontend::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct SourceId(u32);

#[derive(Serialize)]
struct Reference<'a> {
    snapshot: SourceId,
    range: ByteSpan,
    digest: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportedReference {
    snapshot: SourceId,
    range: ByteSpan,
    digest: String,
}

#[derive(Serialize)]
pub(super) struct Export<'a> {
    name: &'a str,
    fn_span: ByteSpan,
    source: Reference<'a>,
    signature: &'a SignatureFacts,
    bindings: &'a [BindingFact],
    body: &'a HirNode,
    build: &'a BuildFacts,
    source_diagnostics: &'a [HirDiagnostic],
    source_scan_complete: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Import {
    name: String,
    fn_span: ByteSpan,
    source: ImportedReference,
    signature: SignatureFacts,
    bindings: Vec<BindingFact>,
    body: HirNode,
    build: BuildFacts,
    source_diagnostics: Vec<HirDiagnostic>,
    source_scan_complete: bool,
}

pub(super) struct Encoded<'a> {
    pub snapshots: Vec<&'a str>,
    pub functions: Vec<Export<'a>>,
}

#[derive(Default)]
struct Owners<'a> {
    snapshots: Vec<&'a str>,
    ids: BTreeMap<usize, SourceId>,
}

impl<'a> Owners<'a> {
    fn reference(&mut self, source: &'a SourceFacts) -> Result<Reference<'a>, String> {
        let (snapshot, range) = source.bytes.snapshot_range()?;
        let pointer = snapshot.as_ptr() as usize;
        let next =
            SourceId(u32::try_from(self.snapshots.len()).map_err(|_| "Oversized source table")?);
        let id = *self.ids.entry(pointer).or_insert_with(|| {
            self.snapshots.push(snapshot.as_ref());
            next
        });

        Ok(Reference {
            snapshot: id,
            range,
            digest: &source.digest,
        })
    }
}

pub(super) fn encode(facts: &[HirFnFacts]) -> Result<Encoded<'_>, String> {
    let mut owners = Owners::default();
    let functions = facts
        .iter()
        .map(|function| Ok(Export::new(function, owners.reference(&function.source)?)))
        .collect::<Result<_, String>>()?;
    Ok(Encoded {
        snapshots: owners.snapshots,
        functions,
    })
}

impl<'a> Export<'a> {
    fn new(facts: &'a HirFnFacts, source: Reference<'a>) -> Self {
        Self {
            name: &facts.name,
            fn_span: facts.fn_span,
            source,
            signature: &facts.signature,
            bindings: &facts.bindings,
            body: &facts.body,
            build: &facts.build,
            source_diagnostics: &facts.source_diagnostics,
            source_scan_complete: facts.source_scan_complete,
        }
    }
}

pub(super) fn decode(
    sources: &[Arc<str>],
    functions: Vec<Import>,
) -> Result<Vec<HirFnFacts>, String> {
    let mut used = vec![false; sources.len()];
    let facts = functions
        .into_iter()
        .map(|function| {
            let index = function.source.snapshot.0 as usize;
            let bytes = function.source.bytes(sources)?;
            used[index] = true;
            Ok(function.into_facts(bytes))
        })
        .collect::<Result<_, String>>()?;
    if used.contains(&false) {
        return Err("Unreferenced bundle source snapshot".into());
    }
    Ok(facts)
}

impl ImportedReference {
    fn bytes(&self, sources: &[Arc<str>]) -> Result<SourceText, String> {
        let snapshot = sources
            .get(self.snapshot.0 as usize)
            .ok_or("Dangling bundle source reference")?;
        SourceText::from_snapshot(Arc::clone(snapshot), self.range)
    }
}

impl Import {
    fn into_facts(self, bytes: SourceText) -> HirFnFacts {
        HirFnFacts {
            name: self.name,
            fn_span: self.fn_span,
            source: SourceFacts {
                bytes,
                digest: self.source.digest,
            },
            signature: self.signature,
            bindings: self.bindings,
            body: self.body,
            build: self.build,
            source_diagnostics: self.source_diagnostics,
            source_scan_complete: self.source_scan_complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_share_one_snapshot_without_trusting_digest_claims() {
        let snapshot: Arc<str> = "\u{feff}// π\r\nfn f() {}\r\nfn g() {}".into();
        let start = snapshot.find("fn").unwrap() as u32;
        let first = SourceFacts {
            bytes: SourceText::from_snapshot(
                Arc::clone(&snapshot),
                ByteSpan {
                    start,
                    end: start + 9,
                },
            )
            .unwrap(),
            digest: "sha256:untrusted-claim".into(),
        };
        let second = SourceFacts {
            bytes: SourceText::from_snapshot(
                Arc::clone(&snapshot),
                ByteSpan {
                    start: start + 11,
                    end: snapshot.len() as u32,
                },
            )
            .unwrap(),
            digest: "sha256:another-claim".into(),
        };
        let mut owners = Owners::default();
        let first_wire = owners.reference(&first).unwrap();
        let second_wire = owners.reference(&second).unwrap();
        assert_eq!(first_wire.snapshot.0, second_wire.snapshot.0);
        assert_eq!(owners.snapshots, [snapshot.as_ref()]);
        let wire = serde_json::to_value(&first_wire).unwrap();
        let imported: ImportedReference = serde_json::from_value(wire).unwrap();
        let restored = imported.bytes(&[Arc::clone(&snapshot)]).unwrap();
        assert_eq!(restored, first.bytes);
        assert_eq!(restored.as_ptr(), first.bytes.as_ptr());
        assert_ne!(restored.identity(), imported.digest);
        let second_imported: ImportedReference =
            serde_json::from_value(serde_json::to_value(second_wire).unwrap()).unwrap();
        let second_restored = second_imported.bytes(&[snapshot]).unwrap();
        assert!(Arc::ptr_eq(
            restored.snapshot_range().unwrap().0,
            second_restored.snapshot_range().unwrap().0
        ));
    }

    #[test]
    fn equal_content_from_independent_owners_keeps_distinct_local_handles() {
        let first = SourceFacts {
            bytes: "fn f() {}".into(),
            digest: "claim".into(),
        };
        let second = SourceFacts {
            bytes: "fn f() {}".into(),
            digest: "claim".into(),
        };
        let clone = first.clone();
        let mut owners = Owners::default();
        assert_eq!(
            owners.reference(&first).unwrap().snapshot.0,
            owners.reference(&clone).unwrap().snapshot.0
        );
        assert_ne!(
            owners.reference(&first).unwrap().snapshot.0,
            owners.reference(&second).unwrap().snapshot.0
        );
    }

    #[test]
    fn imported_references_reject_missing_owners_and_invalid_ranges() {
        let sources: Vec<Arc<str>> = vec!["π\r\nfn f() {}".into()];
        let reference = ImportedReference {
            snapshot: SourceId(0),
            range: ByteSpan { start: 0, end: 2 },
            digest: "claim".into(),
        };
        assert!(reference.bytes(&sources).is_ok());
        for range in [
            ByteSpan { start: 1, end: 2 },
            ByteSpan { start: 4, end: 3 },
            ByteSpan {
                start: 0,
                end: u32::MAX,
            },
        ] {
            assert!(
                ImportedReference {
                    range,
                    snapshot: reference.snapshot,
                    digest: "claim".into()
                }
                .bytes(&sources)
                .is_err()
            );
        }
        assert!(
            ImportedReference {
                snapshot: SourceId(1),
                range: reference.range,
                digest: "claim".into()
            }
            .bytes(&sources)
            .is_err()
        );
        assert!(reference.bytes(&[]).is_err());
        assert!(decode(&sources, Vec::new()).is_err());
        assert!(decode(&[], Vec::new()).unwrap().is_empty());
    }
}
