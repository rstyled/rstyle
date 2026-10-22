//! Checked original source snapshots and lexical tables for one immutable census.
use super::*;
use std::path::Path;
mod functions;

#[derive(Clone, Copy)]
struct SourceId(u32);

pub(crate) struct SourceOptions<'a> {
    pub constants: bool,
    pub cfgs: Option<&'a [String]>,
    pub expansions: bool,
    pub module_lints: bool,
}

/// Construction checks every source fingerprint and the closed module inventory.
/// No serialized record or unchecked bool can construct this view.
pub(crate) struct Environment {
    census: Arc<census::Census>,
    sources: BTreeMap<PathBuf, SourceId>,
    pub(super) modules: Vec<census::Module>,
    functions: functions::Index,
    pub(super) index: Index,
    pub(super) copy_boundary: bool,
    cleanup_sources: std::cell::RefCell<std::collections::BTreeSet<CleanupSource>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct CleanupSource {
    cfgs: Vec<String>,
    target: Namespace,
    profile: &'static str,
    generated: bool,
}

impl CleanupSource {
    fn new(cfgs: &[String], target: &Namespace, profile: &'static str) -> Self {
        Self {
            cfgs: cfgs.to_vec(),
            target: target.clone(),
            profile,
            generated: crate::verification_policy::is_generated(),
        }
    }
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
    #[cfg(test)]
    pub(crate) fn new(census: Arc<census::Census>) -> Result<Self, String> {
        Self::with_constants(census, false)
    }

    #[cfg(test)]
    pub(crate) fn with_constants(
        census: Arc<census::Census>,
        constants: bool,
    ) -> Result<Self, String> {
        Self::with_options(census, constants, None, false)
    }

    #[cfg(test)]
    pub(crate) fn with_options(
        census: Arc<census::Census>,
        constants: bool,
        cfgs: Option<&[String]>,
        expansions: bool,
    ) -> Result<Self, String> {
        Self::with_source_options(
            census,
            SourceOptions {
                constants,
                cfgs,
                expansions,
                module_lints: false,
            },
        )
    }

    pub(crate) fn with_source_options(
        census: Arc<census::Census>,
        options: SourceOptions<'_>,
    ) -> Result<Self, String> {
        let SourceOptions {
            constants,
            cfgs,
            expansions,
            module_lints,
        } = options;
        let closed = if !census.external_imports.is_empty()
            || !census.documentation.is_empty()
            || expansions && !census.expansions.is_empty()
        {
            Err("Module expansion records require independent expanded closure".into())
        } else {
            census::verify_with_constants(&census, constants)
        }
        .and_then(|modules| build_index(&modules, &census, None).map(|index| (modules, index)));
        let (modules, index, copy_boundary) = match closed {
            Ok((modules, index)) => (modules, index, false),
            Err(error) => {
                let Some(cfgs) = cfgs else { return Err(error) };
                let modules = if module_lints {
                    census::verify_with_module_lints(&census, cfgs, expansions)?
                } else if !census.external_imports.is_empty() {
                    census::verify_imported(&census, cfgs, expansions)?
                } else if !census.documentation.is_empty() {
                    census::verify_documented(&census, cfgs, expansions)?
                } else if expansions {
                    census::verify_expanded(&census, cfgs)?
                } else {
                    census::verify_selected(&census, cfgs)?
                };
                let index = build_index(&modules, &census, Some(cfgs))?;
                (modules, index, true)
            }
        };
        let sources = source_ids(&census)?;

        let functions = functions::Index::new(&modules, &census)?;
        Ok(Self {
            census,
            sources,
            modules,
            functions,
            index,
            copy_boundary,
            cleanup_sources: Default::default(),
        })
    }

    pub(super) fn checked_cleanup_source(
        &self,
        cfgs: &[String],
        target: &Namespace,
        profile: &'static str,
    ) -> bool {
        self.cleanup_sources
            .borrow()
            .contains(&CleanupSource::new(cfgs, target, profile))
    }

    pub(super) fn cache_cleanup_source(
        &self,
        cfgs: &[String],
        target: &Namespace,
        profile: &'static str,
    ) {
        let mut cache = self.cleanup_sources.borrow_mut();
        if cache.len() < 256 {
            cache.insert(CleanupSource::new(cfgs, target, profile));
        }
    }

