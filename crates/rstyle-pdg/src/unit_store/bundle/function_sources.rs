//! Bundle-local source ownership; readable facts are restored before inspection.
use crate::frontend::*;
use crate::{
    frontend::option_bundle,
    source_table::{self, SourceId},
};
use serde::{Deserialize, Serialize};
pub(super) mod body;
mod build;

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
pub(super) struct Export<
    'a,
    Body = body::Export<'a>,
    Bindings = &'a [BindingFact],
    Build = build::Export<'a>,
> {
    name: &'a str,
    fn_span: ByteSpan,
    source: Reference<'a>,
    signature: &'a SignatureFacts,
    bindings: Bindings,
    body: Body,
    build: Build,
    source_diagnostics: &'a [HirDiagnostic],
    source_scan_complete: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Import<
    B = body::Import,
    Bindings = Vec<BindingFact>,
    Build = BuildFacts<option_bundle::Import>,
> {
    name: String,
    fn_span: ByteSpan,
    source: ImportedReference,
    signature: SignatureFacts,
    bindings: Bindings,
    body: B,
    build: Build,
    source_diagnostics: Vec<HirDiagnostic>,
    source_scan_complete: bool,
}

pub(super) struct Encoded<
    'a,
    Body = body::Export<'a>,
    Bindings = &'a [BindingFact],
    Build = build::Export<'a>,
> {
    pub snapshots: Vec<&'a str>,
    pub functions: Vec<Export<'a, Body, Bindings, Build>>,
    pub inventories: option_bundle::Inventories<'a>,
}

#[derive(Default)]
struct Owners<'a> {
    table: source_table::Owners<'a>,
}

impl<'a> Owners<'a> {
    fn reference(&mut self, source: &'a SourceFacts) -> Result<Reference<'a>, String> {
        let (snapshot, range) = source.bytes.snapshot_range()?;
        Ok(Reference {
            snapshot: self.table.snapshot(snapshot)?,
            range,
            digest: &source.digest,
        })
    }
}

pub(super) fn encode(facts: &[HirFnFacts]) -> Result<Encoded<'_>, String> {
    encode_with(
        facts,
        |facts| (body::Export(&facts.body), facts.bindings.as_slice()),
        option_bundle::Inventories::default(),
    )
}

/// Share source/build encoding while selecting the original payload representation.
pub(super) fn encode_with<'a, B, Bindings, WireBody, WireBindings>(
    facts: &'a [HirFnFacts<B, Bindings>],
    payload: impl Fn(&'a HirFnFacts<B, Bindings>) -> (WireBody, WireBindings),
    inventories: option_bundle::Inventories<'a>,
) -> Result<Encoded<'a, WireBody, WireBindings>, String> {
    encode_with_build(facts, payload, inventories, build::Export::new)
}

