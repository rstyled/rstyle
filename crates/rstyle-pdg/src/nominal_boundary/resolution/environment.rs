//! Checked original source snapshots and lexical tables for one immutable census.
use super::*;
use std::path::Path;

#[derive(Clone, Copy)]
struct SourceId(u32);

/// Construction checks every source fingerprint and the closed module inventory.
/// No serialized record or unchecked bool can construct this view.
pub(crate) struct Environment {
    census: Arc<census::Census>,
    sources: BTreeMap<PathBuf, SourceId>,
    pub(super) modules: Vec<census::Module>,
    pub(super) index: Index,
}

pub(super) struct SnapshotView<'a> {
    record: &'a census::UnitFile,
}

impl<'a> SnapshotView<'a> {
    pub fn bytes(&self) -> &'a str {
        &self.record.bytes
    }

    pub fn range(&self, span: ByteSpan) -> Result<&'a str, String> {
        self.bytes()
            .get(span.start as usize..span.end as usize)
            .ok_or_else(|| "Source range is out of bounds or splits UTF-8".into())
    }
}

impl Environment {
    pub(crate) fn new(census: Arc<census::Census>) -> Result<Self, String> {
        let modules = census::verify(&census)?;
        let index = build_index(&modules, &census)?;
        let sources = census
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| {
                u32::try_from(index)
                    .map(|index| (file.path.clone(), SourceId(index)))
                    .map_err(|_| "Oversized source table".to_owned())
            })
            .collect::<Result<_, _>>()?;

        Ok(Self {
            census,
            sources,
            modules,
            index,
        })
    }

    pub(super) fn census(&self) -> &census::Census {
        &self.census
    }

    pub(super) fn source(&self, path: &Path) -> Result<SnapshotView<'_>, String> {
        let id = self
            .sources
            .get(path)
            .ok_or("Missing checked source snapshot")?;
        let record = self
            .census
            .files
            .get(id.0 as usize)
            .ok_or("Invalid source handle")?;
        Ok(SnapshotView { record })
    }

    pub(crate) fn check_context(&self, facts: &HirFnFacts) -> Result<(), String> {
        let context: UnitContext = evidence(facts, CONTEXT_KEY)?;
        if context != self.census.context {
            return Err("Nominal census belongs to another selected unit".into());
        }
        Ok(())
    }

    pub(crate) fn check_origin(
        &self,
        origin: &declaration_origins::Origin,
        declaration: &str,
    ) -> Result<(), String> {
        let path = Path::new(&origin.source_file);
        let source = self.source(path)?;
        if origin.file_bytes.as_ref() != source.bytes()
            || origin.file_digest != source.record.digest
            || source.range(origin.declaration_span)? != declaration
            || self
                .index
                .declarations
                .values()
                .filter(|value| value.path == path && value.span == origin.declaration_span)
                .count()
                != 1
        {
            return Err(
                "Declaration origin disagrees with checked source snapshot or item range".into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn census() -> Arc<census::Census> {
        let bytes = "\u{feff}// π\r\npub struct Data { pub value: i32 }\r\n";
        Arc::new(census::Census {
            identity: census::IDENTITY.into(),
            context: crate::nominal_boundary::tests::context(),
            files: vec![census::UnitFile {
                path: "/sample/src/lib.rs".into(),
                bytes: bytes.into(),
                digest: crate::concrete_types::digest(bytes.as_bytes()),
            }],
        })
    }

    #[test]
    fn checked_snapshot_ranges_retain_original_bytes_and_reject_bad_boundaries() {
        let census = census();
        let environment = Environment::new(Arc::clone(&census)).unwrap();
        let source = environment.source(Path::new("/sample/src/lib.rs")).unwrap();
        assert!(std::ptr::eq(
            source.bytes().as_ptr(),
            census.files[0].bytes.as_ptr()
        ));
        assert_eq!(
            source.range(ByteSpan { start: 0, end: 3 }).unwrap(),
            "\u{feff}"
        );
        for span in [
            ByteSpan { start: 1, end: 3 },
            ByteSpan { start: 9, end: 2 },
            ByteSpan {
                start: 0,
                end: u32::MAX,
            },
        ] {
            assert!(source.range(span).is_err());
        }
        assert!(environment.source(Path::new("/other/src/lib.rs")).is_err());
        let mut changed = (*census).clone();
        changed.files[0].bytes = format!("{} ", changed.files[0].bytes).into();
        assert!(Environment::new(Arc::new(changed)).is_err());
    }

    #[test]
    fn origins_use_checked_snapshot_and_exact_parsed_struct_range() {
        let census = census();
        let environment = Environment::new(Arc::clone(&census)).unwrap();
        let declaration = "pub struct Data { pub value: i32 }";
        let bytes = &census.files[0].bytes;
        let start = bytes.find(declaration).unwrap() as u32;
        let mut origin = declaration_origins::Origin {
            nominal: crate::concrete_types::Nominal {
                source_identity: Some(census.context.package.package_id.clone()),
                package_name: "sample".into(),
                package_version: "1.0.0".into(),
                manifest_digest: census.context.package.manifest_digest.clone(),
                definition_path: "sample::Data".into(),
                declaration_digest: crate::concrete_types::digest(declaration.as_bytes()),
            },
            source_file: "/sample/src/lib.rs".into(),
            file_bytes: bytes.clone(),
            file_digest: census.files[0].digest.clone(),
            declaration_span: ByteSpan {
                start,
                end: start + declaration.len() as u32,
            },
        };
        environment.check_origin(&origin, declaration).unwrap();
        origin.file_digest.push('x');
        assert!(environment.check_origin(&origin, declaration).is_err());
        origin.file_digest = census.files[0].digest.clone();
        origin.file_bytes = format!("{} ", origin.file_bytes).into();
        assert!(environment.check_origin(&origin, declaration).is_err());
        origin.file_bytes = bytes.clone();
        origin.declaration_span.start += 4;
        assert!(
            environment
                .check_origin(&origin, "struct Data { pub value: i32 }")
                .is_err()
        );
    }
    #[test]
    fn reordered_files_keep_owner_scoped_sources_and_ranges() {
        let mut raw = (*census()).clone();
        raw.files[0].bytes = format!("{}mod child;\n", raw.files[0].bytes).into();
        raw.files[0].digest = crate::concrete_types::digest(raw.files[0].bytes.as_bytes());
        raw.files.push(census::UnitFile {
            path: "/sample/src/child.rs".into(),
            bytes: "pub struct Other;".into(),
            digest: crate::concrete_types::digest(b"pub struct Other;"),
        });
        let first = Environment::new(Arc::new(raw.clone())).unwrap();
        raw.files.reverse();
        let second = Environment::new(Arc::new(raw)).unwrap();
        for path in [
            Path::new("/sample/src/lib.rs"),
            Path::new("/sample/src/child.rs"),
        ] {
            assert_eq!(
                first.source(path).unwrap().bytes(),
                second.source(path).unwrap().bytes()
            );
            assert_ne!(first.sources[path].0, second.sources[path].0);
        }
        assert_eq!(
            first
                .source(Path::new("/sample/src/lib.rs"))
                .unwrap()
                .range(ByteSpan { start: 0, end: 3 })
                .unwrap(),
            "\u{feff}"
        );
    }
}