    pub(super) fn function_candidates(
        &self,
        module: usize,
        span: ByteSpan,
    ) -> impl Iterator<Item = &syn::ItemFn> {
        self.functions.candidates(module, span).map(move |index| {
            let syn::Item::Fn(function) = &self.modules[module].items[index] else {
                unreachable!("Function index retains only original function items")
            };
            function
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

    /// The lexical index and its checked source snapshots are immutable together.
    /// Memoize actual bytes, never the caller's nominal digest claim.
    pub(super) fn declaration_identity<'a>(
        &self,
        declaration: &'a Declaration,
    ) -> Result<&'a str, String> {
        if let Some(identity) = declaration.identity.get() {
            return Ok(identity);
        }
        let source = self.source(&declaration.path)?;
        let bytes = source.range(declaration.span)?;
        Ok(declaration
            .identity
            .get_or_init(|| concrete_types::digest(bytes.as_bytes())))
    }

    pub(crate) fn check_context(&self, facts: &HirFnFacts) -> Result<(), String> {
        let context: UnitContext = evidence(facts, CONTEXT_KEY)?;
        if context != self.census.context {
            return Err("Nominal census belongs to another selected unit".into());
        }
        Ok(())
    }

    pub(crate) fn check_input_owner(&self, facts: &HirFnFacts) -> Result<(), String> {
        self.check_context(facts)?;
        let path = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Missing source-input function owner")?;
        if self.source(Path::new(path))?.range(facts.fn_span)? != &facts.source.bytes[..] {
            return Err("Source inputs disagree with the exact captured function range".into());
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

fn source_ids(census: &census::Census) -> Result<BTreeMap<PathBuf, SourceId>, String> {
    census
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            u32::try_from(index)
                .map(|index| (file.path.clone(), SourceId(index)))
                .map_err(|_| "Oversized source table".to_owned())
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn census() -> Arc<census::Census> {
        let bytes = "\u{feff}// π\r\npub struct Data { pub value: i32 }\r\n";
        Arc::new(census::Census {
            expansions: Vec::new(),
            documentation: Vec::new(),
            external_imports: Vec::new(),
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
    fn declaration_identity_is_memoized_only_for_its_immutable_index() {
        let original = census();
        let environment = Environment::new(Arc::clone(&original)).unwrap();
        let declaration = environment.index.declarations.values().next().unwrap();
        assert!(declaration.identity.get().is_none());
        let identity = environment.declaration_identity(declaration).unwrap();
        assert_eq!(
            identity,
            concrete_types::digest(b"pub struct Data { pub value: i32 }")
        );
        assert!(std::ptr::eq(
            identity,
            environment.declaration_identity(declaration).unwrap()
        ));

        let mut changed = (*original).clone();
        changed.files[0].bytes = changed.files[0].bytes.replace("i32", "u32").into();
        changed.files[0].digest = changed.files[0].bytes.identity().to_owned();
        let other = Environment::new(Arc::new(changed)).unwrap();
        let declaration = other.index.declarations.values().next().unwrap();
        assert!(declaration.identity.get().is_none());
        assert_ne!(identity, other.declaration_identity(declaration).unwrap());
        assert_eq!(
            identity,
            environment
                .declaration_identity(environment.index.declarations.values().next().unwrap())
                .unwrap()
        );
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
            builtin_copy: None,
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
    #[test]
    fn cleanup_cache_does_not_cross_source_target_or_cfg() {
        let first = Environment::new(census()).unwrap();
        let second = Environment::new(census()).unwrap();
        let target = vec!["sample".into(), "Data".into()];
        let cfgs = vec!["feature=one".into()];
        assert!(!first.checked_cleanup_source(&cfgs, &target, "hir-data-v3"));
        first.cache_cleanup_source(&cfgs, &target, "hir-data-v3");
        assert!(first.checked_cleanup_source(&cfgs, &target, "hir-data-v3"));
        assert!(!first.checked_cleanup_source(&[], &target, "hir-data-v3"));
        assert!(!first.checked_cleanup_source(
            &cfgs,
            &vec!["sample".into(), "Other".into()],
            "hir-data-v3"
        ));
        assert!(!first.checked_cleanup_source(&cfgs, &target, "hir-data-v2"));
        crate::verification_policy::generated(|| {
            assert!(!first.checked_cleanup_source(&cfgs, &target, "hir-data-v3"));
            first.cache_cleanup_source(&cfgs, &target, "hir-data-v3");
            assert!(first.checked_cleanup_source(&cfgs, &target, "hir-data-v3"));
        });
        assert!(!second.checked_cleanup_source(&cfgs, &target, "hir-data-v3"));
    }
}