pub(super) fn encode_with_build<'a, B, Bindings, WireBody, WireBindings, Build>(
    facts: &'a [HirFnFacts<B, Bindings>],
    payload: impl Fn(&'a HirFnFacts<B, Bindings>) -> (WireBody, WireBindings),
    mut inventories: option_bundle::Inventories<'a>,
    mut build: impl FnMut(
        &'a BuildFacts,
        &mut source_table::Owners<'a>,
        &mut option_bundle::Inventories<'a>,
    ) -> Result<Build, String>,
) -> Result<Encoded<'a, WireBody, WireBindings, Build>, String> {
    let mut owners = Owners::default();
    let functions = facts
        .iter()
        .map(|facts| {
            let source = owners.reference(&facts.source)?;
            let (body, bindings) = payload(facts);
            Ok(Export {
                name: &facts.name,
                fn_span: facts.fn_span,
                source,
                signature: &facts.signature,
                bindings,
                body,
                build: build(&facts.build, &mut owners.table, &mut inventories)?,
                source_diagnostics: &facts.source_diagnostics,
                source_scan_complete: facts.source_scan_complete,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Encoded {
        snapshots: owners.table.snapshots,
        functions,
        inventories,
    })
}

#[cfg(test)]
impl ImportedReference {
    fn bytes(&self, sources: &[crate::SourceSnapshot]) -> Result<SourceText, String> {
        let snapshot = sources
            .get(self.snapshot.0 as usize)
            .ok_or("Dangling bundle source reference")?;
        SourceText::from_snapshot(snapshot.clone(), self.range)
    }
}

pub(super) type FullBody = body::Import;
pub(super) type Headers = body::Import<body::HeaderNode>;

pub(super) trait BodyImport {
    type Output;
    fn into_body(self, flat_bodies: bool) -> Result<Self::Output, String>;
}

impl<N: body::BodyNode> BodyImport for body::Import<N> {
    type Output = N::Output;
    fn into_body(self, flat_bodies: bool) -> Result<Self::Output, String> {
        self.into_node(flat_bodies)
    }
}

impl BodyImport for HirRef {
    type Output = Self;
    fn into_body(self, _flat_bodies: bool) -> Result<Self, String> {
        Ok(self)
    }
}

impl<B: BodyImport, Bindings> Import<B, Bindings> {
    pub(super) fn into_facts(
        self,
        sources: &mut source_table::Imported<'_>,
        inventories: &mut option_bundle::ImportedInventories,
        flat_bodies: bool,
    ) -> Result<HirFnFacts<B::Output, Bindings>, String> {
        self.into_facts_with(
            Decode {
                sources,
                inventories,
                flat_bodies,
            },
            Ok,
        )
    }
}

pub(super) struct Decode<'borrow, 'source> {
    pub sources: &'borrow mut source_table::Imported<'source>,
    pub inventories: &'borrow mut option_bundle::ImportedInventories,
    pub flat_bodies: bool,
}

impl<B: BodyImport, Bindings, Build> Import<B, Bindings, Build> {
    pub(super) fn into_facts_with(
        self,
        decode: Decode<'_, '_>,
        restore: impl FnOnce(Build) -> Result<BuildFacts<option_bundle::Import>, String>,
    ) -> Result<HirFnFacts<B::Output, Bindings>, String> {
        let bytes = decode
            .sources
            .range(self.source.snapshot, self.source.range)?;
        Ok(HirFnFacts {
            name: self.name,
            fn_span: self.fn_span,
            source: SourceFacts {
                bytes,
                digest: self.source.digest,
            },
            signature: self.signature,
            bindings: self.bindings,
            body: self.body.into_body(decode.flat_bodies)?,
            build: restore(self.build)?
                .try_map_options(|options| options.decode(decode.sources, decode.inventories))?,
            source_diagnostics: self.source_diagnostics,
            source_scan_complete: self.source_scan_complete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_share_one_snapshot_without_trusting_digest_claims() {
        let snapshot: crate::SourceSnapshot = "\u{feff}// π\r\nfn f() {}\r\nfn g() {}".into();
        let start = snapshot.find("fn").unwrap() as u32;
        let first = SourceFacts {
            bytes: SourceText::from_snapshot(
                snapshot.clone(),
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
                snapshot.clone(),
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
        assert_eq!(owners.table.snapshots, [snapshot.as_ref()]);
        let wire = serde_json::to_value(&first_wire).unwrap();
        let imported: ImportedReference = serde_json::from_value(wire).unwrap();
        let restored = imported.bytes(std::slice::from_ref(&snapshot)).unwrap();
        assert_eq!(restored, first.bytes);
        assert_eq!(restored.as_ptr(), first.bytes.as_ptr());
        assert_ne!(restored.identity(), imported.digest);
        let second_imported: ImportedReference =
            serde_json::from_value(serde_json::to_value(second_wire).unwrap()).unwrap();
        let second_restored = second_imported.bytes(&[snapshot]).unwrap();
        assert!(crate::SourceSnapshot::ptr_eq(
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
        let sources: Vec<crate::SourceSnapshot> = vec!["π\r\nfn f() {}".into()];
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
        assert!(source_table::Imported::new(&sources).finish().is_err());
        assert!(source_table::Imported::new(&[]).finish().is_ok());
    }
}
